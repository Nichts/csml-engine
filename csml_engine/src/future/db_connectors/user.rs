#[cfg(feature = "postgresql-async")]
use crate::future::db_connectors::{is_postgresql, postgresql_connector};

use crate::error_messages::ERROR_DB_SETUP;
use crate::{Client, Database, EngineError};
use csml_interpreter::data::csml_logs::{csml_logger, CsmlLog, LogLvl};
use crate::data::AsyncDatabase;

pub async fn delete_client(client: &Client, db: &mut AsyncDatabase<'_>) -> Result<(), EngineError> {
    csml_logger(
        CsmlLog::new(None, None, None, "db call delete client".to_string()),
        LogLvl::Info,
    );
    csml_logger(
        CsmlLog::new(Some(client), None, None, "db call delete client".to_string()),
        LogLvl::Debug,
    );

    #[cfg(feature = "postgresql-async")]
    if is_postgresql() {
        let db = postgresql_connector::get_db(db)?;

        postgresql_connector::conversations::delete_user_conversations(client, db).await?;
        postgresql_connector::memories::delete_client_memories(client, db).await?;
        postgresql_connector::messages::delete_user_messages(client, db).await?;
        postgresql_connector::state::delete_user_state(client, db).await?;

        return Ok(());
    }

    Err(EngineError::Manager(ERROR_DB_SETUP.to_owned()))
}
