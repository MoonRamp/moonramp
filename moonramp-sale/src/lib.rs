use anyhow::anyhow;
use serde::{Deserialize, Serialize};

use moonramp_core::{anyhow, serde};
#[cfg(feature = "entity")]
use moonramp_entity::invoice;
use moonramp_wallet::Wallet;

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(crate = "moonramp_core::serde")]
pub enum InvoiceStatus {
    Canceled,
    Expired,
    Funded,
    Pending,
}

#[cfg(feature = "entity")]
impl From<InvoiceStatus> for invoice::InvoiceStatus {
    fn from(i: InvoiceStatus) -> invoice::InvoiceStatus {
        match i {
            InvoiceStatus::Pending => invoice::InvoiceStatus::Pending,
            InvoiceStatus::Canceled => invoice::InvoiceStatus::Canceled,
            InvoiceStatus::Funded => invoice::InvoiceStatus::Funded,
            InvoiceStatus::Expired => invoice::InvoiceStatus::Expired,
        }
    }
}

#[cfg(feature = "entity")]
impl From<&InvoiceStatus> for invoice::InvoiceStatus {
    fn from(i: &InvoiceStatus) -> invoice::InvoiceStatus {
        match i {
            InvoiceStatus::Pending => invoice::InvoiceStatus::Pending,
            InvoiceStatus::Canceled => invoice::InvoiceStatus::Canceled,
            InvoiceStatus::Funded => invoice::InvoiceStatus::Funded,
            InvoiceStatus::Expired => invoice::InvoiceStatus::Expired,
        }
    }
}

#[derive(Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(crate = "moonramp_core::serde")]
pub struct Invoice {
    pub wallet: Wallet,
    pub pubkey: String,
    pub address: String,
    pub uri: String,
    pub user_data: Option<Vec<u8>>,
}

impl TryFrom<lunar::ExitData> for Invoice {
    type Error = anyhow::Error;
    fn try_from(val: lunar::ExitData) -> anyhow::Result<Invoice> {
        match val {
            lunar::ExitData::Invoice {
                wallet,
                pubkey,
                address,
                uri,
                user_data,
            } => Ok(Invoice {
                wallet,
                pubkey,
                address,
                uri,
                user_data,
            }),
            _ => Err(anyhow!("ExitData is not sale")),
        }
    }
}

#[derive(Debug, PartialEq, Deserialize, Serialize)]
#[serde(crate = "moonramp_core::serde")]
pub struct Sale {
    pub funded: bool,
    pub amount: f64,
    pub user_data: Option<Vec<u8>>,
}

impl TryFrom<lunar::ExitData> for Sale {
    type Error = anyhow::Error;
    fn try_from(val: lunar::ExitData) -> anyhow::Result<Sale> {
        match val {
            lunar::ExitData::Sale {
                funded,
                amount,
                user_data,
            } => Ok(Sale {
                funded,
                amount,
                user_data,
            }),
            _ => Err(anyhow!("ExitData is not sale")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use moonramp_wallet::{BitcoinColdWalletType, BitcoinWallet, Network, Ticker};

    #[test]
    fn test_invoice_try_into() {
        let wallet = Wallet::Bitcoin(
            BitcoinWallet::new_cold(
                Ticker::BTC,
                Network::Testnet,
                "p2pkh_12345".to_string(),
                BitcoinColdWalletType::XPubkey,
            )
            .expect("Invalid Wallet"),
        );
        let exit_data = lunar::ExitData::Invoice {
            wallet: wallet,
            pubkey: "xpub12345".to_string(),
            address: "12345".to_string(),
            uri: "bitcoin:12345;version=1.0?amount=0.01".to_string(),
            user_data: None,
        };
        assert_eq!(
            exit_data.try_into().ok(),
            Some(Invoice {
                wallet: Wallet::Bitcoin(
                    BitcoinWallet::new_cold(
                        Ticker::BTC,
                        Network::Testnet,
                        "p2pkh_12345".to_string(),
                        BitcoinColdWalletType::XPubkey
                    )
                    .expect("Invalid Wallet"),
                ),
                pubkey: "xpub12345".to_string(),
                address: "12345".to_string(),
                uri: "bitcoin:12345;version=1.0?amount=0.01".to_string(),
                user_data: None,
            })
        );
    }

    #[test]
    fn test_sale_try_into() {
        let exit_data = lunar::ExitData::Sale {
            funded: true,
            amount: 0.00001000,
            user_data: None,
        };
        assert_eq!(
            exit_data.try_into().ok(),
            Some(Sale {
                funded: true,
                amount: 0.00001000,
                user_data: None,
            })
        );
    }
}
