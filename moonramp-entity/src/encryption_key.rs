use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

use moonramp_core::{chrono, sea_orm, serde, Hash};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Deserialize, Serialize)]
#[sea_orm(table_name = "encryption_keys")]
#[serde(crate = "moonramp_core::serde")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false, column_type = "Text")]
    pub id: Hash,
    #[sea_orm(indexed, column_type = "Text")]
    pub merchant_id: String,
    #[sea_orm(indexed, column_type = "Text")]
    pub key_encryption_key_id: Hash,
    pub cipher: super::cipher::Cipher,
    pub key: Vec<u8>,
    pub nonce: Vec<u8>,
    pub created_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::key_encryption_key::Entity",
        from = "Column::KeyEncryptionKeyId",
        to = "super::key_encryption_key::Column::Id"
    )]
    KeyEncryptionKey,
    #[sea_orm(
        belongs_to = "super::merchant::Entity",
        from = "Column::MerchantId",
        to = "super::merchant::Column::Id"
    )]
    Merchant,
}

impl Related<super::key_encryption_key::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::KeyEncryptionKey.def()
    }
}

impl Related<super::merchant::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Merchant.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
