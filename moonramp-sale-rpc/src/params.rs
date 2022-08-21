use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use moonramp_core::{chrono, serde, Hash};
use moonramp_entity::{invoice, sale};
use moonramp_wallet::{Currency, Network, Ticker};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(crate = "moonramp_core::serde", rename_all = "camelCase")]
pub struct SaleInvoiceRequest {
    pub hash: Hash,
    pub uuid: String,
    pub currency: Currency,
    pub amount: f64,
    pub expires_in: Option<i64>,
    pub user_data: Option<Vec<u8>>,
    pub program: Option<Hash>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(crate = "moonramp_core::serde", rename_all = "camelCase", untagged)]
pub enum SaleInvoiceLookupRequest {
    Hash { hash: Hash },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(crate = "moonramp_core::serde", rename_all = "camelCase")]
pub struct SaleInvoiceResponse {
    pub hash: Hash,
    pub wallet_hash: Hash,
    pub ticker: Ticker,
    pub currency: Currency,
    pub network: Network,
    pub invoice_status: invoice::InvoiceStatus,
    pub pubkey: String,
    pub address: String,
    pub amount: f64,
    pub uri: String,
    pub user_data: Option<Vec<u8>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

impl SaleInvoiceResponse {
    pub fn with_user_data(mut self, user_data: Option<Vec<u8>>) -> SaleInvoiceResponse {
        self.user_data = user_data;
        self
    }
}

impl From<invoice::Model> for SaleInvoiceResponse {
    fn from(model: invoice::Model) -> SaleInvoiceResponse {
        SaleInvoiceResponse {
            hash: model.hash,
            wallet_hash: model.wallet_hash,
            ticker: model.ticker.into(),
            currency: model.currency.into(),
            network: model.network.into(),
            invoice_status: model.invoice_status,
            pubkey: model.pubkey,
            address: model.address,
            amount: model.amount,
            uri: model.uri,
            user_data: None,
            created_at: model.created_at,
            updated_at: model.updated_at,
            expires_at: model.expires_at,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(crate = "moonramp_core::serde", rename_all = "camelCase")]
pub struct SaleCaptureRequest {
    pub hash: Hash,
    pub uuid: String,
    pub confirmations: Option<i64>,
    pub user_data: Option<Vec<u8>>,
    pub program: Option<Hash>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(crate = "moonramp_core::serde", rename_all = "camelCase")]
pub struct SaleCaptureAsyncRequest {
    pub hash: Hash,
    pub uuid: String,
    pub webhook_id: String,
    pub confirmations: Option<i64>,
    pub user_data: Option<Vec<u8>>,
    pub program: Option<Hash>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(crate = "moonramp_core::serde", rename_all = "camelCase")]
pub struct SaleCheckoutRequest {
    pub hash: Hash,
    pub uuid: String,
    pub webhook_id: String,
    pub confirmations: Option<i64>,
    pub cancel_url: String,
    pub success_url: String,
    pub user_data: Option<Vec<u8>>,
    pub program: Option<Hash>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(crate = "moonramp_core::serde", rename_all = "camelCase", untagged)]
pub enum SaleLookupRequest {
    Hash { hash: Hash },
    InvoiceHash { invoice_hash: Hash },
}

//#[derive(Clone, Debug, Deserialize, Serialize)]
//#[serde(crate = "moonramp_core::serde", rename_all = "camelCase")]
//pub struct SaleCheckoutResponse {
//    pub hash: String,
//    pub wallet_hash: String,
//    pub invoice_hash: String,
//    pub ticker: ticker::Ticker,
//    pub currency: currency::Currency,
//    pub pubkey: String,
//    pub address: String,
//    pub amount: String,
//    pub payment_url: String,
//    pub user_data: Option<Vec<u8>>,
//    pub created_at: DateTime<Utc>,
//}
//
//impl SaleCheckoutResponse {
//    pub fn with_user_data(mut self, user_data: Option<Vec<u8>>) -> SaleCheckoutResponse {
//        self.user_data = user_data;
//        self
//    }
//}
//
//impl From<sale::Model> for SaleCheckoutResponse {
//    fn from(model: sale::Model) -> SaleCheckoutResponse {
//        SaleCheckoutResponse {
//            hash: model.hash.to_string(),
//            wallet_hash: model.wallet_hash.to_string(),
//            invoice_hash: model.invoice_hash.to_string(),
//            ticker: model.ticker,
//            currency: model.currency,
//            pubkey: model.pubkey,
//            address: model.address,
//            amount: model.amount,
//            user_data: None,
//            created_at: model.created_at,
//        }
//    }
//}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(crate = "moonramp_core::serde", rename_all = "camelCase")]
pub struct SaleResponse {
    pub hash: Hash,
    pub wallet_hash: Hash,
    pub invoice_hash: Hash,
    pub ticker: Ticker,
    pub currency: Currency,
    pub network: Network,
    pub pubkey: String,
    pub address: String,
    pub amount: f64,
    pub confirmations: i64,
    pub user_data: Option<Vec<u8>>,
    pub created_at: DateTime<Utc>,
}

impl SaleResponse {
    pub fn with_user_data(mut self, user_data: Option<Vec<u8>>) -> SaleResponse {
        self.user_data = user_data;
        self
    }
}

impl From<sale::Model> for SaleResponse {
    fn from(model: sale::Model) -> SaleResponse {
        SaleResponse {
            hash: model.hash,
            wallet_hash: model.wallet_hash,
            invoice_hash: model.invoice_hash,
            ticker: model.ticker.into(),
            currency: model.currency.into(),
            network: model.network.into(),
            pubkey: model.pubkey,
            address: model.address,
            amount: model.amount,
            confirmations: model.confirmations,
            user_data: None,
            created_at: model.created_at,
        }
    }
}
