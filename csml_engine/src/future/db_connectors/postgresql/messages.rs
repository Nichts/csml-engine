use diesel::{ExpressionMethods, QueryDsl};
use diesel_async::RunQueryDsl;
use std::convert::TryInto;

use crate::{
    data,
    encrypt::{decrypt_data, encrypt_data},
    future::db_connectors::postgresql::get_db,
    AsyncConversationInfo, AsyncPostgresqlClient, Client, EngineError,
};

use super::pagination::*;
use crate::data::filter::ClientMessageFilter;
use crate::data::models::PaginationData;
use crate::db_connectors::diesel::Direction;
use crate::db_connectors::postgresql::{
    models,
    schema::{csml_conversations, csml_messages},
};
use chrono::NaiveDateTime;
use uuid::Uuid;

pub async fn add_messages_bulk(
    data: &mut AsyncConversationInfo<'_>,
    msgs: &[serde_json::Value],
    interaction_order: i32,
    direction: Direction,
    expires_at: Option<NaiveDateTime>,
) -> Result<(), EngineError> {
    if msgs.is_empty() {
        return Ok(());
    }

    let db = get_db(&mut data.db)?;

    let mut new_messages = vec![];
    for (message_order, message) in msgs.iter().enumerate() {
        let conversation_id = uuid::Uuid::parse_str(&data.conversation_id).unwrap();

        let msg = models::NewMessages {
            id: uuid::Uuid::new_v4(),
            conversation_id,

            flow_id: &data.context.flow,
            step_id: data.context.step.get_step_ref(),
            direction,
            payload: encrypt_data(message)?,
            content_type: message["content_type"].as_str().unwrap_or("text"),

            message_order: message_order as i32,
            interaction_order,
            expires_at,
        };

        new_messages.push(msg);
    }

    diesel::insert_into(csml_messages::table)
        .values(&new_messages)
        .get_result::<models::Message>(db.client.as_mut())
        .await?;

    Ok(())
}

pub async fn delete_user_messages(
    client: &Client,
    db: &mut AsyncPostgresqlClient<'_>,
) -> Result<(), EngineError> {
    let conversations: Vec<models::Conversation> = csml_conversations::table
        .filter(csml_conversations::bot_id.eq(&client.bot_id))
        .filter(csml_conversations::channel_id.eq(&client.channel_id))
        .filter(csml_conversations::user_id.eq(&client.user_id))
        .load(db.client.as_mut())
        .await?;

    for conversation in conversations {
        diesel::delete(
            csml_messages::table.filter(csml_messages::conversation_id.eq(&conversation.id)),
        )
        .execute(db.client.as_mut())
        .await
        .ok();
    }

    Ok(())
}

pub async fn get_client_messages<'conn, 'a: 'conn>(
    db: &'a mut AsyncPostgresqlClient<'conn>,
    filter: ClientMessageFilter<'a>,
) -> Result<data::models::Paginated<data::models::Message>, EngineError> {
    let ClientMessageFilter {
        client,
        limit,
        pagination_key,
        from_date,
        to_date,
        conversation_id,
    } = filter;

    let pagination_key = pagination_key.unwrap_or(1);

    let (conversation_with_messages, total_pages) = match conversation_id {
        None => {
            get_messages_without_conversation_filter(
                &client,
                db,
                limit,
                from_date,
                to_date,
                pagination_key,
            )
            .await?
        }
        Some(conv_id) => {
            get_messages_with_conversation_filter(
                &client,
                db,
                limit,
                from_date,
                to_date,
                pagination_key,
                conv_id,
            )
            .await?
        }
    };

    let (_, messages): (Vec<_>, Vec<_>) = conversation_with_messages.into_iter().unzip();

    let mut msgs = vec![];
    for message in messages {
        let msg: data::models::Message = message.try_into()?;

        msgs.push(msg);
    }

    let pagination = (pagination_key < total_pages).then(|| PaginationData {
        page: pagination_key,
        total_pages,
        per_page: limit,
    });
    Ok(data::models::Paginated {
        data: msgs,
        pagination,
    })
}

