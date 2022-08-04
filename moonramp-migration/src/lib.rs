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
    use chrono::Utc;
    use sea_orm::{entity::*, DatabaseConnection};
    use sha3::{Digest, Sha3_256};
    use uuid::Uuid;

    use moonramp_core::{anyhow, chrono, sea_orm, sha3, uuid, Hash};
    use moonramp_entity::{api_token, merchant, role};

    pub async fn add_role(
        database: &DatabaseConnection,
        merchant_id: String,
        token_id: String,
        resource: role::Resource,
        scope: role::Scope,
    ) -> anyhow::Result<()> {
        role::ActiveModel {
            id: Set(Uuid::new_v4().to_simple().to_string()),
            merchant_id: Set(merchant_id),
            token_id: Set(token_id),
            resource: Set(resource),
            scope: Set(scope),
            api_group: Set(None),
            created_at: Set(Utc::now()),
        }
        .insert(database)
        .await?;
        Ok(())
    }

    pub async fn setup_testdb(database: &DatabaseConnection) -> anyhow::Result<api_token::Model> {
        Migrator::up(&database, None).await?;
        let m = merchant::ActiveModel {
            id: Set(Uuid::new_v4().to_simple().to_string()),
            name: Set("Moonramp".to_string()),
            address: Set("The Moon".to_string()),
            primary_email: Set("developers@moonramp.org".to_string()),
            primary_phone: Set("12223334444".to_string()),
            created_at: Set(Utc::now()),
        }
        .insert(database)
        .await?;

        let mut hasher = Sha3_256::new();
        hasher.update(Uuid::new_v4().to_simple().to_string().as_bytes());
        let token = Hash::try_from(hasher.finalize().to_vec())?;

        let t = api_token::ActiveModel {
            id: Set(Uuid::new_v4().to_simple().to_string()),
            merchant_id: Set(m.id.clone()),
            token: Set(token.to_string()),
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
                add_role(database, m.id.clone(), t.id.clone(), r.clone(), s.clone()).await?;
            }
        }
        Ok(t)
    }
}
