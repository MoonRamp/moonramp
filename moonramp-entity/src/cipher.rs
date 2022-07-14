use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

use moonramp_core::{sea_orm, serde};

#[derive(Clone, Debug, PartialEq, EnumIter, DeriveActiveEnum, Deserialize, Serialize)]
#[sea_orm(rs_type = "String", db_type = "Text", enum_name = "cipher")]
#[serde(crate = "moonramp_core::serde")]
pub enum Cipher {
    #[sea_orm(string_value = "aes256-gcm-siv")]
    Aes256GcmSiv,
    #[sea_orm(string_value = "chacha20-poly1305")]
    ChaCha20Poly1305,
    #[cfg(feature = "testing")]
    #[sea_orm(string_value = "noop")]
    Noop,
}
