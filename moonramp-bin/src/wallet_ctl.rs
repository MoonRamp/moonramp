use std::convert::TryFrom;

use anyhow::anyhow;
use clap::Subcommand;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use moonramp_core::{anyhow, awc, serde, serde_json, uuid};
use moonramp_wallet_rpc::{BitcoinColdWalletType, WalletCreateRequest, WalletLookupRequest};

#[derive(clap::ArgEnum, Clone, Debug, Deserialize, Serialize)]
#[serde(crate = "moonramp_core::serde")]
pub enum Ticker {
    BTC,
    BCH,
    ETH,
    ETC,
}

#[derive(clap::ArgEnum, Clone, Debug, Deserialize, Serialize)]
#[serde(crate = "moonramp_core::serde")]
pub enum WalletType {
    Hot,
    Cold,
}

#[derive(clap::ArgEnum, Clone, Debug, Deserialize, Serialize)]
#[serde(crate = "moonramp_core::serde")]
pub enum WalletColdType {
    BtcXPub,
    BchXPub,
    //BtcP2PKH,
    //BtcP2SHWPKH,
    //BtcP2WPKH,
    //BthP2PKH,
}

impl TryFrom<WalletColdType> for BitcoinColdWalletType {
    type Error = anyhow::Error;
    fn try_from(cold_type: WalletColdType) -> anyhow::Result<BitcoinColdWalletType> {
        match cold_type {
            WalletColdType::BtcXPub => Ok(BitcoinColdWalletType::XPubkey),
            WalletColdType::BchXPub => Ok(BitcoinColdWalletType::XPubkey),
            //WalletColdType::BtcP2PKH => Ok(BitcoinColdWalletType::P2PKH),
            //WalletColdType::BtcP2SHWPKH => Ok(BitcoinColdWalletType::P2SHWPKH),
            //WalletColdType::BtcP2WPKH => Ok(BitcoinColdWalletType::P2WPKH),
            //WalletColdType::BthP2PKH => Ok(BitcoinColdWalletType::P2PKH),
        }
    }
}

#[derive(Subcommand)]
pub enum WalletSubcommand {
    Create {
        #[clap(short, long, arg_enum)]
        ticker: Ticker,

        #[clap(short, long, arg_enum)]
        wallet_type: WalletType,

        #[clap(short, long, required_if_eq("wallet-type", "cold"))]
        pubkey: Option<String>,

        #[clap(short, long, arg_enum, required_if_eq("wallet-type", "cold"))]
        cold_type: Option<WalletColdType>,
    },
    Lookup {
        #[clap(
            short = 'H',
            long,
            conflicts_with("pubkey"),
            required_unless_present("pubkey")
        )]
        hash: Option<String>,

        #[clap(short, long, conflicts_with("hash"), required_unless_present("hash"))]
        pubkey: Option<String>,
    },
    Version {},
}

pub struct WalletCtl {
    verbose: bool,
    endpoint: String,
    api_token: String,
}

impl WalletCtl {
    pub fn new(verbose: bool, endpoint: String, api_token: String) -> Self {
        WalletCtl {
            verbose,
            endpoint,
            api_token,
        }
    }

    pub async fn create(&self, req: WalletCreateRequest) -> anyhow::Result<()> {
        let id = Uuid::new_v4().to_simple().to_string();
        let json_rpc = json!({
            "jsonrpc": "2.0",
            "method": "wallet.create",
            "params": {
                "request": req,
            },
            "id": id,
        });

        let url = format!("{}/jsonrpc", self.endpoint);

        if self.verbose {
            println!("*****************************");
            println!("********** REQUEST **********");
            println!("*****************************");
            println!("{}", url);
            println!("{}", serde_json::to_string_pretty(&json_rpc)?);
        }

        let client = awc::Client::default();
        let mut response = client
            .post(&url)
            .insert_header((
                "User-Agent",
                format!("moonramp-cli/v{}", env!("CARGO_PKG_VERSION")),
            ))
            .bearer_auth(self.api_token.clone())
            .send_json(&json_rpc)
            .await
            .map_err(|err| anyhow!("{}", err))?;

        let response_json: serde_json::Value = response.json().await?;
        if self.verbose {
            println!("******************************");
            println!("********** RESPONSE **********");
            println!("******************************");
            println!("{:?}", response);
        }
        println!("{}", serde_json::to_string_pretty(&response_json)?);
        Ok(())
    }

    pub async fn lookup(&self, req: WalletLookupRequest) -> anyhow::Result<()> {
        let id = Uuid::new_v4().to_simple().to_string();
        let json_rpc = json!({
            "jsonrpc": "2.0",
            "method": "wallet.lookup",
            "params": {
                "request": req,
            },
            "id": id,
        });

        let url = format!("{}/jsonrpc", self.endpoint);

        if self.verbose {
            println!("*****************************");
            println!("********** REQUEST **********");
            println!("*****************************");
            println!("{}", url);
            println!("{}", serde_json::to_string_pretty(&json_rpc)?);
        }

        let client = awc::Client::default();
        let mut response = client
            .post(&url)
            .insert_header((
                "User-Agent",
                format!("moonramp-cli/v{}", env!("CARGO_PKG_VERSION")),
            ))
            .bearer_auth(self.api_token.clone())
            .send_json(&json_rpc)
            .await
            .map_err(|err| anyhow!("{}", err))?;

        let response_json: serde_json::Value = response.json().await?;
        if self.verbose {
            println!("******************************");
            println!("********** RESPONSE **********");
            println!("******************************");
            println!("{:?}", response);
        }
        println!("{}", serde_json::to_string_pretty(&response_json)?);
        Ok(())
    }

    pub async fn version(&self) -> anyhow::Result<()> {
        let id = Uuid::new_v4().to_simple().to_string();
        let json_rpc = json!({
            "jsonrpc": "2.0",
            "method": "wallet.version",
            "params": {},
            "id": id,
        });

        let url = format!("{}/jsonrpc", self.endpoint);

        if self.verbose {
            println!("*****************************");
            println!("********** REQUEST **********");
            println!("*****************************");
            println!("{}", url);
            println!("{}", serde_json::to_string_pretty(&json_rpc)?);
        }

        let client = awc::Client::default();
        let mut response = client
            .post(&url)
            .insert_header((
                "User-Agent",
                format!("moonramp-cli/v{}", env!("CARGO_PKG_VERSION")),
            ))
            .bearer_auth(self.api_token.clone())
            .send_json(&json_rpc)
            .await
            .map_err(|err| anyhow!("{}", err))?;

        let response_json: serde_json::Value = response.json().await?;
        if self.verbose {
            println!("******************************");
            println!("********** RESPONSE **********");
            println!("******************************");
            println!("{:?}", response);
        }
        println!("{}", serde_json::to_string_pretty(&response_json)?);
        Ok(())
    }
}
