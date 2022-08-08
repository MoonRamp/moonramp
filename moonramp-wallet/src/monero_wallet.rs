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
pub struct MoneroHotWallet {
    pub mnemonic: Vec<u8>,
    pub password: String,
    pub address: Vec<u8>,
}

impl fmt::Debug for MoneroHotWallet {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", String::from_utf8_lossy(self.address))
    }
}

#[derive(Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(crate = "moonramp_core::serde")]
pub enum MoneroColdWallet {
    ViewKey { key: Vec<u8> },
}

#[derive(Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(crate = "moonramp_core::serde")]
pub enum MoneroWallet {
    Hot(Ticker, Network, MoneroHotWallet),
    Cold(Ticker, Network, MoneroColdWallet),
}
