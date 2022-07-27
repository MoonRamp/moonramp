use std::time::Duration;

use anyhow::anyhow;
use clap::Subcommand;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use moonramp_core::{anyhow, awc, serde, serde_json, uuid};
use moonramp_sale_rpc::{
    SaleCaptureRequest, SaleInvoiceLookupRequest, SaleInvoiceRequest, SaleLookupRequest,
};

#[derive(clap::ArgEnum, Clone, Debug, Deserialize, Serialize)]
#[serde(crate = "moonramp_core::serde")]
pub enum Currency {
    BTC,
    BCH,
    ETH,
    USDT,
    USDC,
    USDP,
}

impl From<Currency> for moonramp_wallet_rpc::Currency {
    fn from(c: Currency) -> moonramp_wallet_rpc::Currency {
        match c {
            Currency::BTC => moonramp_wallet_rpc::Currency::BTC,
            Currency::BCH => moonramp_wallet_rpc::Currency::BCH,
            Currency::ETH => moonramp_wallet_rpc::Currency::ETH,
            Currency::USDT => moonramp_wallet_rpc::Currency::USDT,
            Currency::USDC => moonramp_wallet_rpc::Currency::USDC,
            Currency::USDP => moonramp_wallet_rpc::Currency::USDP,
        }
    }
}

#[derive(Subcommand)]
pub enum SaleSubcommand {
    Invoice {
        #[clap(short = 'H', long)]
        hash: String,

        #[clap(short, long, arg_enum)]
        currency: Currency,

        #[clap(short, long)]
        amount: u64,

        #[clap(short, long)]
        expires_in: Option<i64>,

        #[clap(short, long)]
        program: Option<String>,
    },
    InvoiceLookup {
        #[clap(short = 'H', long)]
        hash: String,
    },
    Capture {
        #[clap(short = 'H', long)]
        hash: String,

        #[clap(short, long)]
        confirmations: Option<i64>,

        #[clap(short, long)]
        program: Option<String>,
    },
    Lookup {
        #[clap(short = 'H', long)]
        hash: String,
    },
    Version {},
}

pub struct SaleCtl {
    verbose: bool,
    endpoint: String,
    api_token: String,
}

impl SaleCtl {
    pub fn new(verbose: bool, endpoint: String, api_token: String) -> Self {
        SaleCtl {
            verbose,
            endpoint,
            api_token,
        }
    }

    pub async fn invoice(&self, req: SaleInvoiceRequest) -> anyhow::Result<()> {
        let id = Uuid::new_v4().to_simple().to_string();
        let json_rpc = json!({
            "jsonrpc": "2.0",
            "method": "sale.invoice",
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
            .timeout(Duration::from_secs(60))
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

    pub async fn invoice_lookup(&self, req: SaleInvoiceLookupRequest) -> anyhow::Result<()> {
        let id = Uuid::new_v4().to_simple().to_string();
        let json_rpc = json!({
            "jsonrpc": "2.0",
            "method": "sale.invoiceLookup",
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

    pub async fn capture(&self, req: SaleCaptureRequest) -> anyhow::Result<()> {
        let id = Uuid::new_v4().to_simple().to_string();
        let json_rpc = json!({
            "jsonrpc": "2.0",
            "method": "sale.capture",
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
            .timeout(Duration::from_secs(60))
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

    pub async fn lookup(&self, req: SaleLookupRequest) -> anyhow::Result<()> {
        let id = Uuid::new_v4().to_simple().to_string();
        let json_rpc = json!({
            "jsonrpc": "2.0",
            "method": "sale.lookup",
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
            "method": "sale.version",
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
