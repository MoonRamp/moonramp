use clap::{Parser, Subcommand};
use env_logger::Env;
use log::trace;

use moonramp_bin::node_ctl::NodeCtl;
use moonramp_core::{anyhow, log, tokio};

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

        #[clap(short, long)]
        master_merchant_id: String,

        #[clap(short = 'M', long)]
        master_key_encryption_key: String,

        #[clap(short = 'u', long, default_value_t = String::from("postgresql://root@localhost:26257/defaultdb?sslmode=disable"))]
        db_url: String,
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
            master_merchant_id,
            master_key_encryption_key,
            db_url,
        } => {
            let mut node = NodeCtl::new(
                node_id.into(),
                program_http_addr,
                sale_http_addr,
                wallet_http_addr,
                master_merchant_id,
                master_key_encryption_key.as_bytes().to_vec(),
                db_url,
            )
            .await?;
            node.run().await?;
        }
    }
    Ok(())
}
