#[cfg(feature = "dynamo")]
use crate::db_connectors::{dynamodb as dynamodb_connector, is_dynamodb};
#[cfg(feature = "mongo")]
use crate::db_connectors::{is_mongodb, mongodb as mongodb_connector};
#[cfg(feature = "postgresql")]
use crate::db_connectors::{is_postgresql, postgresql_connector};
#[cfg(feature = "sqlite")]
use crate::db_connectors::{is_sqlite, sqlite_connector};

use crate::db_connectors::utils::*;
use crate::error_messages::ERROR_DB_SETUP;
use crate::{Database, EngineError};
use csml_interpreter::data::csml_logs::{csml_logger, CsmlLog, LogLvl};
use csml_interpreter::data::Client;

pub fn delete_state_key(
    client: &Client,
    _type: &str,
    key: &str,
    db: &mut Database,
) -> Result<(), EngineError> {
    csml_logger(
        CsmlLog::new(
            None,
            None,
            None,
            format!("db call delete state key: {:?}, type: {:?}", key, _type),
        ),
        LogLvl::Info,
    );
    csml_logger(
        CsmlLog::new(
            Some(client),
            None,
            None,
            format!("db call delete state key: {:?}, type: {:?}", key, _type),
        ),
        LogLvl::Debug,
    );

    #[cfg(feature = "mongo")]
    if is_mongodb() {
        let db = mongodb_connector::get_db(db)?;
        return mongodb_connector::state::delete_state_key(client, _type, key, db);
    }

    #[cfg(feature = "dynamo")]
    if is_dynamodb() {
        let db = dynamodb_connector::get_db(db)?;
        return dynamodb_connector::state::delete_state_key(client, _type, key, db);
    }

    #[cfg(feature = "postgresql")]
    if is_postgresql() {
        let db = postgresql_connector::get_db(db)?;
        return postgresql_connector::state::delete_state_key(client, _type, key, db);
    }

    #[cfg(feature = "sqlite")]
    if is_sqlite() {
        let db = sqlite_connector::get_db(db)?;
        return sqlite_connector::state::delete_state_key(client, _type, key, db);
    }

    Err(EngineError::Manager(ERROR_DB_SETUP.to_owned()))
}

pub fn get_state_key(
    client: &Client,
    _type: &str,
    _key: &str,
    db: &mut Database,
) -> Result<Option<serde_json::Value>, EngineError> {
    csml_logger(
        CsmlLog::new(
            None,
            None,
            None,
            format!("db call get state key: {:?}, type: {:?}", _key, _type),
        ),
        LogLvl::Info,
    );
    csml_logger(
        CsmlLog::new(
            Some(client),
            None,
            None,
            format!("db call get state key: {:?}, type: {:?}", _key, _type),
        ),
        LogLvl::Debug,
    );

    #[cfg(feature = "mongo")]
    if is_mongodb() {
        let db = mongodb_connector::get_db(db)?;
        return mongodb_connector::state::get_state_key(client, _type, _key, db);
    }

    #[cfg(feature = "dynamo")]
    if is_dynamodb() {
        let db = dynamodb_connector::get_db(db)?;
        return dynamodb_connector::state::get_state_key(client, _type, _key, db);
    }

    #[cfg(feature = "postgresql")]
    if is_postgresql() {
        let db = postgresql_connector::get_db(db)?;
        return postgresql_connector::state::get_state_key(client, _type, _key, db);
    }

    #[cfg(feature = "sqlite")]
    if is_sqlite() {
        let db = sqlite_connector::get_db(db)?;
        return sqlite_connector::state::get_state_key(client, _type, _key, db);
    }

    Err(EngineError::Manager(ERROR_DB_SETUP.to_owned()))
}

pub fn get_current_state(
    client: &Client,
    db: &mut Database,
) -> Result<Option<serde_json::Value>, EngineError> {
    csml_logger(
        CsmlLog::new(None, None, None, "db call get current state".to_string()),
        LogLvl::Info,
    );
    csml_logger(
        CsmlLog::new(
            Some(client),
            None,
            None,
            "db call get current state".to_string(),
        ),
        LogLvl::Debug,
    );

    #[cfg(feature = "mongo")]
    if is_mongodb() {
        let db = mongodb_connector::get_db(db)?;
        return mongodb_connector::state::get_current_state(client, db); // "hold", "position"
    }

    #[cfg(feature = "dynamo")]
    if is_dynamodb() {
        let db = dynamodb_connector::get_db(db)?;
        return dynamodb_connector::state::get_current_state(client, db);
    }

    #[cfg(feature = "postgresql")]
    if is_postgresql() {
        let db = postgresql_connector::get_db(db)?;
        return postgresql_connector::state::get_current_state(client, db);
    }

    #[cfg(feature = "sqlite")]
    if is_sqlite() {
        let db = sqlite_connector::get_db(db)?;
        return sqlite_connector::state::get_current_state(client, db);
    }

    Err(EngineError::Manager(ERROR_DB_SETUP.to_owned()))
}

