use std::time::Duration;

use anyhow::anyhow;
use clap::Subcommand;
use serde_json::json;
use uuid::Uuid;

use moonramp_core::{anyhow, awc, serde_json, uuid};
use moonramp_program_rpc::{ProgramCreateRequest, ProgramLookupRequest, ProgramUpdateRequest};

#[derive(Subcommand)]
pub enum ProgramSubcommand {
    Create {
        #[clap(short, long)]
        name: String,

        #[clap(short, long)]
        version: String,

        #[clap(short, long)]
        url: Option<String>,

        #[clap(short, long)]
        description: Option<String>,

        #[clap(short = 'P', long, parse(from_os_str))]
        program_path: std::path::PathBuf,

        #[clap(short, long)]
        public: bool,
    },
    Update {
        #[clap(short, long)]
        name: String,

        #[clap(short, long)]
        version: String,

        #[clap(short, long)]
        url: Option<String>,

        #[clap(short, long)]
        description: Option<String>,

        #[clap(short = 'P', long, parse(from_os_str))]
        program_path: std::path::PathBuf,
    },
    Lookup {
        #[clap(
            short = 'H',
            long,
            conflicts_with("name"),
            required_unless_present("name")
        )]
        hash: Option<String>,

        #[clap(short, long, conflicts_with("hash"), required_unless_present("hash"))]
        name: Option<String>,
    },
    Version {},
}

pub struct ProgramCtl {
    verbose: bool,
    endpoint: String,
    api_token: String,
}

impl ProgramCtl {
    pub fn new(verbose: bool, endpoint: String, api_token: String) -> Self {
        ProgramCtl {
            verbose,
            endpoint,
            api_token,
        }
    }

    pub async fn create(&self, req: ProgramCreateRequest) -> anyhow::Result<()> {
        let id = Uuid::new_v4().to_simple().to_string();
        let json_rpc = json!({
            "jsonrpc": "2.0",
            "method": "program.create",
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
            //println!("{}", serde_json::to_string_pretty(&json_rpc)?);
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

    pub async fn update(&self, req: ProgramUpdateRequest) -> anyhow::Result<()> {
        let id = Uuid::new_v4().to_simple().to_string();
        let json_rpc = json!({
            "jsonrpc": "2.0",
            "method": "program.update",
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

    pub async fn lookup(&self, req: ProgramLookupRequest) -> anyhow::Result<()> {
        let id = Uuid::new_v4().to_simple().to_string();
        let json_rpc = json!({
            "jsonrpc": "2.0",
            "method": "program.lookup",
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
            "method": "program.version",
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
