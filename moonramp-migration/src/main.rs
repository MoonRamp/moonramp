use std::{error::Error, sync::Arc};

use argon2::{
    password_hash::{
        rand_core::{OsRng, RngCore},
        PasswordHasher, SaltString,
    },
    Argon2,
};
use chrono::Utc;
use clap::{Parser, Subcommand};
use sea_orm::{entity::*, Database, QueryFilter, QueryOrder};
use serde_json::json;
use sha3::{Digest, Sha3_256};

//use sea_orm::SqlxSqliteConnector;
//use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};

use moonramp_core::{argon2, chrono, sea_orm, serde_json, sha3, tokio, ApiCredential, Hash};
use moonramp_encryption::{
    EncryptionKeyCustodian, KeyCustodian, KeyEncryptionKeyCustodian,
    MasterKeyEncryptionKeyCustodian, MerchantScopedSecret,
};
use moonramp_entity::{api_token, cipher::Cipher, key_encryption_key, merchant, role};
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
        merchant_hash: Hash,

        #[clap(short = 'M', long)]
        master_key_encryption_key: String,
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
            let mut hasher = Sha3_256::new();
            hasher.update(name.as_bytes());
            let merchant_hash = Hash::try_from(hasher.finalize().to_vec())?;
            let m = merchant::ActiveModel {
                hash: Set(merchant_hash),
                name: Set(name),
                address: Set(address),
                primary_email: Set(primary_email),
                primary_phone: Set(primary_phone),
                created_at: Set(Utc::now()),
            }
            .insert(&database)
            .await?;
            println!("{}", serde_json::to_string(&m)?);
        }
        Commands::CreateApiToken {
            merchant_hash,
            master_key_encryption_key,
        } => {
            let kek_custodian = {
                let master_custodian = MasterKeyEncryptionKeyCustodian::new(
                    master_key_encryption_key.as_bytes().to_vec(),
                )?;
                let kek = if let Some(kek) = key_encryption_key::Entity::find()
                    .filter(
                        key_encryption_key::Column::MasterKeyEncryptionKeyHash
                            .eq(master_custodian.hash()),
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

            let mut token_secret = vec![0u8; 32];
            OsRng.fill_bytes(&mut token_secret);

            let mut hasher = Sha3_256::new();
            hasher.update(&token_secret);
            let token_hash = Hash::try_from(hasher.finalize().to_vec())?;

            let ek = kek_custodian
                .lock(MerchantScopedSecret {
                    merchant_hash: merchant_hash.clone(),
                    secret: kek_custodian.gen_secret()?,
                })?
                .insert(&database)
                .await?;

            let ek_custodian = EncryptionKeyCustodian::new(
                kek_custodian.unlock(ek)?.secret.to_vec(),
                Cipher::Aes256GcmSiv,
            )?;

            let salt = SaltString::generate(&mut OsRng);

            let argon2 = Argon2::default();
            let password_hash = argon2.hash_password(&token_secret, &salt)?;

            let (nonce, ciphertext) = ek_custodian.encrypt(password_hash.to_string().as_bytes())?;

            let api_credential = ApiCredential {
                hash: token_hash.clone(),
                secret: token_secret,
            };

            let t = api_token::ActiveModel {
                hash: Set(token_hash),
                merchant_hash: Set(merchant_hash.clone()),
                cipher: Set(Cipher::Aes256GcmSiv),
                encryption_key_hash: Set(ek_custodian.hash()),
                blob: Set(ciphertext),
                nonce: Set(nonce),
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
                    let mut hasher = Sha3_256::new();
                    hasher.update(&merchant_hash);
                    hasher.update(&t.hash);
                    hasher.update(format!("{:?}", r).as_bytes());
                    hasher.update(format!("{:?}", s).as_bytes());
                    let role_hash = Hash::try_from(hasher.finalize().to_vec())?;
                    role::ActiveModel {
                        hash: Set(role_hash),
                        merchant_hash: Set(merchant_hash.clone()),
                        token_hash: Set(t.hash.clone()),
                        resource: Set(r.clone()),
                        scope: Set(s),
                        api_group: Set(None),
                        created_at: Set(Utc::now()),
                    }
                    .insert(&database)
                    .await?;
                }
            }
            println!(
                "{}",
                serde_json::to_string(&json!({
                    "api_credential": api_credential.to_bearer()?,
                    "api_token": t
                }))?
            );
        }
    }
    Ok(())
}
