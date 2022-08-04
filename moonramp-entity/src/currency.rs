use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

use moonramp_core::{sea_orm, serde};

#[derive(Clone, Debug, PartialEq, EnumIter, DeriveActiveEnum, Deserialize, Serialize)]
#[sea_orm(rs_type = "String", db_type = "Text", enum_name = "currency")]
#[serde(crate = "moonramp_core::serde")]
pub enum Currency {
    #[sea_orm(string_value = "Beanstalk")]
    BEAN,
    #[sea_orm(string_value = "Bitcoin")]
    BTC,
    #[sea_orm(string_value = "Bitcoin Cash")]
    BTH,
    #[sea_orm(string_value = "Ethereum Classic")]
    ETC,
    #[sea_orm(string_value = "Ethereum")]
    ETH,
    #[sea_orm(string_value = "Pax Gold")]
    PAXG,
    #[sea_orm(string_value = "USD Coin")]
    USDC,
    #[sea_orm(string_value = "Pax Dollar")]
    USDP,
}
