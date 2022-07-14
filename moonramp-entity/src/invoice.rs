use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

use moonramp_core::{chrono, sea_orm, serde, Hash};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Deserialize, Serialize)]
#[sea_orm(table_name = "invoices")]
#[serde(crate = "moonramp_core::serde")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false, column_type = "Text")]
    pub hash: Hash,
    #[sea_orm(indexed, column_type = "Text")]
    pub merchant_id: String,
    #[sea_orm(indexed, column_type = "Text")]
    pub wallet_hash: Hash,
    pub ticker: super::ticker::Ticker,
    pub currency: super::currency::Currency,
    pub network: super::network::Network,
    pub invoice_status: InvoiceStatus,
    pub pubkey: String,
    pub address: String,
    pub amount: i64,
    pub uri: String,
    pub cipher: super::cipher::Cipher,
    #[sea_orm(indexed, column_type = "Text")]
    pub encryption_key_id: Hash,
    pub blob: Vec<u8>,
    pub nonce: Vec<u8>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Clone, Debug, PartialEq, EnumIter, DeriveActiveEnum, Deserialize, Serialize)]
#[sea_orm(rs_type = "String", db_type = "Text", enum_name = "invoice_status")]
#[serde(crate = "moonramp_core::serde")]
pub enum InvoiceStatus {
    #[sea_orm(string_value = "Canceled")]
    Canceled,
    #[sea_orm(string_value = "Expired")]
    Expired,
    #[sea_orm(string_value = "Funded")]
    Funded,
    #[sea_orm(string_value = "Pending")]
    Pending,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::encryption_key::Entity",
        from = "Column::EncryptionKeyId",
        to = "super::encryption_key::Column::Id"
    )]
    EncryptionKey,
    #[sea_orm(
        belongs_to = "super::merchant::Entity",
        from = "Column::MerchantId",
        to = "super::merchant::Column::Id"
    )]
    Merchant,
    #[sea_orm(has_many = "super::sale::Entity", on_delete = "Cascade")]
    Sale,
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

impl Related<super::sale::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Sale.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
