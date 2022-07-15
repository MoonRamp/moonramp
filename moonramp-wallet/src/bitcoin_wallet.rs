use std::{fmt, str::FromStr};

use anyhow::anyhow;
use bip39::Mnemonic;
use bitcoin::{
    secp256k1::Secp256k1,
    util::{
        address::Address,
        bip32::{DerivationPath, ExtendedPrivKey, ExtendedPubKey},
    },
};
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use serde::{Deserialize, Serialize};

use moonramp_core::{anyhow, bip39, bitcoin, rand, serde};

use crate::{BitcoinColdWalletType, Network, Ticker, WalletType};

#[derive(Eq, PartialEq, Deserialize, Serialize)]
#[serde(crate = "moonramp_core::serde")]
pub struct BitcoinHotWallet {
    pub mnemonic: Vec<u8>,
    pub password: String,
    pub xpub: Vec<u8>,
    pub index: u64,
}

impl fmt::Debug for BitcoinHotWallet {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match ExtendedPubKey::decode(&self.xpub) {
            Ok(xpub) => write!(f, "{}", xpub),
            Err(err) => write!(f, "Invalid BitcoinHotWallet({})", err),
        }
    }
}

#[derive(Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(crate = "moonramp_core::serde")]
pub enum BitcoinColdWallet {
    XPubkey { xpub: String, index: u64 },
}

#[derive(Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(crate = "moonramp_core::serde")]
pub enum BitcoinWallet {
    Hot(Ticker, Network, BitcoinHotWallet),
    Cold(Ticker, Network, BitcoinColdWallet),
}

impl BitcoinWallet {
    pub fn new_hot(ticker: Ticker, network: Network) -> anyhow::Result<BitcoinWallet> {
        let password: String = thread_rng()
            .sample_iter(&Alphanumeric)
            .take(32)
            .map(char::from)
            .collect();

        let mut entropy = [0u8; 32];
        thread_rng().fill(&mut entropy);

        let mnemonic = Mnemonic::from_entropy(&entropy)?;
        let seed = mnemonic.to_seed(password.clone());
        let key = ExtendedPrivKey::new_master(network.clone().into(), &seed)?;

        let secp = Secp256k1::new();
        Ok(BitcoinWallet::Hot(
            ticker,
            network,
            BitcoinHotWallet {
                mnemonic: mnemonic.to_entropy(),
                password,
                xpub: ExtendedPubKey::from_priv(&secp, &key).encode().to_vec(),
                index: 0,
            },
        ))
    }

    pub fn new_cold(
        ticker: Ticker,
        network: Network,
        pubkey: String,
        cold_type: BitcoinColdWalletType,
    ) -> anyhow::Result<BitcoinWallet> {
        match (ticker, cold_type) {
            (Ticker::BTC, BitcoinColdWalletType::XPubkey) => Ok(BitcoinWallet::Cold(
                Ticker::BTC,
                network,
                BitcoinColdWallet::XPubkey {
                    xpub: pubkey,
                    index: 0,
                },
            )),
            (Ticker::BCH, BitcoinColdWalletType::XPubkey) => Ok(BitcoinWallet::Cold(
                Ticker::BCH,
                network,
                BitcoinColdWallet::XPubkey {
                    xpub: pubkey,
                    index: 0,
                },
            )),
            _ => Err(anyhow!("Ticker not supported")),
        }
    }

    pub fn pubkey(&self) -> String {
        match self {
            BitcoinWallet::Hot(_, _, w) => match ExtendedPubKey::decode(&w.xpub) {
                Ok(xpub) => xpub.to_string(),
                Err(_) => "Invalid BitcoinHotWallet".to_string(),
            },
            BitcoinWallet::Cold(_, _, w) => match w {
                BitcoinColdWallet::XPubkey { xpub, .. } => xpub,
            }
            .to_string(),
        }
    }

    pub fn ticker(&self) -> Ticker {
        match self {
            BitcoinWallet::Hot(ticker, _, _) => ticker,
            BitcoinWallet::Cold(ticker, _, _) => ticker,
        }
        .clone()
    }

    pub fn network(&self) -> Network {
        match self {
            BitcoinWallet::Hot(_, network, _) => network,
            BitcoinWallet::Cold(_, network, _) => network,
        }
        .clone()
    }

