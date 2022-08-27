use std::{fmt, str::FromStr};

use curve25519_dalek::scalar::Scalar;
use monero::{
    cryptonote,
    util::{
        address::Address,
        key::{PrivateKey, PublicKey},
    },
};
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};

use moonramp_core::{anyhow, bs58, curve25519_dalek, monero, rand, serde};

use crate::{Network, Ticker, WalletType};

#[derive(Eq, PartialEq, Deserialize, Serialize)]
#[serde(crate = "moonramp_core::serde")]
pub struct MoneroHotWallet {
    //pub mnemonic: Vec<u8>,
    //pub password: String,
    pub spend_key: [u8; 32],
    pub view_key: [u8; 32],
}

impl fmt::Debug for MoneroHotWallet {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match PrivateKey::from_slice(&self.view_key) {
            Ok(view_key) => {
                let view_pub_key = PublicKey::from_private_key(&view_key);
                write!(f, "{}", bs58::encode(view_pub_key.as_bytes()).into_string())
            }
            Err(err) => write!(f, "Invalid MoneroHotWallet({})", err),
        }
    }
}

impl fmt::Display for MoneroHotWallet {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl From<PrivateKey> for MoneroHotWallet {
    fn from(spend_key: PrivateKey) -> MoneroHotWallet {
        let spend_key_hash = cryptonote::hash::Hash::new(&spend_key.to_bytes()).to_bytes();
        let view_scalar = Scalar::from_bytes_mod_order(spend_key_hash);
        let view_key = PrivateKey::from_scalar(view_scalar);

        MoneroHotWallet {
            spend_key: spend_key.to_bytes(),
            view_key: view_key.to_bytes(),
        }
    }
}

impl FromStr for MoneroHotWallet {
    type Err = anyhow::Error;
    fn from_str(val: &str) -> anyhow::Result<MoneroHotWallet> {
        let spend_key = PrivateKey::from_str(val)?;
        Ok(MoneroHotWallet::from(spend_key))
    }
}

#[derive(Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(crate = "moonramp_core::serde")]
pub enum MoneroColdWallet {
    ViewKey {
        address: String,
        spend_pub_key: Vec<u8>,
        view_key: Vec<u8>,
    },
}

#[derive(Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(crate = "moonramp_core::serde")]
pub enum MoneroWallet {
    Hot(Network, MoneroHotWallet),
    Cold(Network, MoneroColdWallet),
}

impl MoneroWallet {
    pub fn new_hot(network: Network) -> anyhow::Result<MoneroWallet> {
        let mut entropy = [0u8; 64];
        thread_rng().fill(&mut entropy);
        let spend_scalar = Scalar::from_bytes_mod_order_wide(&entropy);
        let spend_key = PrivateKey::from_scalar(spend_scalar);

        Ok(MoneroWallet::Hot(network, MoneroHotWallet::from(spend_key)))
    }

    pub fn pubkey(&self) -> String {
        match self {
            MoneroWallet::Hot(_, w) => {
                let view_key = PrivateKey::from_slice(&w.view_key).expect("Invalid ViewKey");
                PublicKey::from_private_key(&view_key).to_string()
            }
            MoneroWallet::Cold(_, w) => match w {
                MoneroColdWallet::ViewKey { view_key, .. } => {
                    let view_key = PrivateKey::from_slice(&view_key).expect("Invalid ViewKey");
                    PublicKey::from_private_key(&view_key)
                }
            }
            .to_string(),
        }
    }

    pub fn ticker(&self) -> Ticker {
        Ticker::XMR
    }

    pub fn network(&self) -> Network {
        match self {
            MoneroWallet::Hot(network, _) => network,
            MoneroWallet::Cold(network, _) => network,
        }
        .clone()
    }

    pub fn wallet_type(&self) -> WalletType {
        match self {
            MoneroWallet::Hot(_, _) => WalletType::Hot,
            MoneroWallet::Cold(_, _) => WalletType::Cold,
        }
    }
}

impl MoneroWallet {
    pub fn addr(&self) -> String {
        match self {
            MoneroWallet::Hot(network, w) => {
                let spend_key = PrivateKey::from_slice(&w.spend_key).expect("Invalid spend_key");
                let view_key = PrivateKey::from_slice(&w.view_key).expect("Invalid view_key");
                let spend_pub_key = PublicKey::from_private_key(&spend_key);
                let view_pub_key = PublicKey::from_private_key(&view_key);
                Address::standard(network.clone().into(), spend_pub_key, view_pub_key).to_string()
            }
            MoneroWallet::Cold(_, w) => match w {
                MoneroColdWallet::ViewKey { address, .. } => address,
            }
            .to_string(),
        }
    }
}

#[test]
fn test_hot_wallet() {
    let spend_key = "aa7c977f3f03ba300bd530f12839437b8fd0f95c10ea6128fb60e31ba0bd8409";
    let w = MoneroHotWallet::from_str(spend_key).expect("Invalid MoneroHotWallet");
    assert_eq!(w.to_string(), "fqx7WSrjtN6FwhMJyzHiiQ2YM8de8e4rLXV8PBHXdPX");
    let expected_spend_key =
        PrivateKey::from_str("aa7c977f3f03ba300bd530f12839437b8fd0f95c10ea6128fb60e31ba0bd8409")
            .expect("Invalid SpendKey");
    let expected_view_key =
        PrivateKey::from_str("e065c2bb784345ff500807bea4eda8a9512d974f3e7695d120d73c54045f6704")
            .expect("Invalid ViewKey");
    assert_eq!(w.spend_key, expected_spend_key.to_bytes());
    assert_eq!(w.view_key, expected_view_key.to_bytes());

    let mainnet_w = MoneroWallet::Hot(Network::Mainnet, w);
    assert_eq!(mainnet_w.addr(), "436cmaKNNvJQMBqWf3EaUCKJ7sJW4RkhJR1paum66s2ac8e8TrGYLwpiFsG66UAWkARupVhNkuiTweNehKn1x4Wa3zo7oFr");
}
