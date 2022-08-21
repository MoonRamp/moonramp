use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

use moonramp_core::{chrono, sea_orm, serde, Hash};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Deserialize, Serialize)]
#[sea_orm(table_name = "key_encryption_keys")]
#[serde(crate = "moonramp_core::serde")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false, column_type = "Text")]
    pub hash: Hash,
    #[sea_orm(indexed, column_type = "Text")]
    pub master_key_encryption_key_hash: Hash,
    pub cipher: super::cipher::Cipher,
    pub key: Vec<u8>,
    pub nonce: Vec<u8>,
    pub created_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::encryption_key::Entity", on_delete = "Cascade")]
    EncryptionKey,
}

impl Related<super::encryption_key::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::EncryptionKey.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
