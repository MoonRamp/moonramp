#[moonramp_lunar::program(DefaultSale)]
mod program {
    use moonramp_lunar::{
        gateway::{BitcoinGateway, BitcoinGatewayResponse},
        moonramp_wallet::Wallet,
        EntryData, ExitData, LunarError, Program,
    };

    pub struct DefaultSale {}

    impl Default for DefaultSale {
        fn default() -> Self {
            Self {}
        }
    }

    impl Program for DefaultSale {
        fn launch(self, entry_data: EntryData) -> Result<ExitData, LunarError> {
            match entry_data {
                EntryData::Invoice { wallet, amount, .. } => {
                    let mut bitcoin_wallet = wallet
                        .into_bitcoin()
                        .map_err(|_| LunarError::Wallet("Wallet is not bitcoin".to_string()))?;
                    let (pubkey, address) = bitcoin_wallet
                        .next_addr()
                        .map_err(|err| LunarError::Wallet(err.to_string()))?;
                    let uri = format!("bitcoin:{};version=1.0&amount={}", address, amount);
                    Ok(ExitData::Invoice {
                        wallet: Wallet::Bitcoin(bitcoin_wallet),
                        pubkey: pubkey.to_string(),
                        address,
                        uri,
                        user_data: None,
                    })
                }
                EntryData::Sale {
                    address,
                    amount,
                    confirmations,
                    ..
                } => {
                    let bitcoin_gateway = BitcoinGateway::new();
                    loop {
                        match bitcoin_gateway.scan_tx_out(vec![format!("addr({})", address)])? {
                            BitcoinGatewayResponse::ScanTxOut(scan_res) => {
                                let total_amount = scan_res.total_amount.as_btc();
                                if let Some(current_height) = scan_res.height {
                                    let mut confirmed = true;
                                    for unspent in &scan_res.unspents {
                                        if current_height - unspent.height < confirmations {
                                            confirmed = false;
                                            break;
                                        }
                                    }
                                    if total_amount >= amount && confirmed {
                                        return Ok(ExitData::Sale {
                                            funded: true,
                                            amount: total_amount,
                                            user_data: None,
                                        });
                                    }
                                }
                            }
                        }
                        std::thread::sleep(std::time::Duration::from_secs(1));
                    }
                }
            }
        }
    }
}
