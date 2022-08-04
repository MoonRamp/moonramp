pub mod api_token;
pub mod cipher;
pub mod currency;
pub mod encryption_key;
pub mod invoice;
pub mod key_encryption_key;
pub mod merchant;
pub mod network;
pub mod program;
pub mod role;
pub mod sale;
pub mod ticker;
pub mod wallet;

use sea_orm::{ConnectOptions, Database, DatabaseConnection};

use moonramp_core::{anyhow, sea_orm};

//use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};

//pub async fn database_connection_pool(
//    db_path: &str,
//    db_secret: &str,
//) -> Result<DatabaseConnection, Box<dyn Error>> {
//    let opt = SqliteConnectOptions::from_str(&format!("sqlite:{}", db_path))?
//        .pragma("key", db_secret.to_string())
//        .create_if_missing(false)
//        .journal_mode(SqliteJournalMode::Wal);
//
//    let pool = SqlitePoolOptions::new().connect_with(opt).await?;
//    Ok(SqlxSqliteConnector::from_sqlx_sqlite_pool(pool))
//}

pub async fn database_connection_pool(url: &str) -> anyhow::Result<DatabaseConnection> {
    let mut opts = ConnectOptions::from(url);
    opts.max_connections(10);
    Ok(Database::connect(opts).await?)
}
