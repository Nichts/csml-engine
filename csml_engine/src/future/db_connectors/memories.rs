#[cfg(feature = "postgresql-async")]
use crate::future::db_connectors::{is_postgresql, postgresql_connector};

use csml_interpreter::data::csml_logs::{csml_logger, CsmlLog, LogLvl};

use crate::error_messages::ERROR_DB_SETUP;
use crate::future::db_connectors::utils::*;
use crate::{AsyncConversationInfo, AsyncDatabase, Client, EngineError, Memory};
use std::collections::HashMap;

pub async fn add_memories(
    data: &mut AsyncConversationInfo<'_>,
    memories: &HashMap<String, Memory>,
) -> Result<(), EngineError> {
    csml_logger(
        CsmlLog::new(
            None,
            None,
            None,
            format!("db call save memories {:?}", memories.keys()),
        ),
        LogLvl::Info,
    );
    csml_logger(
        CsmlLog::new(
            None,
            None,
            None,
            format!("db call save memories {:?}", memories.keys()),
        ),
        LogLvl::Debug,
    );

    #[cfg(feature = "postgresql-async")]
    if is_postgresql() {
        let expires_at = get_expires_at_for_postgresql(data.ttl);
        return postgresql_connector::memories::add_memories(data, memories, expires_at).await;
    }

    Err(EngineError::Manager(ERROR_DB_SETUP.to_owned()))
}

pub async fn create_client_memory(
    client: &Client,
    key: String,
    value: serde_json::Value,
    ttl: Option<chrono::Duration>,
    db: &mut AsyncDatabase<'_>,
) -> Result<(), EngineError> {
    csml_logger(
        CsmlLog::new(None, None, None, format!("db call save memory {:?}", key)),
        LogLvl::Info,
    );
    csml_logger(
        CsmlLog::new(
            None,
            None,
            None,
            format!("db call save memory {:?} with value {:?}", key, value),
        ),
        LogLvl::Debug,
    );

    #[cfg(feature = "postgresql-async")]
    if is_postgresql() {
        let db = postgresql_connector::get_db(db)?;
        let expires_at = get_expires_at_for_postgresql(ttl);
        return postgresql_connector::memories::create_client_memory(
            client, &key, &value, expires_at, db,
        )
        .await;
    }

    Err(EngineError::Manager(ERROR_DB_SETUP.to_owned()))
}

pub async fn internal_use_get_memories(
    client: &Client,
    db: &mut AsyncDatabase<'_>,
) -> Result<serde_json::Value, EngineError> {
    csml_logger(
        CsmlLog::new(None, None, None, "db call get memories".to_string()),
        LogLvl::Info,
    );
    csml_logger(
        CsmlLog::new(Some(client), None, None, "db call get memories".to_string()),
        LogLvl::Debug,
    );

    #[cfg(feature = "postgresql-async")]
    if is_postgresql() {
        let db = postgresql_connector::get_db(db)?;
        return postgresql_connector::memories::internal_use_get_memories(client, db).await;
    }

    Err(EngineError::Manager(ERROR_DB_SETUP.to_owned()))
}

/**
 * Get client Memories
 */
pub async fn get_memories(
    client: &Client,
    db: &mut AsyncDatabase<'_>,
) -> Result<serde_json::Value, EngineError> {
    csml_logger(
        CsmlLog::new(None, None, None, "db call get memories client".to_string()),
        LogLvl::Info,
    );
    csml_logger(
        CsmlLog::new(
            Some(client),
            None,
            None,
            "db call get memories client".to_string(),
        ),
        LogLvl::Debug,
    );

    #[cfg(feature = "postgresql-async")]
    if is_postgresql() {
        let db = postgresql_connector::get_db(db)?;
        return postgresql_connector::memories::get_memories(client, db).await;
    }

    Err(EngineError::Manager(ERROR_DB_SETUP.to_owned()))
}

/**
 * Get client Memory
 */
pub async fn get_memory(
    client: &Client,
    key: &str,
    db: &mut AsyncDatabase<'_>,
) -> Result<serde_json::Value, EngineError> {
    csml_logger(
        CsmlLog::new(None, None, None, format!("db call get memory {:?}", key)),
        LogLvl::Info,
    );
    csml_logger(
        CsmlLog::new(
            Some(client),
            None,
            None,
            format!("db call get memory {:?}", key),
        ),
        LogLvl::Debug,
    );

    #[cfg(feature = "postgresql-async")]
    if is_postgresql() {
        let db = postgresql_connector::get_db(db)?;
        return postgresql_connector::memories::get_memory(client, key, db).await;
    }

    Err(EngineError::Manager(ERROR_DB_SETUP.to_owned()))
}

pub async fn delete_client_memory(
    client: &Client,
    key: &str,
    db: &mut AsyncDatabase<'_>,
) -> Result<(), EngineError> {
    csml_logger(
        CsmlLog::new(None, None, None, format!("db call delete memory {:?}", key)),
        LogLvl::Info,
    );
    csml_logger(
        CsmlLog::new(
            Some(client),
            None,
            None,
            format!("db call delete memory {:?}", key),
        ),
        LogLvl::Debug,
    );

    #[cfg(feature = "postgresql-async")]
    if is_postgresql() {
        let db = postgresql_connector::get_db(db)?;
        return postgresql_connector::memories::delete_client_memory(client, key, db).await;
    }

    Err(EngineError::Manager(ERROR_DB_SETUP.to_owned()))
}

pub async fn delete_client_memories(
    client: &Client,
    db: &mut AsyncDatabase<'_>,
) -> Result<(), EngineError> {
    csml_logger(
        CsmlLog::new(None, None, None, "db call delete memories".to_string()),
        LogLvl::Info,
    );
    csml_logger(
        CsmlLog::new(
            Some(client),
            None,
            None,
            "db call delete memories".to_string(),
        ),
        LogLvl::Debug,
    );

    #[cfg(feature = "postgresql-async")]
    if is_postgresql() {
        let db = postgresql_connector::get_db(db)?;
        return postgresql_connector::memories::delete_client_memories(client, db).await;
    }

    Err(EngineError::Manager(ERROR_DB_SETUP.to_owned()))
}
