pub use sea_orm_migration::prelude::*;

mod m20220330_000001_create_merchants_table;
mod m20220330_000002_create_api_tokens_table;
mod m20220330_000003_create_roles_table;
mod m20220330_000004_create_key_encryption_keys_table;
mod m20220330_000005_create_encryption_keys_table;
mod m20220330_000006_create_programs_table;
mod m20220330_000007_create_wallets_table;
mod m20220504_000008_create_invoices_table;
mod m20220504_000009_create_sales_table;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20220330_000001_create_merchants_table::Migration),
            Box::new(m20220330_000002_create_api_tokens_table::Migration),
            Box::new(m20220330_000003_create_roles_table::Migration),
            Box::new(m20220330_000004_create_key_encryption_keys_table::Migration),
            Box::new(m20220330_000005_create_encryption_keys_table::Migration),
            Box::new(m20220330_000006_create_programs_table::Migration),
            Box::new(m20220330_000007_create_wallets_table::Migration),
            Box::new(m20220504_000008_create_invoices_table::Migration),
            Box::new(m20220504_000009_create_sales_table::Migration),
        ]
    }
}

#[cfg(feature = "testing")]
pub mod testing {
    use super::{Migrator, MigratorTrait};

    use std::sync::Arc;

    use argon2::{
        password_hash::{
            rand_core::{OsRng, RngCore},
            PasswordHasher, SaltString,
        },
        Argon2,
    };
    use chrono::Utc;
    use sea_orm::{entity::*, DatabaseConnection, QueryFilter, QueryOrder};
    use sha3::{Digest, Sha3_256};

    use moonramp_core::{anyhow, argon2, chrono, sea_orm, sha3, ApiCredential, Hash};
    use moonramp_encryption::{
        EncryptionKeyCustodian, KeyCustodian, KeyEncryptionKeyCustodian,
        MasterKeyEncryptionKeyCustodian, MerchantScopedSecret,
    };
    use moonramp_entity::{api_token, cipher::Cipher, key_encryption_key, merchant, role};

    async fn add_role(
        database: &DatabaseConnection,
        merchant_hash: Hash,
        token_hash: Hash,
        resource: role::Resource,
        scope: role::Scope,
    ) -> anyhow::Result<()> {
        let mut hasher = Sha3_256::new();
        hasher.update(&merchant_hash);
        hasher.update(&token_hash);
        hasher.update(format!("{:?}", resource).as_bytes());
        hasher.update(format!("{:?}", scope).as_bytes());
        let role_hash = Hash::try_from(hasher.finalize().to_vec())?;
        role::ActiveModel {
            hash: Set(role_hash),
            merchant_hash: Set(merchant_hash),
            token_hash: Set(token_hash),
            resource: Set(resource),
            scope: Set(scope),
            api_group: Set(None),
            created_at: Set(Utc::now()),
        }
        .insert(database)
        .await?;
        Ok(())
    }

    pub async fn setup_testdb(
        database: &DatabaseConnection,
        password: &str,
    ) -> anyhow::Result<(
        Arc<KeyEncryptionKeyCustodian>,
        ApiCredential,
        api_token::Model,
    )> {
        Migrator::up(&database, None).await?;

        let kek_custodian = {
            let master_custodian =
                MasterKeyEncryptionKeyCustodian::new(password.as_bytes().to_vec())?;
            let kek = if let Some(kek) = key_encryption_key::Entity::find()
                .filter(
                    key_encryption_key::Column::MasterKeyEncryptionKeyHash
                        .eq(master_custodian.hash()),
                )
                .order_by_desc(key_encryption_key::Column::CreatedAt)
                .one(database)
                .await?
            {
                kek
            } else {
                let secret = master_custodian.gen_secret()?;
                master_custodian.lock(secret)?.insert(database).await?
            };
            Arc::new(KeyEncryptionKeyCustodian::new(
                master_custodian.unlock(kek)?.to_vec(),
            )?)
        };
        inner_setup_testdb(database, kek_custodian).await
    }

    pub async fn inner_setup_testdb(
        database: &DatabaseConnection,
        kek_custodian: Arc<KeyEncryptionKeyCustodian>,
    ) -> anyhow::Result<(
        Arc<KeyEncryptionKeyCustodian>,
        ApiCredential,
        api_token::Model,
    )> {
        let mut hasher = Sha3_256::new();
        hasher.update(b"Moonramp");
        let merchant_hash = Hash::try_from(hasher.finalize().to_vec())?;
        let m = merchant::ActiveModel {
            hash: Set(merchant_hash.clone()),
            name: Set("Moonramp".to_string()),
            address: Set("The Moon".to_string()),
            primary_email: Set("developers@moonramp.org".to_string()),
            primary_phone: Set("12223334444".to_string()),
            created_at: Set(Utc::now()),
        }
        .insert(database)
        .await?;

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
            .insert(database)
            .await?;

        let ek_custodian = EncryptionKeyCustodian::new(
            kek_custodian.unlock(ek)?.secret.to_vec(),
            Cipher::Aes256GcmSiv,
        )?;

        let salt = SaltString::generate(&mut OsRng);

        let argon2 = Argon2::default();
        let password_hash = argon2.hash_password(&token_secret, &salt)?;

        let api_credential = ApiCredential {
            hash: token_hash.clone(),
            secret: token_secret,
        };

        let (nonce, ciphertext) = ek_custodian.encrypt(password_hash.to_string().as_bytes())?;

        let t = api_token::ActiveModel {
            hash: Set(token_hash),
            merchant_hash: Set(m.hash.clone()),
            cipher: Set(Cipher::Aes256GcmSiv),
            encryption_key_hash: Set(ek_custodian.hash()),
            blob: Set(ciphertext),
            nonce: Set(nonce),
            created_at: Set(Utc::now()),
        }
        .insert(database)
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
                add_role(
                    database,
                    m.hash.clone(),
                    t.hash.clone(),
                    r.clone(),
                    s.clone(),
                )
                .await?;
            }
        }
        Ok((kek_custodian, api_credential, t))
    }
}
