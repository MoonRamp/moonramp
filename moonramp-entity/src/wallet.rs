use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

use moonramp_core::{chrono, sea_orm, serde, Hash};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Deserialize, Serialize)]
#[sea_orm(table_name = "wallets")]
#[serde(crate = "moonramp_core::serde")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false, column_type = "Text")]
    pub hash: Hash,
    #[sea_orm(indexed, column_type = "Text")]
    pub merchant_hash: Hash,
    #[sea_orm(unique, indexed, column_type = "Text")]
    pub pubkey: String,
    pub ticker: super::ticker::Ticker,
    pub network: super::network::Network,
    pub wallet_type: WalletType,
    pub cipher: super::cipher::Cipher,
    #[sea_orm(indexed, column_type = "Text")]
    pub encryption_key_hash: Hash,
    pub blob: Vec<u8>,
    pub nonce: Vec<u8>,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, PartialEq, EnumIter, DeriveActiveEnum, Deserialize, Serialize)]
#[sea_orm(rs_type = "String", db_type = "Text", enum_name = "wallet_type")]
#[serde(crate = "moonramp_core::serde")]
pub enum WalletType {
    #[sea_orm(string_value = "Hot")]
    Hot,
    #[sea_orm(string_value = "Cold")]
    Cold,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::encryption_key::Entity",
        from = "Column::EncryptionKeyHash",
        to = "super::encryption_key::Column::Hash"
    )]
    EncryptionKey,
    #[sea_orm(
        belongs_to = "super::merchant::Entity",
        from = "Column::MerchantHash",
        to = "super::merchant::Column::Hash"
    )]
    Merchant,
}

impl Related<super::encryption_key::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::EncryptionKey.def()
    }
}

impl Related<super::merchant::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Merchant.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
