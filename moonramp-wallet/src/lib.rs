use serde::{Deserialize, Serialize};

#[cfg(feature = "bitcoin")]
use moonramp_core::bitcoin;
use moonramp_core::{anyhow, serde};
#[cfg(feature = "entity")]
use moonramp_entity::{currency, network, ticker, wallet};

#[cfg(feature = "bitcoin")]
mod bitcoin_wallet;

#[cfg(feature = "bitcoin")]
pub use bitcoin_wallet::*;

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(crate = "moonramp_core::serde")]
pub enum Ticker {
    BTC,
    BCH,
    ETH,
}

#[cfg(feature = "entity")]
impl From<Ticker> for ticker::Ticker {
    fn from(t: Ticker) -> ticker::Ticker {
        match t {
            Ticker::BTC => ticker::Ticker::BTC,
            Ticker::BTH => ticker::Ticker::BTH,
            Ticker::ETH => ticker::Ticker::ETH,
            Ticker::ETC => ticker::Ticker::ETC,
        }
    }
}

#[cfg(feature = "entity")]
impl From<&Ticker> for ticker::Ticker {
    fn from(t: &Ticker) -> ticker::Ticker {
        match t {
            Ticker::BTC => ticker::Ticker::BTC,
            Ticker::BTH => ticker::Ticker::BTH,
            Ticker::ETH => ticker::Ticker::ETH,
            Ticker::ETC => ticker::Ticker::ETC,
        }
    }
}

