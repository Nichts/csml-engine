pub mod bot;
pub mod conversations;
pub mod memories;
pub mod messages;
pub mod state;

pub mod pagination;

pub mod models;
pub mod schema;

pub mod expired_data;

use crate::{Database, AsyncDatabase, EngineError, AsyncPostgresqlClient, PostgresqlClient};

use diesel::prelude::{Connection, PgConnection};
use diesel_async::{AsyncConnection, AsyncPgConnection};
use diesel_migrations::{EmbeddedMigrations, HarnessWithOutput, MigrationHarness};

const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations/postgresql");

pub async fn init() -> Result<AsyncDatabase<'static>, EngineError> {
    let uri = match std::env::var("POSTGRESQL_URL") {
        Ok(var) => var,
        _ => "".to_owned(),
    };

    let pg_connection =
        AsyncPgConnection::establish(&uri).await.unwrap_or_else(|_| panic!("Error connecting to {}", uri));

    let db = AsyncDatabase::Postgresql(AsyncPostgresqlClient::new(pg_connection));
    Ok(db)
}

pub fn init_sync() -> Result<Database<'static>, EngineError> {
    let uri = match std::env::var("POSTGRESQL_URL") {
        Ok(var) => var,
        _ => "".to_owned(),
    };

    let pg_connection =
        PgConnection::establish(&uri).unwrap_or_else(|_| panic!("Error connecting to {}", uri));

    let db = Database::Postgresql(PostgresqlClient::new(pg_connection));
    Ok(db)
}

pub fn make_migrations() -> Result<(), EngineError> {
    let uri = match std::env::var("POSTGRESQL_URL") {
        Ok(var) => var,
        _ => "".to_owned(),
    };

    let mut pg_connection =
        PgConnection::establish(&uri).unwrap_or_else(|_| panic!("Error connecting to {}", uri));

    let mut harness = HarnessWithOutput::write_to_stdout(&mut pg_connection);
    harness.run_pending_migrations(MIGRATIONS)?;

    Ok(())
}

pub fn get_db<'a, 'b>(db: &'a mut AsyncDatabase<'b>) -> Result<&'a mut AsyncPostgresqlClient<'b>, EngineError> {
    match db {
        AsyncDatabase::Postgresql(db) => Ok(db),
        _ => Err(EngineError::Manager(
            "Postgresql connector is not setup correctly".to_owned(),
        )),
    }
}
