use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

use moonramp_core::{sea_orm, serde};

#[derive(Clone, Debug, PartialEq, EnumIter, DeriveActiveEnum, Deserialize, Serialize)]
#[sea_orm(rs_type = "String", db_type = "Text", enum_name = "ticker")]
#[serde(crate = "moonramp_core::serde")]
pub enum Ticker {
    #[sea_orm(string_value = "Bitcoin")]
    BTC,
    #[sea_orm(string_value = "Bitcoin Cash")]
    BCH,
    #[sea_orm(string_value = "Ethereum")]
    ETH,
}
