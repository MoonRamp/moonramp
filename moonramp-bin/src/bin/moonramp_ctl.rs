use clap::{Parser, Subcommand};
use env_logger::Env;
use log::trace;
use tokio::fs;
use uuid::Uuid;

use moonramp::program_ctl::{ProgramCtl, ProgramSubcommand};
use moonramp::sale_ctl::{SaleCtl, SaleSubcommand};
use moonramp::wallet_ctl::{Ticker, WalletCtl, WalletSubcommand, WalletType};
use moonramp_core::{actix_rt, anyhow, log, tokio, uuid};
use moonramp_program_rpc::{ProgramCreateRequest, ProgramLookupRequest, ProgramUpdateRequest};
use moonramp_sale_rpc::{
    SaleCaptureRequest, SaleInvoiceLookupRequest, SaleInvoiceRequest, SaleLookupRequest,
};
use moonramp_wallet_rpc::{WalletCreateRequest, WalletLookupRequest};

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    #[clap(short = 'V', long)]
    verbose: bool,

    #[clap(short, long, default_value_t = String::from("http://127.0.0.1:9370"))]
    program_endpoint: String,

    #[clap(short, long, default_value_t = String::from("http://127.0.0.1:9371"))]
    sale_endpoint: String,

    #[clap(short, long, default_value_t = String::from("http://127.0.0.1:9372"))]
    wallet_endpoint: String,

    #[clap(short, long)]
    api_token: String,

    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    #[clap(subcommand)]
    Program(ProgramSubcommand),
    #[clap(subcommand)]
    Sale(SaleSubcommand),
    #[clap(subcommand)]
    Wallet(WalletSubcommand),
}

#[actix_rt::main]
async fn main() -> anyhow::Result<()> {
    let env_level = Env::default().default_filter_or("moonramp=info");
    env_logger::Builder::from_env(env_level).init();

    let cli = Cli::parse();

    trace!("moonrampctl {:?}", env!("CARGO_PKG_VERSION"));

    match cli.command {
        Commands::Program(subcommand) => {
            let program = ProgramCtl::new(cli.verbose, cli.program_endpoint, cli.api_token);
            match subcommand {
                ProgramSubcommand::Create {
                    name,
                    version,
                    url,
                    description,
                    program_path,
                    public,
                } => {
                    let data = fs::read(program_path).await?;
                    program
                        .create(ProgramCreateRequest {
                            name,
                            version,
                            url,
                            description,
                            data,
                            private: !public,
                        })
                        .await?;
                }
                ProgramSubcommand::Update {
                    name,
                    version,
                    url,
                    description,
                    program_path,
                } => {
                    let data = fs::read(program_path).await?;
                    program
                        .update(ProgramUpdateRequest {
                            name,
                            version,
                            url,
                            description,
                            data,
                        })
                        .await?;
                }
                ProgramSubcommand::Lookup { hash, name } => match (hash, name) {
                    (Some(hash), None) => {
                        program.lookup(ProgramLookupRequest::Hash { hash }).await?;
                    }
                    (None, Some(name)) => {
                        program.lookup(ProgramLookupRequest::Name { name }).await?;
                    }
                    _ => unreachable!(),
                },
                ProgramSubcommand::Version {} => {
                    program.version().await?;
                }
            }
        }
        Commands::Sale(subcommand) => {
            let sale = SaleCtl::new(cli.verbose, cli.sale_endpoint, cli.api_token);
            match subcommand {
                SaleSubcommand::Invoice {
                    hash,
                    currency,
                    amount,
                    expires_in,
                    program,
                } => {
                    sale.invoice(SaleInvoiceRequest {
                        hash,
                        uuid: Uuid::new_v4().to_simple().to_string(),
                        currency: currency.into(),
                        amount,
                        expires_in,
                        user_data: None,
                        program,
                    })
                    .await?;
                }
                SaleSubcommand::InvoiceLookup { hash } => {
                    sale.invoice_lookup(SaleInvoiceLookupRequest::Hash { hash })
                        .await?;
                }
                SaleSubcommand::Capture {
                    hash,
                    confirmations,
                    program,
                } => {
                    sale.capture(SaleCaptureRequest {
                        hash,
                        uuid: Uuid::new_v4().to_simple().to_string(),
                        confirmations,
                        user_data: None,
                        program,
                    })
                    .await?;
                }
                SaleSubcommand::Lookup { hash, invoice_hash } => match (hash, invoice_hash) {
                    (Some(hash), None) => sale.lookup(SaleLookupRequest::Hash { hash }).await?,
                    (None, Some(invoice_hash)) => {
                        sale.lookup(SaleLookupRequest::InvoiceHash { invoice_hash })
                            .await?
                    }
                    _ => unreachable!(),
                },
                SaleSubcommand::Version {} => {
                    sale.version().await?;
                }
            }
        }
        Commands::Wallet(subcommand) => {
            let wallet = WalletCtl::new(cli.verbose, cli.wallet_endpoint, cli.api_token);
            match subcommand {
                WalletSubcommand::Create {
                    ticker,
                    wallet_type,
                    pubkey,
                    cold_type,
                } => {
                    let req = match (ticker, wallet_type, pubkey, cold_type) {
                        (Ticker::BTC, WalletType::Hot, None, None) => WalletCreateRequest::BtcHot,
                        (Ticker::BTC, WalletType::Cold, Some(pubkey), Some(cold_type)) => {
                            WalletCreateRequest::BtcCold {
                                pubkey,
                                cold_type: cold_type.try_into()?,
                            }
                        }
                        (Ticker::BCH, WalletType::Hot, None, None) => WalletCreateRequest::BchHot,
                        (Ticker::BCH, WalletType::Cold, Some(pubkey), Some(cold_type)) => {
                            WalletCreateRequest::BchCold {
                                pubkey,
                                cold_type: cold_type.try_into()?,
                            }
                        }
                        (Ticker::XMR, WalletType::Hot, None, None) => WalletCreateRequest::XmrHot,
                        _ => todo!(),
                    };
                    wallet.create(req).await?;
                }
                WalletSubcommand::Lookup { hash, pubkey } => match (hash, pubkey) {
                    (Some(hash), None) => {
                        wallet.lookup(WalletLookupRequest::Hash { hash }).await?;
                    }
                    (None, Some(pubkey)) => {
                        wallet
                            .lookup(WalletLookupRequest::Pubkey { pubkey })
                            .await?;
                    }
                    _ => unreachable!(),
                },
                WalletSubcommand::Version {} => {
                    wallet.version().await?;
                }
            }
        }
    }
    Ok(())
}
