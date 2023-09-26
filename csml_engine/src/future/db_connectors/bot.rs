#[cfg(feature = "postgresql-async")]
use crate::future::db_connectors::{is_postgresql, postgresql_connector};

use crate::data::AsyncDatabase;
use crate::error_messages::ERROR_DB_SETUP;
use crate::models::BotVersion;
use crate::{CsmlBot, EngineError};
use csml_interpreter::data::csml_logs::*;

pub async fn create_bot_version(
    bot_id: String,
    csml_bot: CsmlBot,
    db: &mut AsyncDatabase<'_>,
) -> Result<String, EngineError> {
    csml_logger(
        CsmlLog::new(
            None,
            None,
            None,
            format!("db call create bot version, bot_id: {:?}", bot_id),
        ),
        LogLvl::Info,
    );
    csml_logger(
        CsmlLog::new(
            None,
            None,
            None,
            format!(
                "db call create bot version, bot_id: {:?}, csml_bot: {:?}",
                bot_id, csml_bot
            ),
        ),
        LogLvl::Debug,
    );

    #[cfg(feature = "postgresql-async")]
    if is_postgresql() {
        let db = postgresql_connector::get_db(db)?;

        let serializable_bot = crate::data::to_serializable_bot(&csml_bot);
        let bot = serde_json::json!(serializable_bot).to_string();

        let version_id = postgresql_connector::bot::create_bot_version(bot_id, bot, db).await?;

        return Ok(version_id);
    }

    Err(EngineError::Manager(ERROR_DB_SETUP.to_owned()))
}

pub async fn get_last_bot_version(
    bot_id: &str,
    db: &mut AsyncDatabase<'_>,
) -> Result<Option<BotVersion>, EngineError> {
    csml_logger(
        CsmlLog::new(
            None,
            None,
            None,
            format!("db call get last bot version, bot_id: {:?}", bot_id),
        ),
        LogLvl::Info,
    );

    #[cfg(feature = "postgresql-async")]
    if is_postgresql() {
        let db = postgresql_connector::get_db(db)?;
        return postgresql_connector::bot::get_last_bot_version(bot_id, db).await;
    }

    Err(EngineError::Manager(ERROR_DB_SETUP.to_owned()))
}

pub async fn get_by_version_id(
    version_id: &str,
    _bot_id: &str,
    db: &mut AsyncDatabase<'_>,
) -> Result<Option<BotVersion>, EngineError> {
    csml_logger(
        CsmlLog::new(
            None,
            None,
            None,
            format!("db call get by version id, version_id: {:?}", version_id),
        ),
        LogLvl::Info,
    );
    csml_logger(
        CsmlLog::new(
            None,
            None,
            None,
            format!(
                "db call get by version id, version_id: {:?}, bot_id: {:?}",
                version_id, _bot_id
            ),
        ),
        LogLvl::Debug,
    );

    #[cfg(feature = "postgresql-async")]
    if is_postgresql() {
        let db = postgresql_connector::get_db(db)?;
        return postgresql_connector::bot::get_bot_by_version_id(version_id, db).await;
    }

    Err(EngineError::Manager(ERROR_DB_SETUP.to_owned()))
}

pub async fn get_bot_versions(
    bot_id: &str,
    limit: Option<u32>,
    pagination_key: Option<u32>,
    db: &mut AsyncDatabase<'_>,
) -> Result<serde_json::Value, EngineError> {
    csml_logger(
        CsmlLog::new(
            None,
            None,
            None,
            format!("db call get bot versions, bot_id: {:?}", bot_id),
        ),
        LogLvl::Info,
    );
    csml_logger(
        CsmlLog::new(
            None,
            None,
            None,
            format!(
                "db call get bot versions, bot_id: {:?}, limit {:?}, pagination_key {:?}",
                bot_id, limit, pagination_key
            ),
        ),
        LogLvl::Debug,
    );

    #[cfg(feature = "postgresql-async")]
    if is_postgresql() {
        let db = postgresql_connector::get_db(db)?;
        return postgresql_connector::bot::get_bot_versions(bot_id, limit, pagination_key, db)
            .await;
    }

    Err(EngineError::Manager(ERROR_DB_SETUP.to_owned()))
}

pub async fn delete_bot_version(
    _bot_id: &str,
    version_id: &str,
    db: &mut AsyncDatabase<'_>,
) -> Result<(), EngineError> {
    csml_logger(
        CsmlLog::new(
            None,
            None,
            None,
            format!("db call delete bot version, version_id: {:?}", version_id),
        ),
        LogLvl::Info,
    );
    csml_logger(
        CsmlLog::new(
            None,
            None,
            None,
            format!("db call delete bot version, version_id: {:?}", version_id),
        ),
        LogLvl::Debug,
    );

    #[cfg(feature = "postgresql-async")]
    if is_postgresql() {
        let db = postgresql_connector::get_db(db)?;
        return postgresql_connector::bot::delete_bot_version(version_id, db).await;
    }

    Err(EngineError::Manager(ERROR_DB_SETUP.to_owned()))
}

pub async fn delete_bot_versions(
    bot_id: &str,
    db: &mut AsyncDatabase<'_>,
) -> Result<(), EngineError> {
    csml_logger(
        CsmlLog::new(None, None, None, "db call delete bot versions".to_string()),
        LogLvl::Info,
    );
    csml_logger(
        CsmlLog::new(
            None,
            None,
            None,
            format!("db call delete bot versions, bot_id: {:?}", bot_id),
        ),
        LogLvl::Debug,
    );

    #[cfg(feature = "postgresql-async")]
    if is_postgresql() {
        let db = postgresql_connector::get_db(db)?;
        return postgresql_connector::bot::delete_bot_versions(bot_id, db).await;
    }

    Err(EngineError::Manager(ERROR_DB_SETUP.to_owned()))
}

pub async fn delete_all_bot_data(
    bot_id: &str,
    db: &mut AsyncDatabase<'_>,
) -> Result<(), EngineError> {
    csml_logger(
        CsmlLog::new(None, None, None, "db call delete all bot data".to_string()),
        LogLvl::Info,
    );
    csml_logger(
        CsmlLog::new(
            None,
            None,
            None,
            format!("db call delete all bot data, bot_id: {:?}", bot_id),
        ),
        LogLvl::Debug,
    );

    #[cfg(feature = "postgresql-async")]
    if is_postgresql() {
        delete_bot_versions(bot_id, db).await?;

        let db = postgresql_connector::get_db(db)?;

        postgresql_connector::conversations::delete_all_bot_data(bot_id, db).await?;
        postgresql_connector::memories::delete_all_bot_data(bot_id, db).await?;
        postgresql_connector::state::delete_all_bot_data(bot_id, db).await?;
        return Ok(());
    }

    Err(EngineError::Manager(ERROR_DB_SETUP.to_owned()))
}