#[cfg(feature = "entity")]
impl From<ticker::Ticker> for Ticker {
    fn from(t: ticker::Ticker) -> Ticker {
        match t {
            ticker::Ticker::BTC => Ticker::BTC,
            ticker::Ticker::BTH => Ticker::BTH,
            ticker::Ticker::ETH => Ticker::ETH,
            ticker::Ticker::ETC => Ticker::ETC,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(crate = "moonramp_core::serde")]
pub enum Currency {
    BEAN,
    BTC,
    BTH,
    ETC,
    ETH,
    PAXG,
    USDC,
    USDP,
}

#[cfg(feature = "entity")]
impl From<Currency> for currency::Currency {
    fn from(c: Currency) -> currency::Currency {
        match c {
            Currency::BEAN => currency::Currency::BEAN,
            Currency::BTC => currency::Currency::BTC,
            Currency::BTH => currency::Currency::BTH,
            Currency::ETH => currency::Currency::ETH,
            Currency::ETC => currency::Currency::ETC,
            Currency::PAXG => currency::Currency::PAXG,
            Currency::USDC => currency::Currency::USDC,
            Currency::USDP => currency::Currency::USDP,
        }
    }
}

#[cfg(feature = "entity")]
impl From<&Currency> for currency::Currency {
    fn from(c: &Currency) -> currency::Currency {
        match c {
            Currency::BEAN => currency::Currency::BEAN,
            Currency::BTC => currency::Currency::BTC,
            Currency::BTH => currency::Currency::BTH,
            Currency::ETH => currency::Currency::ETH,
            Currency::ETC => currency::Currency::ETC,
            Currency::PAXG => currency::Currency::PAXG,
            Currency::USDC => currency::Currency::USDC,
            Currency::USDP => currency::Currency::USDP,
        }
    }
}

#[cfg(feature = "entity")]
impl From<currency::Currency> for Currency {
    fn from(c: currency::Currency) -> Currency {
        match c {
            currency::Currency::BEAN => Currency::BEAN,
            currency::Currency::BTC => Currency::BTC,
            currency::Currency::BTH => Currency::BTH,
            currency::Currency::ETH => Currency::ETH,
            currency::Currency::ETC => Currency::ETC,
            currency::Currency::PAXG => Currency::PAXG,
            currency::Currency::USDC => Currency::USDC,
            currency::Currency::USDP => Currency::USDP,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(crate = "moonramp_core::serde")]
pub enum Network {
    Mainnet,
    Testnet,
    Regtest,
}

#[cfg(feature = "entity")]
impl From<Network> for network::Network {
    fn from(n: Network) -> network::Network {
        match n {
            Network::Mainnet => network::Network::Mainnet,
            Network::Testnet => network::Network::Testnet,
            Network::Regtest => network::Network::Regtest,
        }
    }
}

#[cfg(feature = "entity")]
impl From<&Network> for network::Network {
    fn from(n: &Network) -> network::Network {
        match n {
            Network::Mainnet => network::Network::Mainnet,
            Network::Testnet => network::Network::Testnet,
            Network::Regtest => network::Network::Regtest,
        }
    }
}

#[cfg(feature = "entity")]
impl From<network::Network> for Network {
    fn from(n: network::Network) -> Network {
        match n {
            network::Network::Mainnet => Network::Mainnet,
            network::Network::Testnet => Network::Testnet,
            network::Network::Regtest => Network::Regtest,
        }
    }
}

#[cfg(feature = "bitcoin")]
impl From<Network> for bitcoin::Network {
    fn from(n: Network) -> bitcoin::Network {
        match n {
            Network::Mainnet => bitcoin::Network::Bitcoin,
            Network::Testnet => bitcoin::Network::Testnet,
            Network::Regtest => bitcoin::Network::Regtest,
        }
    }
}

#[cfg(feature = "bitcoin")]
impl From<&Network> for bitcoin::Network {
    fn from(n: &Network) -> bitcoin::Network {
        match n {
            Network::Mainnet => bitcoin::Network::Bitcoin,
            Network::Testnet => bitcoin::Network::Testnet,
            Network::Regtest => bitcoin::Network::Regtest,
        }
    }
}

#[cfg(feature = "bitcoin")]
impl From<&mut Network> for bitcoin::Network {
    fn from(n: &mut Network) -> bitcoin::Network {
        match n {
            Network::Mainnet => bitcoin::Network::Bitcoin,
            Network::Testnet => bitcoin::Network::Testnet,
            Network::Regtest => bitcoin::Network::Regtest,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(crate = "moonramp_core::serde")]
pub enum WalletType {
    Hot,
    Cold,
}

#[cfg(feature = "entity")]
impl From<WalletType> for wallet::WalletType {
    fn from(t: WalletType) -> wallet::WalletType {
        match t {
            WalletType::Hot => wallet::WalletType::Hot,
            WalletType::Cold => wallet::WalletType::Cold,
        }
    }
}

#[cfg(feature = "entity")]
impl From<&WalletType> for wallet::WalletType {
    fn from(t: &WalletType) -> wallet::WalletType {
        match t {
            WalletType::Hot => wallet::WalletType::Hot,
            WalletType::Cold => wallet::WalletType::Cold,
        }
    }
}

#[cfg(feature = "entity")]
impl From<wallet::WalletType> for WalletType {
    fn from(t: wallet::WalletType) -> WalletType {
        match t {
            wallet::WalletType::Hot => WalletType::Hot,
            wallet::WalletType::Cold => WalletType::Cold,
        }
    }
}

#[cfg(feature = "bitcoin")]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(crate = "moonramp_core::serde", rename_all = "UPPERCASE")]
pub enum BitcoinColdWalletType {
    XPubkey,
    //P2PKH,
    //P2SHWPKH,
    //P2WPKH,
}

#[derive(Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(crate = "moonramp_core::serde")]
pub enum Wallet {
    #[cfg(feature = "bitcoin")]
    Bitcoin(bitcoin_wallet::BitcoinWallet),
}

impl Wallet {
    pub fn pubkey(&self) -> String {
        match self {
            #[cfg(feature = "bitcoin")]
            Wallet::Bitcoin(w) => w.pubkey(),
        }
    }

    pub fn ticker(&self) -> Ticker {
        match self {
            #[cfg(feature = "bitcoin")]
            Wallet::Bitcoin(w) => w.ticker(),
        }
    }

    pub fn network(&self) -> Network {
        match self {
            #[cfg(feature = "bitcoin")]
            Wallet::Bitcoin(w) => w.network(),
        }
    }

    pub fn wallet_type(&self) -> WalletType {
        match self {
            #[cfg(feature = "bitcoin")]
            Wallet::Bitcoin(w) => w.wallet_type(),
        }
    }
}

#[cfg(feature = "bitcoin")]
impl Wallet {
    pub fn is_bitcoin(&self) -> bool {
        match self {
            Wallet::Bitcoin(_) => true,
            #[allow(unreachable_patterns)]
            _ => false,
        }
    }

    pub fn into_bitcoin(self) -> anyhow::Result<bitcoin_wallet::BitcoinWallet, Self> {
        match self {
            Wallet::Bitcoin(w) => Ok(w),
            #[allow(unreachable_patterns)]
            _ => Err(self),
        }
    }
}