pub(crate) async fn get_messages_without_conversation_filter(
    client: &Client,
    db: &mut AsyncPostgresqlClient<'_>,
    limit_per_page: u32,
    from_date: Option<i64>,
    to_date: Option<i64>,
    pagination_key: u32,
) -> Result<(Vec<(models::Conversation, models::Message)>, u32), EngineError> {
    let client = client.to_owned();
    let res = match from_date {
        Some(from_date) => {
            let from_date = NaiveDateTime::from_timestamp_opt(from_date, 0).ok_or(
                EngineError::DateTimeError("Date time is out of range".to_owned()),
            )?;
            let to_date = match to_date {
                Some(to_date) => NaiveDateTime::from_timestamp_opt(to_date, 0).ok_or(
                    EngineError::DateTimeError("Date time is out of range".to_owned()),
                )?,
                None => chrono::Utc::now().naive_utc(),
            };

            let mut query = csml_conversations::table
                .filter(csml_conversations::bot_id.eq(client.bot_id))
                .filter(csml_conversations::channel_id.eq(client.channel_id))
                .filter(csml_conversations::user_id.eq(client.user_id))
                .inner_join(csml_messages::table)
                .filter(csml_messages::created_at.ge(from_date))
                .filter(csml_messages::created_at.le(to_date))
                .select((csml_conversations::all_columns, csml_messages::all_columns))
                .order_by(csml_messages::created_at.desc())
                .then_order_by(csml_messages::message_order.desc())
                .paginate(pagination_key);

            query = query.per_page(limit_per_page);

            query.load_and_count_pages(db.client.as_mut()).await?
        }
        None => {
            let mut query = csml_conversations::table
                .filter(csml_conversations::bot_id.eq(client.bot_id))
                .filter(csml_conversations::channel_id.eq(client.channel_id))
                .filter(csml_conversations::user_id.eq(client.user_id))
                .inner_join(csml_messages::table)
                .select((csml_conversations::all_columns, csml_messages::all_columns))
                .order_by(csml_messages::created_at.desc())
                .then_order_by(csml_messages::message_order.desc())
                .paginate(pagination_key);

            query = query.per_page(limit_per_page);

            query.load_and_count_pages(db.client.as_mut()).await?
        }
    };
    Ok(res)
}

async fn get_messages_with_conversation_filter(
    client: &Client,
    db: &mut AsyncPostgresqlClient<'_>,
    limit_per_page: u32,
    from_date: Option<i64>,
    to_date: Option<i64>,
    pagination_key: u32,
    conversation_id: Uuid,
) -> Result<(Vec<(models::Conversation, models::Message)>, u32), EngineError> {
    let client = client.to_owned();
    let res = match from_date {
        Some(from_date) => {
            let from_date = NaiveDateTime::from_timestamp_opt(from_date, 0).ok_or(
                EngineError::DateTimeError("Date time is out of range".to_owned()),
            )?;
            let to_date = match to_date {
                Some(to_date) => NaiveDateTime::from_timestamp_opt(to_date, 0).ok_or(
                    EngineError::DateTimeError("Date time is out of range".to_owned()),
                )?,
                None => chrono::Utc::now().naive_utc(),
            };

            let mut query = csml_conversations::table
                .filter(csml_conversations::bot_id.eq(client.bot_id))
                .filter(csml_conversations::channel_id.eq(client.channel_id))
                .filter(csml_conversations::user_id.eq(client.user_id))
                .filter(csml_conversations::id.eq(conversation_id))
                .inner_join(csml_messages::table)
                .filter(csml_messages::created_at.ge(from_date))
                .filter(csml_messages::created_at.le(to_date))
                .select((csml_conversations::all_columns, csml_messages::all_columns))
                .order_by(csml_messages::created_at.desc())
                .then_order_by(csml_messages::message_order.desc())
                .paginate(pagination_key);

            query = query.per_page(limit_per_page);

            query.load_and_count_pages(db.client.as_mut()).await?
        }
        None => {
            let mut query = csml_conversations::table
                .filter(csml_conversations::bot_id.eq(client.bot_id))
                .filter(csml_conversations::channel_id.eq(client.channel_id))
                .filter(csml_conversations::user_id.eq(client.user_id))
                .filter(csml_conversations::id.eq(conversation_id))
                .inner_join(csml_messages::table)
                .select((csml_conversations::all_columns, csml_messages::all_columns))
                .order_by(csml_messages::created_at.desc())
                .then_order_by(csml_messages::message_order.desc())
                .paginate(pagination_key);

            query = query.per_page(limit_per_page);

            query.load_and_count_pages(db.client.as_mut()).await?
        }
    };
    Ok(res)
}
