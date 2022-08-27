use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use moonramp_core::{chrono, serde, Hash};
use moonramp_entity::wallet;

use crate::{BitcoinColdWalletType, MoneroColdWalletType, Network, Ticker, WalletType};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(crate = "moonramp_core::serde", rename_all = "camelCase")]
pub enum WalletCreateRequest {
    BtcHot,
    #[serde(rename_all = "camelCase")]
    BtcCold {
        pubkey: String,
        cold_type: BitcoinColdWalletType,
    },
    BchHot,
    #[serde(rename_all = "camelCase")]
    BchCold {
        pubkey: String,
        cold_type: BitcoinColdWalletType,
    },
    //EthereumHot(ticker::Ticker),
    //EthereumCold {
    //    ticker: ticker::Ticker,
    //    pubkey: String,
    //},
    XmrHot,
    #[serde(rename_all = "camelCase")]
    XmrCold {
        view_key: String,
        cold_type: MoneroColdWalletType,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(crate = "moonramp_core::serde", rename_all = "camelCase", untagged)]
pub enum WalletLookupRequest {
    Hash { hash: Hash },
    Pubkey { pubkey: String },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(crate = "moonramp_core::serde", rename_all = "camelCase")]
pub struct WalletResponse {
    pub hash: Hash,
    pub ticker: Ticker,
    pub network: Network,
    pub wallet_type: WalletType,
    pub pubkey: String,
    pub created_at: DateTime<Utc>,
}

impl From<wallet::Model> for WalletResponse {
    fn from(model: wallet::Model) -> WalletResponse {
        WalletResponse {
            hash: model.hash,
            ticker: model.ticker.into(),
            network: model.network.into(),
            wallet_type: model.wallet_type.into(),
            pubkey: model.pubkey,
            created_at: model.created_at,
        }
    }
}
