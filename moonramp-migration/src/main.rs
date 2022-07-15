use std::error::Error;

use chrono::Utc;
use clap::{Parser, Subcommand};
use sea_orm::{entity::*, Database};
use sha3::{Digest, Sha3_256};
use uuid::Uuid;

//use sea_orm::SqlxSqliteConnector;
//use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};

use moonramp_core::{chrono, sea_orm, sha3, tokio, uuid, Hash};
use moonramp_entity::{api_token, merchant, role};
use moonramp_migration::{Migrator, MigratorTrait};

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    #[clap(subcommand)]
    command: Commands,

    #[clap(
        short = 'u',
        long,
        default_value_t = String::from("postgresql://root@localhost:26257/defaultdb?sslmode=disable")
    )]
    db_url: String,
}

#[derive(Subcommand)]
enum Commands {
    Migrate {
        #[clap(short, long)]
        count: Option<u32>,
    },
    Rollback {
        #[clap(short, long)]
        count: Option<u32>,
    },
    Reapply {},
    Nuke {},
    List {},
    CreateMerchant {
        #[clap(short, long)]
        name: String,
        #[clap(short, long)]
        address: String,
        #[clap(short = 'e', long)]
        primary_email: String,
        #[clap(short = 'p', long)]
        primary_phone: String,
    },
    CreateApiToken {
        #[clap(short = 'm', long)]
        merchant_id: String,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    //env_logger::Builder::from_env(Env::default().default_filter_or("moonramp=info")).init();

    let cli = Cli::parse();

    //let opt = SqliteConnectOptions::from_str(&format!("sqlite:{}", cli.db_path))?
    //    .pragma("key", cli.db_secret)
    //    .create_if_missing(true)
    //    .journal_mode(SqliteJournalMode::Wal);

    //let pool = SqlitePoolOptions::new().connect_with(opt).await?;
    //let database = SqlxSqliteConnector::from_sqlx_sqlite_pool(pool);
    let database = Database::connect(&cli.db_url).await?;

    match cli.command {
        Commands::Migrate { count } => {
            Migrator::up(&database, count).await?;
        }
        Commands::Rollback { count } => {
            Migrator::down(&database, count).await?;
        }
        Commands::Reapply {} => {
            Migrator::refresh(&database).await?;
        }
        Commands::Nuke {} => {
            Migrator::fresh(&database).await?;
        }
        Commands::List {} => {
            println!("{:?}", Migrator::status(&database).await);
        }
        Commands::CreateMerchant {
            name,
            address,
            primary_email,
            primary_phone,
        } => {
            let m = merchant::ActiveModel {
                id: Set(Uuid::new_v4().to_simple().to_string()),
                name: Set(name),
                address: Set(address),
                primary_email: Set(primary_email),
                primary_phone: Set(primary_phone),
                created_at: Set(Utc::now()),
            }
            .insert(&database)
            .await?;
            println!("{:?}", m);
        }
        Commands::CreateApiToken { merchant_id } => {
            let mut hasher = Sha3_256::new();
            hasher.update(Uuid::new_v4().to_simple().to_string().as_bytes());
            let token = Hash::try_from(hasher.finalize().to_vec())?;

            let t = api_token::ActiveModel {
                id: Set(Uuid::new_v4().to_simple().to_string()),
                merchant_id: Set(merchant_id.clone()),
                token: Set(token.to_string()),
                created_at: Set(Utc::now()),
            }
            .insert(&database)
            .await?;

            let resources = vec![
                role::Resource::Program,
                role::Resource::Sale,
                role::Resource::Wallet,
            ];
            for r in resources {
                let scopes = vec![
                    role::Scope::Execute,
                    role::Scope::Read,
                    role::Scope::Watch,
                    role::Scope::Write,
                ];
                for s in scopes {
                    role::ActiveModel {
                        id: Set(Uuid::new_v4().to_simple().to_string()),
                        merchant_id: Set(merchant_id.clone()),
                        token_id: Set(t.id.clone()),
                        resource: Set(r.clone()),
                        scope: Set(s),
                        api_group: Set(None),
                        created_at: Set(Utc::now()),
                    }
                    .insert(&database)
                    .await?;
                }
            }
            println!("{:?}", t);
        }
    }
    Ok(())
}
