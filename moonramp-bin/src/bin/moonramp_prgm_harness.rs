use std::sync::Arc;

use chrono::Utc;
use clap::{Parser, Subcommand};
use env_logger::Env;
use log::{error, info};
use sea_orm::{entity::*, query::*, DatabaseConnection};
use sha3::{Digest, Sha3_256};
use tokio::{
    fs,
    time::{Duration, Instant},
};
use uuid::Uuid;

use moonramp_core::{anyhow, chrono, log, sea_orm, sha3, tokio, uuid};
use moonramp_encryption::{
    KeyCustodian, KeyEncryptionKeyCustodian, MasterKeyEncryptionKeyCustodian,
};
use moonramp_entity::{key_encryption_key, merchant};
use moonramp_migration::{Migrator, MigratorTrait};
use moonramp_program_rpc::{BitcoinRpcConfig, Runtime};
use moonramp_wallet_rpc::{BitcoinWallet, Currency, Network, Ticker, Wallet};

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Run {
        #[clap(short, long, default_value_t = String::from("moonramp-lunar-program"))]
        master_key_encryption_key: String,

        #[clap(short = 'u', long, default_value_t = String::from("sqlite::memory:"))]
        db_url: String,

        #[clap(short = 'P', long, parse(from_os_str))]
        program_path: std::path::PathBuf,

        #[clap(short, long, default_value_t = 1)]
        iters: usize,
    },
    //Export {
    //    #[clap(short = 'P', long, parse(from_os_str))]
    //    program_path: std::path::PathBuf,
    //},
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let env_level = Env::default().default_filter_or("moonramp=debug");
    env_logger::Builder::from_env(env_level).init();

    let cli = Cli::parse();

    info!("moonramp-prgm-harness {:?}", env!("CARGO_PKG_VERSION"));

    match cli.command {
        Commands::Run {
            master_key_encryption_key,
            db_url,
            program_path,
            iters,
        } => {
            let database = moonramp_entity::database_connection_pool(&db_url).await?;
            setup_harnessdb(&database).await?;

            let mut hasher = Sha3_256::new();
            hasher.update(master_key_encryption_key.as_bytes());

            let _kek_custodian = {
                let master_custodian =
                    MasterKeyEncryptionKeyCustodian::new(hasher.finalize().to_vec())?;
                let kek = if let Some(kek) = key_encryption_key::Entity::find()
                    .filter(
                        key_encryption_key::Column::MasterKeyEncryptionKeyId
                            .eq(master_custodian.id()),
                    )
                    .order_by_desc(key_encryption_key::Column::CreatedAt)
                    .one(&database)
                    .await?
                {
                    kek
                } else {
                    let secret = master_custodian.gen_secret()?;
                    master_custodian.lock(secret)?.insert(&database).await?
                };
                Arc::new(KeyEncryptionKeyCustodian::new(
                    master_custodian.unlock(kek)?.to_vec(),
                )?)
            };

            let bitcoin_gateway_config = BitcoinRpcConfig {
                uri: "http://localhost:18443".to_string(),
                basic_auth: None,
            };
            info!("Loading program data...");
            let data = fs::read(program_path).await?;
            info!("Compiling program...");
            let start = Instant::now();
            let wasm_mod_bytes = Runtime::compile(&data)?;
            info!("Program compiled in {}ms", start.elapsed().as_millis());
            for _ in 0..iters {
                let w = Wallet::Bitcoin(BitcoinWallet::new_hot(Ticker::BTC, Network::Testnet)?);
                info!("Running program...");
                let start = Instant::now();
                match Runtime::exec(
                    &wasm_mod_bytes,
                    lunar::EntryData::Invoice {
                        wallet: w,
                        currency: Currency::BTC,
                        amount: 0.00001000,
                        user_data: None,
                    },
                    Duration::from_secs(1),
                    bitcoin_gateway_config.clone(),
                )
                .await
                {
                    Ok(exit_data) => info!("Program ExitData: {:?}", exit_data),
                    Err(err) => error!("{}", err),
                }
                info!("Program ran in {}ms", start.elapsed().as_millis());
            }
        } //Commands::Export { program_path } => {}
    }
    Ok(())
}

async fn setup_harnessdb(database: &DatabaseConnection) -> anyhow::Result<()> {
    Migrator::fresh(database).await?;
    Migrator::up(database, None).await?;
    merchant::ActiveModel {
        id: Set(Uuid::new_v4().to_simple().to_string()),
        name: Set("Space Force".to_string()),
        address: Set("The Moon".to_string()),
        primary_email: Set("mission-control@themoon.com".to_string()),
        primary_phone: Set("13337779999".to_string()),
        created_at: Set(Utc::now()),
    }
    .insert(database)
    .await?;

    Ok(())
}
