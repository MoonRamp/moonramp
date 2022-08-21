use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

use moonramp_core::{chrono, sea_orm, serde, Hash};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Deserialize, Serialize)]
#[sea_orm(table_name = "roles")]
#[serde(crate = "moonramp_core::serde")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false, column_type = "Text")]
    pub hash: Hash,
    #[sea_orm(indexed, column_type = "Text")]
    pub merchant_hash: Hash,
    pub token_hash: Hash,
    pub resource: Resource,
    pub scope: Scope,
    pub api_group: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, PartialEq, EnumIter, DeriveActiveEnum, Deserialize, Serialize)]
#[sea_orm(rs_type = "String", db_type = "Text", enum_name = "scope")]
#[serde(crate = "moonramp_core::serde")]
pub enum Scope {
    #[sea_orm(string_value = "Execute")]
    Execute,
    #[sea_orm(string_value = "Read")]
    Read,
    #[sea_orm(string_value = "Watch")]
    Watch,
    #[sea_orm(string_value = "Write")]
    Write,
}

#[derive(Clone, Debug, PartialEq, EnumIter, DeriveActiveEnum, Deserialize, Serialize)]
#[sea_orm(rs_type = "String", db_type = "Text", enum_name = "resource")]
#[serde(crate = "moonramp_core::serde")]
pub enum Resource {
    #[sea_orm(string_value = "ApiToken")]
    ApiToken,
    #[sea_orm(string_value = "Merchant")]
    Merchant,
    #[sea_orm(string_value = "Program")]
    Program,
    #[sea_orm(string_value = "Sale")]
    Sale,
    #[sea_orm(string_value = "Wallet")]
    Wallet,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(
        belongs_to = "super::api_token::Entity",
        from = "Column::TokenHash",
        to = "super::api_token::Column::Hash"
    )]
    ApiToken,
    #[sea_orm(
        belongs_to = "super::merchant::Entity",
        from = "Column::MerchantHash",
        to = "super::merchant::Column::Hash"
    )]
    Merchant,
}

impl Related<super::api_token::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ApiToken.def()
    }
}

impl Related<super::merchant::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Merchant.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
