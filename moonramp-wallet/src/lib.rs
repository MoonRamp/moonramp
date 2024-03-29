use serde::{Deserialize, Serialize};

#[cfg(feature = "bitcoin")]
use moonramp_core::bitcoin;
#[cfg(feature = "monero")]
use moonramp_core::monero;
use moonramp_core::{anyhow, serde};
#[cfg(feature = "entity")]
use moonramp_entity::{currency, network, ticker, wallet};

#[cfg(feature = "bitcoin")]
mod bitcoin_wallet;
#[cfg(feature = "bitcoin")]
pub use bitcoin_wallet::*;

#[cfg(feature = "monero")]
mod monero_wallet;
#[cfg(feature = "monero")]
pub use monero_wallet::*;

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(crate = "moonramp_core::serde")]
pub enum Ticker {
    BCH,
    BTC,
    ETC,
    ETH,
    XMR,
}

#[cfg(feature = "entity")]
impl From<Ticker> for ticker::Ticker {
    fn from(t: Ticker) -> ticker::Ticker {
        match t {
            Ticker::BCH => ticker::Ticker::BCH,
            Ticker::BTC => ticker::Ticker::BTC,
            Ticker::ETC => ticker::Ticker::ETC,
            Ticker::ETH => ticker::Ticker::ETH,
            Ticker::XMR => ticker::Ticker::XMR,
        }
    }
}

#[cfg(feature = "entity")]
impl From<&Ticker> for ticker::Ticker {
    fn from(t: &Ticker) -> ticker::Ticker {
        match t {
            Ticker::BCH => ticker::Ticker::BCH,
            Ticker::BTC => ticker::Ticker::BTC,
            Ticker::ETC => ticker::Ticker::ETC,
            Ticker::ETH => ticker::Ticker::ETH,
            Ticker::XMR => ticker::Ticker::XMR,
        }
    }
}

#[cfg(feature = "entity")]
impl From<ticker::Ticker> for Ticker {
    fn from(t: ticker::Ticker) -> Ticker {
        match t {
            ticker::Ticker::BCH => Ticker::BCH,
            ticker::Ticker::BTC => Ticker::BTC,
            ticker::Ticker::ETC => Ticker::ETC,
            ticker::Ticker::ETH => Ticker::ETH,
            ticker::Ticker::XMR => Ticker::XMR,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(crate = "moonramp_core::serde")]
pub enum Currency {
    BCH,
    BTC,
    ETC,
    ETH,
    USDC,
    USDP,
    USDT,
    XMR,
}

#[cfg(feature = "entity")]
impl From<Currency> for currency::Currency {
    fn from(c: Currency) -> currency::Currency {
        match c {
            Currency::BCH => currency::Currency::BCH,
            Currency::BTC => currency::Currency::BTC,
            Currency::ETC => currency::Currency::ETC,
            Currency::ETH => currency::Currency::ETH,
            Currency::USDC => currency::Currency::USDC,
            Currency::USDP => currency::Currency::USDP,
            Currency::USDT => currency::Currency::USDT,
            Currency::XMR => currency::Currency::XMR,
        }
    }
}

#[cfg(feature = "entity")]
impl From<&Currency> for currency::Currency {
    fn from(c: &Currency) -> currency::Currency {
        match c {
            Currency::BCH => currency::Currency::BCH,
            Currency::BTC => currency::Currency::BTC,
            Currency::ETC => currency::Currency::ETC,
            Currency::ETH => currency::Currency::ETH,
            Currency::USDC => currency::Currency::USDC,
            Currency::USDP => currency::Currency::USDP,
            Currency::USDT => currency::Currency::USDT,
            Currency::XMR => currency::Currency::XMR,
        }
    }
}

#[cfg(feature = "entity")]
impl From<currency::Currency> for Currency {
    fn from(c: currency::Currency) -> Currency {
        match c {
            currency::Currency::BCH => Currency::BCH,
            currency::Currency::BTC => Currency::BTC,
            currency::Currency::ETC => Currency::ETC,
            currency::Currency::ETH => Currency::ETH,
            currency::Currency::USDC => Currency::USDC,
            currency::Currency::USDP => Currency::USDP,
            currency::Currency::USDT => Currency::USDT,
            currency::Currency::XMR => Currency::XMR,
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

#[cfg(feature = "monero")]
impl From<Network> for monero::Network {
    fn from(n: Network) -> monero::Network {
        match n {
            Network::Mainnet => monero::Network::Mainnet,
            Network::Testnet => monero::Network::Stagenet,
            Network::Regtest => monero::Network::Testnet,
        }
    }
}

#[cfg(feature = "monero")]
impl From<&Network> for monero::Network {
    fn from(n: &Network) -> monero::Network {
        match n {
            Network::Mainnet => monero::Network::Mainnet,
            Network::Testnet => monero::Network::Stagenet,
            Network::Regtest => monero::Network::Testnet,
        }
    }
}

#[cfg(feature = "monero")]
impl From<&mut Network> for monero::Network {
    fn from(n: &mut Network) -> monero::Network {
        match n {
            Network::Mainnet => monero::Network::Mainnet,
            Network::Testnet => monero::Network::Stagenet,
            Network::Regtest => monero::Network::Testnet,
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

#[cfg(feature = "monero")]
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(crate = "moonramp_core::serde", rename_all = "UPPERCASE")]
pub enum MoneroColdWalletType {
    ViewKey, //P2PKH,
             //P2SHWPKH,
             //P2WPKH,
}

#[derive(Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(crate = "moonramp_core::serde")]
pub enum Wallet {
    #[cfg(feature = "bitcoin")]
    Bitcoin(bitcoin_wallet::BitcoinWallet),
    #[cfg(feature = "monero")]
    Monero(monero_wallet::MoneroWallet),
}

impl Wallet {
    pub fn pubkey(&self) -> String {
        match self {
            #[cfg(feature = "bitcoin")]
            Wallet::Bitcoin(w) => w.pubkey(),
            #[cfg(feature = "monero")]
            Wallet::Monero(w) => w.pubkey(),
        }
    }

    pub fn ticker(&self) -> Ticker {
        match self {
            #[cfg(feature = "bitcoin")]
            Wallet::Bitcoin(w) => w.ticker(),
            #[cfg(feature = "monero")]
            Wallet::Monero(w) => w.ticker(),
        }
    }

    pub fn network(&self) -> Network {
        match self {
            #[cfg(feature = "bitcoin")]
            Wallet::Bitcoin(w) => w.network(),
            #[cfg(feature = "monero")]
            Wallet::Monero(w) => w.network(),
        }
    }

    pub fn wallet_type(&self) -> WalletType {
        match self {
            #[cfg(feature = "bitcoin")]
            Wallet::Bitcoin(w) => w.wallet_type(),
            #[cfg(feature = "monero")]
            Wallet::Monero(w) => w.wallet_type(),
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
