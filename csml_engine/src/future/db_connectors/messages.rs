#[cfg(feature = "postgresql-async")]
use crate::future::db_connectors::{is_postgresql, postgresql_connector};

use crate::data::filter::ClientMessageFilter;
use crate::data::models::{Direction, Message, Paginated};
use crate::error_messages::ERROR_DB_SETUP;
use crate::future::db_connectors::utils::*;
use crate::{AsyncConversationInfo, AsyncDatabase, EngineError};
use csml_interpreter::data::csml_logs::{csml_logger, CsmlLog, LogLvl};

pub async fn add_messages_bulk(
    data: &mut AsyncConversationInfo<'_>,
    msgs: Vec<serde_json::Value>,
    interaction_order: i32,
    direction: Direction,
) -> Result<(), EngineError> {
    csml_logger(
        CsmlLog::new(
            None,
            None,
            None,
            format!("db call save messages {:?}", msgs),
        ),
        LogLvl::Info,
    );
    csml_logger(
        CsmlLog::new(
            Some(&data.client),
            None,
            None,
            format!("db call save messages {:?}", msgs),
        ),
        LogLvl::Debug,
    );

    #[cfg(feature = "postgresql-async")]
    if is_postgresql() {
        let expires_at = get_expires_at_for_postgresql(data.ttl);

        return postgresql_connector::messages::add_messages_bulk(
            data,
            &msgs,
            interaction_order,
            direction.into(),
            expires_at,
        )
        .await;
    }

    Err(EngineError::Manager(ERROR_DB_SETUP.to_owned()))
}

pub async fn get_client_messages<'conn, 'a: 'conn, 'b: 'conn>(
    db: &'a mut AsyncDatabase<'conn>,
    filter: ClientMessageFilter<'b>,
) -> Result<Paginated<Message>, EngineError> {
    csml_logger(
        CsmlLog::new(None, None, None, "db call get messages".to_string()),
        LogLvl::Info,
    );
    let ClientMessageFilter { client, .. } = filter;
    csml_logger(
        CsmlLog::new(Some(client), None, None, "db call get messages".to_string()),
        LogLvl::Debug,
    );

    #[cfg(feature = "postgresql-async")]
    if is_postgresql() {
        let db = postgresql_connector::get_db(db)?;

        return postgresql_connector::messages::get_client_messages(db, filter).await;
    }

    Err(EngineError::Manager(ERROR_DB_SETUP.to_owned()))
}
