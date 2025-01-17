use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl};

use crate::data::models::{Conversation, PaginationData};
use crate::{data, Client, EngineError, PostgresqlClient};
use chrono::NaiveDateTime;
use uuid::Uuid;

use super::{models, pagination::*, schema::csml_conversations};

pub fn create_conversation(
    flow_id: &str,
    step_id: &str,
    client: &Client,
    expires_at: Option<NaiveDateTime>,
    db: &mut PostgresqlClient,
) -> Result<Uuid, EngineError> {
    let new_conversation = models::NewConversation {
        id: uuid::Uuid::new_v4(),
        bot_id: &client.bot_id,
        channel_id: &client.channel_id,
        user_id: &client.user_id,
        flow_id,
        step_id,
        status: "OPEN",
        expires_at,
    };

    let conversation: models::Conversation = diesel::insert_into(csml_conversations::table)
        .values(&new_conversation)
        .get_result(db.client.as_mut())?;

    Ok(conversation.id)
}

pub fn close_conversation(
    id: Uuid,
    _client: &Client,
    status: &str,
    db: &mut PostgresqlClient,
) -> Result<(), EngineError> {

    diesel::update(csml_conversations::table.filter(csml_conversations::id.eq(id)))
        .set(csml_conversations::status.eq(status))
        .execute(db.client.as_mut())?;

    Ok(())
}

pub fn close_all_conversations(
    client: &Client,
    db: &mut PostgresqlClient,
) -> Result<(), EngineError> {
    diesel::update(
        csml_conversations::table
            .filter(csml_conversations::bot_id.eq(&client.bot_id))
            .filter(csml_conversations::channel_id.eq(&client.channel_id))
            .filter(csml_conversations::user_id.eq(&client.user_id)),
    )
    .set(csml_conversations::status.eq("CLOSED"))
    .execute(db.client.as_mut())?;

    Ok(())
}

pub fn get_latest_open(
    client: &Client,
    db: &mut PostgresqlClient,
) -> Result<Option<Conversation>, EngineError> {
    let result: Result<models::Conversation, diesel::result::Error> = csml_conversations::table
        .filter(csml_conversations::bot_id.eq(&client.bot_id))
        .filter(csml_conversations::channel_id.eq(&client.channel_id))
        .filter(csml_conversations::user_id.eq(&client.user_id))
        .filter(csml_conversations::status.eq("OPEN"))
        .order_by(csml_conversations::updated_at.desc())
        .limit(1)
        .get_result(db.client.as_mut());

    match result {
        Ok(conv) => {
            let conversation = conv.into();

            Ok(Some(conversation))
        }
        Err(..) => Ok(None),
    }
}

pub fn update_conversation(
    conversation_id: Uuid,
    flow_id: Option<String>,
    step_id: Option<String>,
    db: &mut PostgresqlClient,
) -> Result<(), EngineError> {

    match (flow_id, step_id) {
        (Some(flow_id), Some(step_id)) => {
            diesel::update(csml_conversations::table.filter(csml_conversations::id.eq(&conversation_id)))
                .set((
                    csml_conversations::flow_id.eq(flow_id.as_str()),
                    csml_conversations::step_id.eq(step_id.as_str()),
                ))
                .execute(db.client.as_mut())?;
        }
        (Some(flow_id), _) => {
            diesel::update(csml_conversations::table.filter(csml_conversations::id.eq(&conversation_id)))
                .set(csml_conversations::flow_id.eq(flow_id.as_str()))
                .get_result::<models::Conversation>(db.client.as_mut())?;
        }
        (_, Some(step_id)) => {
            diesel::update(csml_conversations::table.filter(csml_conversations::id.eq(&conversation_id)))
                .set(csml_conversations::step_id.eq(step_id.as_str()))
                .get_result::<models::Conversation>(db.client.as_mut())?;
        }
        _ => return Ok(()),
    };

    Ok(())
}

pub fn delete_user_conversations(
    client: &Client,
    db: &mut PostgresqlClient,
) -> Result<(), EngineError> {
    diesel::delete(
        csml_conversations::table
            .filter(csml_conversations::bot_id.eq(&client.bot_id))
            .filter(csml_conversations::channel_id.eq(&client.channel_id))
            .filter(csml_conversations::user_id.eq(&client.user_id)),
    )
    .execute(db.client.as_mut())
    .ok();

    Ok(())
}

pub fn get_client_conversations(
    client: &Client,
    db: &mut PostgresqlClient,
    limit: Option<u32>,
    pagination_key: Option<u32>,
) -> Result<data::models::Paginated<Conversation>, EngineError> {
    let pagination_key = pagination_key.unwrap_or(1);

    let mut query = csml_conversations::table
        .order_by(csml_conversations::updated_at.desc())
        .filter(csml_conversations::bot_id.eq(&client.bot_id))
        .filter(csml_conversations::channel_id.eq(&client.channel_id))
        .filter(csml_conversations::user_id.eq(&client.user_id))
        .paginate(pagination_key);

    let limit_per_page = limit.unwrap_or(25).min(25);
    query = query.per_page(limit_per_page);

    let (conversations, total_pages): (Vec<models::Conversation>, _) =
        query.load_and_count_pages::<models::Conversation>(db.client.as_mut())?;

    let convs: Vec<_> = conversations.into_iter().map(Into::into).collect();

    let pagination = (pagination_key < total_pages).then_some(PaginationData {
        page: pagination_key,
        total_pages,
        per_page: limit_per_page,
    });
    Ok(data::models::Paginated {
        data: convs,
        pagination,
    })
}

pub fn delete_all_bot_data(bot_id: &str, db: &mut PostgresqlClient) -> Result<(), EngineError> {
    diesel::delete(csml_conversations::table.filter(csml_conversations::bot_id.eq(bot_id)))
        .execute(db.client.as_mut())
        .ok();

    Ok(())
}
