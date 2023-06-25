/**
 * This module defines the interactions between the CSML Manager and the underlying
 * database engines.
 *
 * There are several engines to choose from (see module features). To use one
 * of the database options, the ENGINE_DB_TYPE env var must be set
 * to one of the accepted values:
 *
 * - `mongodb`: requires a MongoDB-compatible database and additional variables:
 *   - MONGODB_HOST
 *   - MONGODB_PORT
 *   - MONGODB_DATABASE
 *   - MONGODB_USERNAME
 *   - MONGODB_PASSWORD
 *
 * - `dynamodb`: requires a DynamoDB-compatible database (on AWS, or dynamodb-local
 * for dev purposes). A S3-compatible storage is also needed for storing bots in the engine.
 * The following env vars are required (alternatively if deploying on AWS, prefer using IAM roles)
 *   - AWS_REGION
 *   - AWS_ACCESS_KEY_ID
 *   - AWS_SECRET_ACCESS_KEY
 *   - AWS_DYNAMODB_TABLE
 *   - AWS_DYNAMODB_ENDPOINT optional, defaults to the dynamodb endpoint for the given region.
 *   - AWS_S3_BUCKET
 *   - AWS_S3_ENDPOINT optional, defaults to the S3 endpoint for the given region
 * Both AWS_REGION AND AWS_DYNAMODB_ENDPOINT must be set to use a custom dynamodb-compatible DB.
 *
 * If the ENGINE_DB_TYPE env var is not set, mongodb is used by default.
 *
 * To add a new DB type, please use one of the existing templates implementations.
 * Each method of each module must be fully reimplemented in order to extend the "generic"
 * implementation at the root of db_connectors directory.
 */
use crate::data::{AsyncDatabase, EngineError};
use crate::error_messages::ERROR_DB_SETUP;

#[cfg(feature = "postgresql-async")]
use self::postgresql as postgresql_connector;

pub mod bot;
pub mod conversations;
pub mod memories;
pub mod messages;
pub mod state;

pub mod clean_db;
pub mod user;
pub mod utils;

pub mod db_test;

#[cfg(feature = "postgresql-async")]
pub(crate) mod postgresql;


#[cfg(feature = "postgresql-async")]
pub fn is_postgresql() -> bool {
    match std::env::var("ENGINE_DB_TYPE") {
        Ok(val) => val == *"postgresql",
        Err(_) => false,
    }
}

pub async fn init_db() -> Result<AsyncDatabase<'static>, EngineError> {
    #[cfg(feature = "postgresql-async")]
    if is_postgresql() {
        return postgresql_connector::init().await;
    }

    Err(EngineError::Manager(ERROR_DB_SETUP.to_owned()))
}
