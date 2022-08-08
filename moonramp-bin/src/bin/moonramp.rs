use clap::{Parser, Subcommand};
use env_logger::Env;
use log::trace;
use serde::{Deserialize, Serialize};

use moonramp::node_ctl::NodeCtl;
use moonramp_core::{anyhow, log, serde, tokio};

#[derive(clap::ArgEnum, Clone, Debug, Deserialize, Serialize)]
#[serde(crate = "moonramp_core::serde")]
pub enum NetworkOpt {
    Regtest,
    Testnet,
    Mainnet,
}

impl From<NetworkOpt> for moonramp_wallet_rpc::Network {
    fn from(val: NetworkOpt) -> moonramp_wallet_rpc::Network {
        match val {
            NetworkOpt::Regtest => moonramp_wallet_rpc::Network::Regtest,
            NetworkOpt::Testnet => moonramp_wallet_rpc::Network::Testnet,
            NetworkOpt::Mainnet => moonramp_wallet_rpc::Network::Mainnet,
        }
    }
}

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Node {
        #[clap(short, long)]
        node_id: String,

        #[clap(short, long, default_value_t = String::from("127.0.0.1:9370"))]
        program_http_addr: String,

        #[clap(short, long, default_value_t = String::from("127.0.0.1:9371"))]
        sale_http_addr: String,

        #[clap(short, long, default_value_t = String::from("127.0.0.1:9372"))]
        wallet_http_addr: String,

        #[clap(short = 'b', long, default_value_t = String::from("http://127.0.0.1:18443/"))]
        bitcoin_rpc_endpoint: String,

        #[clap(short = 'B', long, default_value_t = String::from("bW9vbnJhbXA6bW9vbnJhbXA="))]
        bitcoin_rpc_auth: String,

        #[clap(short, long)]
        master_merchant_id: String,

        #[clap(short = 'M', long)]
        master_key_encryption_key: String,

        #[clap(short = 'u', long, default_value_t = String::from("postgresql://postgres:postgres@localhost:26257/postgres?sslmode=disable"))]
        db_url: String,

        #[clap(short = 'N', long, arg_enum, default_value_t = NetworkOpt::Mainnet)]
        network: NetworkOpt,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let env_level = Env::default().default_filter_or("moonramp=info");
    env_logger::Builder::from_env(env_level).init();

    let cli = Cli::parse();

    trace!("moonramp {:?}", env!("CARGO_PKG_VERSION"));

    match cli.command {
        Commands::Node {
            node_id,
            program_http_addr,
            sale_http_addr,
            wallet_http_addr,
            bitcoin_rpc_endpoint,
            bitcoin_rpc_auth,
            master_merchant_id,
            master_key_encryption_key,
            db_url,
            network,
        } => {
            let mut node = NodeCtl::new(
                node_id.into(),
                program_http_addr,
                sale_http_addr,
                wallet_http_addr,
                bitcoin_rpc_endpoint,
                bitcoin_rpc_auth,
                master_merchant_id,
                master_key_encryption_key.as_bytes().to_vec(),
                db_url,
                network.into(),
            )
            .await?;
            node.run().await?;
        }
    }
    Ok(())
}