pub fn set_state_items(
    _client: &Client,
    _type: &str,
    _keys_values: Vec<(&str, &serde_json::Value)>,
    ttl: Option<chrono::Duration>,
    _db: &mut Database,
) -> Result<(), EngineError> {
    csml_logger(
        CsmlLog::new(
            None,
            None,
            None,
            format!(
                "db call set state type: {:?}, keys and values {:?}",
                _type, _keys_values
            ),
        ),
        LogLvl::Info,
    );
    csml_logger(
        CsmlLog::new(
            Some(_client),
            None,
            None,
            format!(
                "db call set state type: {:?}, keys and values {:?}",
                _type, _keys_values
            ),
        ),
        LogLvl::Debug,
    );

    #[cfg(feature = "mongo")]
    if is_mongodb() {
        let db = mongodb_connector::get_db(_db)?;
        let expires_at = get_expires_at_for_mongodb(ttl);

        return mongodb_connector::state::set_state_items(
            _client,
            _type,
            _keys_values,
            expires_at,
            &db,
        );
    }

    #[cfg(feature = "dynamo")]
    if is_dynamodb() {
        let db = dynamodb_connector::get_db(_db)?;
        let expires_at = get_expires_at_for_dynamodb(ttl);

        return dynamodb_connector::state::set_state_items(
            _client,
            _type,
            _keys_values,
            expires_at,
            db,
        );
    }

    #[cfg(feature = "postgresql")]
    if is_postgresql() {
        let db = postgresql_connector::get_db(_db)?;
        let expires_at = get_expires_at_for_postgresql(ttl);

        return postgresql_connector::state::set_state_items(
            _client,
            _type,
            _keys_values,
            expires_at,
            db,
        );
    }

    #[cfg(feature = "sqlite")]
    if is_sqlite() {
        let db = sqlite_connector::get_db(_db)?;
        let expires_at = get_expires_at_for_sqlite(ttl);

        return sqlite_connector::state::set_state_items(
            _client,
            _type,
            _keys_values,
            expires_at,
            db,
        );
    }

    Err(EngineError::Manager(ERROR_DB_SETUP.to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db_connectors::init_db;
    use crate::{Hold, IndexInfo};
    use core::panic;

    #[test]
    fn ok_hold() {
        let client = Client {
            bot_id: "bot_id".to_owned(),
            channel_id: "channel_id".to_owned(),
            user_id: "test".to_owned(),
        };
        let mut db = init_db().unwrap();

        let hash = "Hash".to_owned();
        let index_info = Hold {
            index: IndexInfo {
                command_index: 42,
                loop_index: vec![],
            },
            step_vars: serde_json::json!({}),
            step_name: "step_name".to_owned(),
            flow_name: "flow_name".to_owned(),
            previous: None,
            secure: false,
        };

        let state_hold: serde_json::Value = serde_json::json!({
            "index": index_info.index,
            "step_vars": index_info.step_vars,
            "hash": hash
        });

        set_state_items(
            &client,
            "hold",
            vec![("position", &state_hold)],
            None,
            &mut db,
        )
        .unwrap();

        let hold = get_state_key(&client, "hold", "position", &mut db)
            .unwrap()
            .unwrap();

        let index_result = match serde_json::from_value::<IndexInfo>(hold["index"].clone()) {
            Ok(index) => index,
            Err(_) => panic!("value not found in db"),
        };

        if index_result.loop_index != index_info.index.loop_index
            && index_result.command_index != index_info.index.command_index
        {
            panic!("db get hodl got the wrong value")
        }

        delete_state_key(&client, "hold", "position", &mut db).unwrap();

        if get_state_key(&client, "hold", "position", &mut db)
            .unwrap()
            .is_some()
        {
            panic!("get_state_key should not have found a hold because it has deleted just before")
        }
    }
}
