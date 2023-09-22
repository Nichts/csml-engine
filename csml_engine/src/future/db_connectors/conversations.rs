use uuid::Uuid;
#[cfg(feature = "postgresql-async")]
use crate::future::db_connectors::{is_postgresql, postgresql_connector};

use csml_interpreter::data::csml_logs::{csml_logger, CsmlLog, LogLvl};

use crate::error_messages::ERROR_DB_SETUP;
use crate::future::db_connectors::{state, utils::*};
use crate::models::DbConversation;
use crate::{AsyncConversationInfo, AsyncDatabase, Client, EngineError};

pub async fn create_conversation(
    flow_id: &str,
    step_id: &str,
    client: &Client,
    ttl: Option<chrono::Duration>,
    db: &mut AsyncDatabase<'_>,
) -> Result<String, EngineError> {
    csml_logger(
        CsmlLog::new(
            None,
            None,
            None,
            format!(
                "db call create conversation flow_id: {}, step_id:{}",
                flow_id, step_id
            ),
        ),
        LogLvl::Info,
    );
    csml_logger(
        CsmlLog::new(
            Some(client),
            None,
            None,
            format!(
                "db call create conversation flow_id: {}, step_id:{}",
                flow_id, step_id
            ),
        ),
        LogLvl::Debug,
    );

    #[cfg(feature = "postgresql-async")]
    if is_postgresql() {
        let db = postgresql_connector::get_db(db)?;
        let expires_at = get_expires_at_for_postgresql(ttl);
        return postgresql_connector::conversations::create_conversation(
            flow_id, step_id, client, expires_at, db,
        )
        .await;
    }

    Err(EngineError::Manager(ERROR_DB_SETUP.to_owned()))
}

pub async fn close_conversation(
    id: &str,
    client: &Client,
    db: &mut AsyncDatabase<'_>,
) -> Result<(), EngineError> {
    csml_logger(
        CsmlLog::new(
            None,
            None,
            None,
            format!("db call close conversation conversation_id: {}", id),
        ),
        LogLvl::Info,
    );
    csml_logger(
        CsmlLog::new(
            Some(client),
            None,
            None,
            format!("db call close conversation conversation_id: {}", id),
        ),
        LogLvl::Debug,
    );

    // delete previous bot info at the end of the conversation
    state::delete_state_key(client, "bot", "previous", db).await?;

    #[cfg(feature = "postgresql-async")]
    if is_postgresql() {
        let db = postgresql_connector::get_db(db)?;
        return postgresql_connector::conversations::close_conversation(id, client, "CLOSED", db)
            .await;
    }

    Err(EngineError::Manager(ERROR_DB_SETUP.to_owned()))
}

pub async fn close_all_conversations(
    client: &Client,
    db: &mut AsyncDatabase<'_>,
) -> Result<(), EngineError> {
    csml_logger(
        CsmlLog::new(
            None,
            None,
            None,
            "db call close all conversations".to_string(),
        ),
        LogLvl::Info,
    );
    csml_logger(
        CsmlLog::new(
            Some(client),
            None,
            None,
            format!("db call close all conversations, client: {:?}", client),
        ),
        LogLvl::Debug,
    );

    #[cfg(feature = "postgresql-async")]
    if is_postgresql() {
        let db = postgresql_connector::get_db(db)?;
        return postgresql_connector::conversations::close_all_conversations(client, db).await;
    }

    Err(EngineError::Manager(ERROR_DB_SETUP.to_owned()))
}

pub async fn get_latest_open(
    client: &Client,
    db: &mut AsyncDatabase<'_>,
) -> Result<Option<DbConversation>, EngineError> {
    csml_logger(
        CsmlLog::new(
            None,
            None,
            None,
            "db call get latest open conversations".to_string(),
        ),
        LogLvl::Info,
    );
    csml_logger(
        CsmlLog::new(
            Some(client),
            None,
            None,
            "db call get latest open conversations".to_string(),
        ),
        LogLvl::Debug,
    );

    #[cfg(feature = "postgresql-async")]
    if is_postgresql() {
        let db = postgresql_connector::get_db(db)?;
        return postgresql_connector::conversations::get_latest_open(client, db).await;
    }

    Err(EngineError::Manager(ERROR_DB_SETUP.to_owned()))
}

pub async fn update_conversation(
    data: &mut AsyncConversationInfo<'_>,
    flow_id: Option<String>,
    step_id: Option<String>,
) -> Result<(), EngineError> {
    csml_logger(
        CsmlLog::new(
            None,
            None,
            None,
            format!(
                "db call update conversations flow_id {:?}, step_id {:?}",
                flow_id, step_id
            ),
        ),
        LogLvl::Info,
    );
    csml_logger(
        CsmlLog::new(
            Some(&data.client),
            None,
            None,
            format!(
                "db call update conversations flow_id {:?}, step_id {:?}",
                flow_id, step_id
            ),
        ),
        LogLvl::Debug,
    );

    #[cfg(feature = "postgresql-async")]
    if is_postgresql() {
        let db = postgresql_connector::get_db(&mut data.db)?;
        return postgresql_connector::conversations::update_conversation(
            &data.conversation_id,
            flow_id,
            step_id,
            db,
        )
        .await;
    }

    Err(EngineError::Manager(ERROR_DB_SETUP.to_owned()))
}

pub async fn get_conversation(
    db: &mut AsyncDatabase<'_>,
    id: Uuid,
) -> Result<serde_json::Value, EngineError> {
    csml_logger(
        CsmlLog::new(
            None,
            None,
            None,
            format!("db call get client conversation"),
        ),
        LogLvl::Info,
    );

    #[cfg(feature = "postgresql-async")]
    if is_postgresql() {
        let db = postgresql_connector::get_db(db)?;
        return postgresql_connector::conversations::get_conversation(
            db,
            id,
        ).await;
    }

    Err(EngineError::Manager(ERROR_DB_SETUP.to_owned()))
}

pub async fn get_client_conversations(
    client: &Client,
    db: &mut AsyncDatabase<'_>,
    limit: Option<i64>,
    pagination_key: Option<String>,
) -> Result<serde_json::Value, EngineError> {
    csml_logger(
        CsmlLog::new(
            None,
            None,
            None,
            format!("db call get client conversations, limit: {:?}", limit),
        ),
        LogLvl::Info,
    );
    csml_logger(
        CsmlLog::new(
            Some(client),
            None,
            None,
            format!(
                "db call get client conversations limit: {:?}, pagination_key: {:?}",
                limit, pagination_key
            ),
        ),
        LogLvl::Info,
    );

    #[cfg(feature = "postgresql-async")]
    if is_postgresql() {
        let db = postgresql_connector::get_db(db)?;
        return postgresql_connector::conversations::get_client_conversations(
            client,
            db,
            limit,
            pagination_key,
        )
        .await;
    }

    Err(EngineError::Manager(ERROR_DB_SETUP.to_owned()))
}
