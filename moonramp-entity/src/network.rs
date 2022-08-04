use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

use moonramp_core::{sea_orm, serde};

#[derive(Clone, Debug, PartialEq, EnumIter, DeriveActiveEnum, Deserialize, Serialize)]
#[sea_orm(rs_type = "String", db_type = "Text", enum_name = "network")]
#[serde(crate = "moonramp_core::serde")]
pub enum Network {
    #[sea_orm(string_value = "Mainnet")]
    Mainnet,
    #[sea_orm(string_value = "Testnet")]
    Testnet,
    #[sea_orm(string_value = "Regtest")]
    Regtest,
}
