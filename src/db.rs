use std::time::Duration;

use sqlx::{PgPool, postgres::PgPoolOptions};
use thiserror::Error;

use crate::config::Config;

const CONNECT_ATTEMPTS: u32 = 30;
const CONNECT_DELAY: Duration = Duration::from_secs(1);

#[derive(Debug, Error)]
pub enum DatabaseError {
    #[error("could not connect to PostgreSQL: {0}")]
    Connect(#[source] sqlx::Error),
    #[error("could not apply database migrations: {0}")]
    Migrate(#[from] sqlx::migrate::MigrateError),
}

pub async fn connect(config: &Config) -> Result<PgPool, DatabaseError> {
    let mut last_error = None;

    for attempt in 1..=CONNECT_ATTEMPTS {
        match PgPoolOptions::new()
            .max_connections(10)
            .acquire_timeout(Duration::from_secs(5))
            .connect(&config.database_url)
            .await
        {
            Ok(pool) => {
                sqlx::migrate!("./migrations").run(&pool).await?;
                tracing::info!(attempt, "database ready");
                return Ok(pool);
            }
            Err(error) => {
                tracing::warn!(
                    attempt,
                    max_attempts = CONNECT_ATTEMPTS,
                    "database is not ready yet"
                );
                last_error = Some(error);
                if attempt < CONNECT_ATTEMPTS {
                    tokio::time::sleep(CONNECT_DELAY).await;
                }
            }
        }
    }

    Err(DatabaseError::Connect(last_error.expect(
        "the connection loop always records an error on failure",
    )))
}
