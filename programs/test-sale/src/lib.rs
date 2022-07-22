#[lunar::program(DefaultSale)]
mod program {
    use lunar::{EntryData, ExitData, LunarError, Program};

    pub struct DefaultSale {}

    impl Default for DefaultSale {
        fn default() -> Self {
            Self {}
        }
    }

    impl Program for DefaultSale {
        fn launch(self, entry_data: EntryData) -> Result<ExitData, LunarError> {
            match entry_data {
                EntryData::Invoice { wallet, .. } => Ok(ExitData::Invoice {
                    wallet,
                    pubkey: "test_pubkey".to_string(),
                    address: "test_address".to_string(),
                    uri: "test_uri".to_string(),
                    user_data: None,
                }),
                EntryData::Sale { .. } => Ok(ExitData::Sale {
                    funded: true,
                    amount: 1000,
                    user_data: None,
                }),
                #[allow(unreachable_patterns)]
                _ => Err(LunarError::Crash("EntryData not supported".to_string())),
            }
        }
    }
}
