use diesel::{ExpressionMethods, QueryDsl};
use diesel_async::{RunQueryDsl};

use crate::{
    future::db_connectors::postgresql::get_db,
    encrypt::{decrypt_data, encrypt_data},
    Client, AsyncConversationInfo, EngineError, AsyncPostgresqlClient,
};

use super::{
    models,
    pagination::*,
    schema::{csml_conversations, csml_messages},
};
use chrono::NaiveDateTime;
use uuid::Uuid;

pub async fn add_messages_bulk(
    data: &mut AsyncConversationInfo<'_>,
    msgs: &[serde_json::Value],
    interaction_order: i32,
    direction: &str,
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
        .get_result::<models::Message>(db.client.as_mut()).await?;

    Ok(())
}

pub async fn delete_user_messages(client: &Client, db: &mut AsyncPostgresqlClient<'_>) -> Result<(), EngineError> {
    let conversations: Vec<models::Conversation> = csml_conversations::table
        .filter(csml_conversations::bot_id.eq(&client.bot_id))
        .filter(csml_conversations::channel_id.eq(&client.channel_id))
        .filter(csml_conversations::user_id.eq(&client.user_id))
        .load(db.client.as_mut()).await?;

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
    client: &'a Client,
    db: &'a mut AsyncPostgresqlClient<'conn>,
    limit: Option<i64>,
    pagination_key: Option<String>,
    from_date: Option<i64>,
    to_date: Option<i64>,
    conversation_id: Option<String>,
) -> Result<serde_json::Value, EngineError> {
    let pagination_key = match pagination_key {
        Some(paginate) => paginate.parse::<i64>().unwrap_or(1),
        None => 1,
    };

    let (conversation_with_messages, total_pages) = match conversation_id {
        None => get_messages_without_conversation_filter(&client, db, limit, from_date, to_date, pagination_key).await?,
        Some(conv_id) => get_messages_with_conversation_filter(&client, db, limit, from_date, to_date, pagination_key, conv_id).await?
    };

    let (_, messages): (Vec<_>, Vec<_>) = conversation_with_messages.into_iter().unzip();

    let mut msgs = vec![];
    for message in messages {
        let json = serde_json::json!({
            "client": {
                "bot_id": &client.bot_id,
                "channel_id": &client.channel_id,
                "user_id": &client.user_id
            },
            "conversation_id": message.conversation_id,
            "flow_id": message.flow_id,
            "step_id": message.step_id,
            "direction": message.direction,
            "payload": decrypt_data(message.payload)?,

            "updated_at": message.updated_at.format("%Y-%m-%dT%H:%M:%S%.fZ").to_string(),
            "created_at": message.created_at.format("%Y-%m-%dT%H:%M:%S%.fZ").to_string()
        });

        msgs.push(json);
    }

    match pagination_key < total_pages {
        true => {
            let pagination_key = (pagination_key + 1).to_string();
            Ok(serde_json::json!({"messages": msgs, "pagination_key": pagination_key}))
        }
        false => Ok(serde_json::json!({ "messages": msgs })),
    }
}

pub(crate) async fn get_messages_without_conversation_filter(
    client: &Client,
    db: &mut AsyncPostgresqlClient<'_>,
    limit: Option<i64>,
    from_date: Option<i64>,
    to_date: Option<i64>, pagination_key: i64
) -> Result<(Vec<(models::Conversation, models::Message)>, i64), EngineError> {
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

            let limit_per_page = limit.unwrap_or(25);
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

            let limit_per_page = limit.unwrap_or(25);
            query = query.per_page(limit_per_page);

            query.load_and_count_pages(db.client.as_mut()).await?
        }
    };
    Ok(res)
}

async fn get_messages_with_conversation_filter(
    client: &Client,
    db: &mut AsyncPostgresqlClient<'_>,
    limit: Option<i64>,
    from_date: Option<i64>,
    to_date: Option<i64>, pagination_key: i64,
    conversation_id: String,
) -> Result<(Vec<(models::Conversation, models::Message)>, i64), EngineError> {
    let id = Uuid::parse_str(&conversation_id)?;
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
                .filter(csml_conversations::id.eq(id))
                .inner_join(csml_messages::table)
                .filter(csml_messages::created_at.ge(from_date))
                .filter(csml_messages::created_at.le(to_date))
                .select((csml_conversations::all_columns, csml_messages::all_columns))
                .order_by(csml_messages::created_at.desc())
                .then_order_by(csml_messages::message_order.desc())
                .paginate(pagination_key);

            let limit_per_page = limit.unwrap_or(25);
            query = query.per_page(limit_per_page);

            query.load_and_count_pages(db.client.as_mut()).await?
        }
        None => {
            let mut query = csml_conversations::table
                .filter(csml_conversations::bot_id.eq(client.bot_id))
                .filter(csml_conversations::channel_id.eq(client.channel_id))
                .filter(csml_conversations::user_id.eq(client.user_id))
                .filter(csml_conversations::id.eq(id))
                .inner_join(csml_messages::table)
                .select((csml_conversations::all_columns, csml_messages::all_columns))
                .order_by(csml_messages::created_at.desc())
                .then_order_by(csml_messages::message_order.desc())
                .paginate(pagination_key);

            let limit_per_page = limit.unwrap_or(25);
            query = query.per_page(limit_per_page);

            query.load_and_count_pages(db.client.as_mut()).await?
        }
    };
    Ok(res)
}