    pub fn wallet_type(&self) -> WalletType {
        match self {
            BitcoinWallet::Hot(_, _, _) => WalletType::Hot,
            BitcoinWallet::Cold(_, _, _) => WalletType::Cold,
        }
    }
}

impl BitcoinWallet {
    pub fn next_xpub(&mut self) -> anyhow::Result<ExtendedPubKey> {
        match self {
            BitcoinWallet::Hot(ticker, _, w) => match ticker {
                Ticker::BTC | Ticker::BCH => {
                    let xpub = ExtendedPubKey::decode(&w.xpub)?;
                    let derivation_path = format!("m/1/0/{}", w.index);
                    let secp = Secp256k1::new();
                    let chxpub =
                        xpub.derive_pub(&secp, &DerivationPath::from_str(&derivation_path)?)?;
                    w.index += 1;
                    Ok(chxpub)
                }
                _ => Err(anyhow!("Ticker {:?} not supported", ticker)),
            },
            BitcoinWallet::Cold(ticker, _, w) => match ticker {
                Ticker::BTC | Ticker::BCH => {
                    let BitcoinColdWallet::XPubkey { xpub, index } = w;
                    let xpub = ExtendedPubKey::from_str(xpub)?;
                    let derivation_path = format!("m/1/0/{}", index);
                    let secp = Secp256k1::new();
                    let chxpub =
                        xpub.derive_pub(&secp, &DerivationPath::from_str(&derivation_path)?)?;
                    *index += 1;
                    Ok(chxpub)
                }
                _ => Err(anyhow!("Ticker {:?} not supported", ticker)),
            },
        }
    }

    pub fn next_p2pkh_addr(&mut self) -> anyhow::Result<(ExtendedPubKey, String)> {
        match self {
            BitcoinWallet::Hot(ticker, network, _) => match ticker {
                Ticker::BTC | Ticker::BCH => {
                    let network = network.into();
                    let next_xpub = self.next_xpub()?;
                    Ok((
                        next_xpub,
                        Address::p2pkh(&next_xpub.to_pub(), network).to_string(),
                    ))
                }
                _ => Err(anyhow!("Ticker {:?} not supported", ticker)),
            },
            BitcoinWallet::Cold(ticker, network, _) => match ticker {
                Ticker::BTC | Ticker::BCH => {
                    let network = network.into();
                    let next_xpub = self.next_xpub()?;
                    Ok((
                        next_xpub,
                        Address::p2pkh(&next_xpub.to_pub(), network).to_string(),
                    ))
                }
                _ => Err(anyhow!("Ticker {:?} not supported", ticker)),
            },
        }
    }

    pub fn next_p2wpkh_addr(&mut self) -> anyhow::Result<(ExtendedPubKey, String)> {
        match self {
            BitcoinWallet::Hot(ticker, network, _) => match ticker {
                Ticker::BTC => {
                    let network = network.into();
                    let next_xpub = self.next_xpub()?;
                    Ok((
                        next_xpub,
                        Address::p2wpkh(&next_xpub.to_pub(), network)?.to_string(),
                    ))
                }
                _ => Err(anyhow!("p2wpkh not supported for BCH")),
            },
            BitcoinWallet::Cold(ticker, network, _) => match ticker {
                Ticker::BTC => {
                    let network = network.into();
                    let next_xpub = self.next_xpub()?;
                    Ok((
                        next_xpub,
                        Address::p2wpkh(&next_xpub.to_pub(), network)?.to_string(),
                    ))
                }
                _ => Err(anyhow!("p2wpkh not supported for BCH")),
            },
        }
    }

    pub fn next_addr(&mut self) -> anyhow::Result<(ExtendedPubKey, String)> {
        match self {
            BitcoinWallet::Hot(ticker, _, _) => match ticker {
                Ticker::BTC => self.next_p2wpkh_addr(),
                Ticker::BCH => self.next_p2pkh_addr(),
                _ => Err(anyhow!("Ticker {:?} not supported", ticker)),
            },
            BitcoinWallet::Cold(ticker, _, _) => match ticker {
                Ticker::BTC => self.next_p2wpkh_addr(),
                Ticker::BCH => self.next_p2pkh_addr(),
                _ => Err(anyhow!("Ticker {:?} not supported", ticker)),
            },
        }
    }
}
