pub mod bot;
pub mod conversations;
pub mod memories;
pub mod messages;
pub mod state;

pub mod pagination;

pub mod models;
pub mod schema;

pub mod expired_data;

use crate::{AsyncDatabase, AsyncPostgresqlClient, EngineError};

use diesel_async::{AsyncConnection, AsyncPgConnection};

pub async fn init() -> Result<AsyncDatabase<'static>, EngineError> {
    let uri = match std::env::var("POSTGRESQL_URL") {
        Ok(var) => var,
        _ => "".to_owned(),
    };

    let pg_connection = AsyncPgConnection::establish(&uri)
        .await
        .unwrap_or_else(|_| panic!("Error connecting to {}", uri));

    let db = AsyncDatabase::Postgresql(AsyncPostgresqlClient::new(pg_connection));
    Ok(db)
}

pub fn get_db<'a, 'b>(
    db: &'a mut AsyncDatabase<'b>,
) -> Result<&'a mut AsyncPostgresqlClient<'b>, EngineError> {
    match db {
        AsyncDatabase::Postgresql(db) => Ok(db),
        _ => Err(EngineError::Manager(
            "Postgresql connector is not setup correctly".to_owned(),
        )),
    }
}
