#[cfg(feature = "postgresql-async")]
use crate::future::db_connectors::{is_postgresql, postgresql_connector};

use crate::error_messages::ERROR_DB_SETUP;
use crate::{EngineError};
use crate::data::AsyncDatabase;

pub async fn delete_expired_data(_db: &mut AsyncDatabase<'_>) -> Result<(), EngineError> {
    #[cfg(feature = "postgresql-async")]
    if is_postgresql() {
        let db = postgresql_connector::get_db(_db)?;

        postgresql_connector::expired_data::delete_expired_data(db).await?;

        return Ok(());
    }

    Err(EngineError::Manager(ERROR_DB_SETUP.to_owned()))
}
