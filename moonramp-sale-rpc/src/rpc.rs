use std::sync::Arc;

use anyhow::anyhow;
use async_trait::async_trait;
use chrono::{Duration, Utc};
use jsonrpsee::{core::RpcResult, proc_macros::rpc, RpcModule};
use log::debug;
use sea_orm::{entity::*, query::*, DatabaseConnection, DatabaseTransaction};
use sha3::{Digest, Sha3_256};
use tokio::{
    sync::{mpsc, RwLock},
    time::Instant,
};

use moonramp_core::{
    anyhow, async_trait, chrono, log, sea_orm, serde_json, sha3, tokio, Hash,
    NetworkTunnelReceiver, NetworkTunnelSender, NodeId, TunnelName,
};
use moonramp_encryption::{
    EncryptionKeyCustodian, KeyCustodian, KeyEncryptionKeyCustodian, MerchantScopedSecret,
};
use moonramp_entity::{cipher::Cipher, encryption_key, invoice, program, sale, wallet};
use moonramp_program::{BitcoinRpcConfig, Runtime};
use moonramp_rpc::{IntoRpcResult, RpcService};
use moonramp_sale::{Invoice, Sale};
use moonramp_wallet::{Network, Wallet};

use crate::params::*;

#[rpc(server)]
pub trait SaleRpc {
    #[method(name = "sale.version")]
    fn version(&self) -> RpcResult<String>;

    #[method(name = "sale.invoice")]
    async fn invoice(
        &self,
        merchant_id: String,
        request: SaleInvoiceRequest,
    ) -> RpcResult<SaleInvoiceResponse>;

    #[method(name = "sale.invoiceLookup")]
    async fn invoice_lookup(
        &self,
        merchant_id: String,
        request: SaleInvoiceLookupRequest,
    ) -> RpcResult<Option<SaleInvoiceResponse>>;

    #[method(name = "sale.capture")]
    async fn capture(
        &self,
        merchant_id: String,
        request: SaleCaptureRequest,
    ) -> RpcResult<SaleResponse>;

    //#[method(name = "sale.captureAsync")]
    //async fn capture_async(
    //    &self,
    //    merchant_id: String,
    //    request: SaleCaptureAsyncRequest,
    //) -> RpcResult<SaleResponse>;

    //#[method(name = "sale.checkout")]
    //async fn checkout(
    //    &self,
    //    merchant_id: String,
    //    request: SaleCheckoutRequest,
    //) -> RpcResult<SaleCheckoutResponse>;

    #[method(name = "sale.lookup")]
    async fn lookup(
        &self,
        merchant_id: String,
        request: SaleLookupRequest,
    ) -> RpcResult<Option<SaleResponse>>;
}

#[derive(Clone)]
pub struct SaleRpcImpl {
    master_merchant_id: Arc<String>,
    kek_custodian: Arc<KeyEncryptionKeyCustodian>,
    database: DatabaseConnection,
    bitcoin_gateway_config: BitcoinRpcConfig,
}

impl SaleRpcImpl {
    async fn load_program(
        &self,
        txn: &DatabaseTransaction,
        merchant_id: String,
        program: Option<String>,
    ) -> anyhow::Result<(program::Model, EncryptionKeyCustodian)> {
        let p = if let Some(p) = program {
            program::Entity::find()
                .filter(
                    Condition::all()
                        .add(program::Column::Hash.eq(p))
                        .add(program::Column::MerchantId.eq(merchant_id)),
                )
                .one(&self.database)
                .await?
        } else {
            program::Entity::find()
                .filter(
                    Condition::all()
                        .add(program::Column::Name.eq("moonramp-program-default-sale"))
                        .add(
                            program::Column::MerchantId
                                .eq(self.master_merchant_id.as_ref().clone()),
                        ),
                )
                .order_by_desc(program::Column::Revision)
                .all(txn)
                .await?
                .into_iter()
                .next()
        }
        .ok_or(anyhow!("Failed to find program"))?;

        let p_ek = encryption_key::Entity::find()
            .filter(
                Condition::all()
                    .add(encryption_key::Column::Id.eq(p.encryption_key_id.clone()))
                    .add(encryption_key::Column::KeyEncryptionKeyId.eq(self.kek_custodian.id())),
            )
            .all(txn)
            .await?
            .into_iter()
            .next()
            .ok_or(anyhow!("Failed load program"))?;

        let p_ek_custodian = EncryptionKeyCustodian::new(
            self.kek_custodian.unlock(p_ek)?.secret.to_vec(),
            p.cipher.clone(),
        )?;

        Ok((p, p_ek_custodian))
    }

    async fn load_wallet(
        &self,
        txn: &DatabaseTransaction,
        merchant_id: String,
        hash: String,
    ) -> anyhow::Result<(wallet::Model, EncryptionKeyCustodian)> {
        let w = wallet::Entity::find()
            .filter(
                Condition::all()
                    .add(wallet::Column::Hash.eq(hash))
                    .add(wallet::Column::MerchantId.eq(merchant_id)),
            )
            .all(txn)
            .await?
            .into_iter()
            .next()
            .ok_or(anyhow!("Failed to find wallet"))?;

        let w_ek = encryption_key::Entity::find()
            .filter(
                Condition::all()
                    .add(encryption_key::Column::Id.eq(w.encryption_key_id.clone()))
                    .add(encryption_key::Column::KeyEncryptionKeyId.eq(self.kek_custodian.id())),
            )
            .all(txn)
            .await?
            .into_iter()
            .next()
            .ok_or(anyhow!("Failed load wallet"))?;

        let w_ek_custodian = EncryptionKeyCustodian::new(
            self.kek_custodian.unlock(w_ek)?.secret.to_vec(),
            w.cipher.clone(),
        )?;

        Ok((w, w_ek_custodian))
    }

    async fn load_wallet_with_lock(
        &self,
        txn: &DatabaseTransaction,
        merchant_id: String,
        hash: String,
    ) -> anyhow::Result<(wallet::Model, EncryptionKeyCustodian)> {
        let w = wallet::Entity::find()
            .filter(
                Condition::all()
                    .add(wallet::Column::Hash.eq(hash))
                    .add(wallet::Column::MerchantId.eq(merchant_id)),
            )
            .lock_exclusive()
            .all(txn)
            .await?
            .into_iter()
            .next()
            .ok_or(anyhow!("Failed to find wallet"))?;

        let w_ek = encryption_key::Entity::find()
            .filter(
                Condition::all()
                    .add(encryption_key::Column::Id.eq(w.encryption_key_id.clone()))
                    .add(encryption_key::Column::KeyEncryptionKeyId.eq(self.kek_custodian.id())),
            )
            .all(txn)
            .await?
            .into_iter()
            .next()
            .ok_or(anyhow!("Failed load wallet"))?;

        let w_ek_custodian = EncryptionKeyCustodian::new(
            self.kek_custodian.unlock(w_ek)?.secret.to_vec(),
            w.cipher.clone(),
        )?;

        Ok((w, w_ek_custodian))
    }
}

#[async_trait]
impl SaleRpcServer for SaleRpcImpl {
    fn version(&self) -> RpcResult<String> {
        Ok(env!("CARGO_PKG_VERSION").to_string())
    }

    async fn invoice(
        &self,
        merchant_id: String,
        request: SaleInvoiceRequest,
    ) -> RpcResult<SaleInvoiceResponse> {
        debug!("sale.invoice {:?}", request);
        let program_find_start = Instant::now();

        let txn = self.database.begin().await.into_rpc_result()?;
        let (p, p_ek_custodian) = self
            .load_program(&txn, merchant_id.clone(), request.program)
            .await
            .into_rpc_result()?;

        debug!(
            "Program found in {}ms",
            program_find_start.elapsed().as_millis()
        );

        let (w, w_ek_custodian) = self
            .load_wallet_with_lock(&txn, merchant_id.clone(), request.hash)
            .await
            .into_rpc_result()?;

        let program_decrypt_start = Instant::now();
        let wasm_mod_bytes = p_ek_custodian
            .decrypt(&p.nonce, &p.blob)
            .into_rpc_result()?;

        debug!(
            "Program decrypted in {}ms",
            program_decrypt_start.elapsed().as_millis()
        );

        let wallet_bytes = w_ek_custodian
            .decrypt(&w.nonce, &w.blob)
            .into_rpc_result()?;

        let live_w: Wallet = serde_json::from_slice(&wallet_bytes).into_rpc_result()?;

        let program_run_start = Instant::now();
        let i: Invoice = Runtime::exec(
            &wasm_mod_bytes,
            moonramp_lunar::EntryData::Invoice {
                wallet: live_w,
                currency: request.currency.clone(),
                amount: request.amount.clone(),
                user_data: request.user_data,
            },
            tokio::time::Duration::from_millis(55000),
            self.bitcoin_gateway_config.clone(),
        )
        .await?
        .try_into()
        .into_rpc_result()?;

        debug!(
            "Program ran in {}ms",
            program_run_start.elapsed().as_millis()
        );

        let live_w = i.wallet;

        let (nonce, ciphertext) = w_ek_custodian
            .encrypt(&serde_json::to_vec(&live_w).into_rpc_result()?)
            .into_rpc_result()?;
        let mut w: wallet::ActiveModel = w.into();
        w.blob = Set(ciphertext);
        w.nonce = Set(nonce);
        let w = w.update(&txn).await.into_rpc_result()?;
        txn.commit().await.into_rpc_result()?;

        let ek = self
            .kek_custodian
            .lock(MerchantScopedSecret {
                merchant_id: merchant_id.clone(),
                secret: self.kek_custodian.gen_secret().into_rpc_result()?,
            })
            .into_rpc_result()?
            .insert(&self.database)
            .await
            .into_rpc_result()?;

        let ek_custodian = EncryptionKeyCustodian::new(
            self.kek_custodian
                .unlock(ek)
                .into_rpc_result()?
                .secret
                .to_vec(),
            Cipher::Aes256GcmSiv,
        )
        .into_rpc_result()?;

        let (nonce, ciphertext) = ek_custodian
            .encrypt(&serde_json::to_vec(&i.user_data).into_rpc_result()?)
            .into_rpc_result()?;

        let mut hasher = Sha3_256::new();
        hasher.update(request.uuid + &i.address);
        let hash = Hash::try_from(hasher.finalize().to_vec()).into_rpc_result()?;

        let expires_in = request.expires_in.unwrap_or(15 * 60);

        let invoice_res: SaleInvoiceResponse = invoice::ActiveModel {
            hash: Set(hash),
            merchant_id: Set(merchant_id),
            wallet_hash: Set(w.hash),
            ticker: Set(w.ticker),
            currency: Set(request.currency.into()),
            network: Set(w.network),
            invoice_status: Set(invoice::InvoiceStatus::Pending),
            pubkey: Set(i.pubkey),
            address: Set(i.address),
            amount: Set(request.amount),
            uri: Set(i.uri),
            encryption_key_id: Set(ek_custodian.id()),
            cipher: Set(Cipher::Aes256GcmSiv),
            blob: Set(ciphertext),
            nonce: Set(nonce),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
            expires_at: Set(Utc::now() + Duration::seconds(expires_in)),
        }
        .insert(&self.database)
        .await
        .into_rpc_result()?
        .into();
        Ok(invoice_res.with_user_data(i.user_data))
    }

    async fn invoice_lookup(
        &self,
        merchant_id: String,
        request: SaleInvoiceLookupRequest,
    ) -> RpcResult<Option<SaleInvoiceResponse>> {
        debug!("sale.invoiceLookup {:?}", request);
        let i = match request {
            SaleInvoiceLookupRequest::Hash { hash } => invoice::Entity::find()
                .filter(
                    Condition::all()
                        .add(invoice::Column::Hash.eq(hash))
                        .add(invoice::Column::MerchantId.eq(merchant_id)),
                )
                .one(&self.database)
                .await
                .into_rpc_result()?,
        };

        match i {
            Some(i) => {
                let ek = encryption_key::Entity::find()
                    .filter(
                        Condition::all()
                            .add(encryption_key::Column::Id.eq(i.encryption_key_id.clone()))
                            .add(
                                encryption_key::Column::KeyEncryptionKeyId
                                    .eq(self.kek_custodian.id()),
                            ),
                    )
                    .one(&self.database)
                    .await
                    .into_rpc_result()?
                    .ok_or(anyhow!("Failed load invoice"))
                    .into_rpc_result()?;

                let ek_custodian = EncryptionKeyCustodian::new(
                    self.kek_custodian
                        .unlock(ek)
                        .into_rpc_result()?
                        .secret
                        .to_vec(),
                    i.cipher.clone(),
                )
                .into_rpc_result()?;

                let blob = ek_custodian.decrypt(&i.nonce, &i.blob).into_rpc_result()?;
                let user_data: Option<Vec<u8>> = serde_json::from_slice(&blob).into_rpc_result()?;
                let invoice_res: SaleInvoiceResponse = i.into();
                Ok(Some(invoice_res.with_user_data(user_data)))
            }
            None => Ok(None),
        }
    }

    async fn capture(
        &self,
        merchant_id: String,
        request: SaleCaptureRequest,
    ) -> RpcResult<SaleResponse> {
        debug!("sale.capture {:?}", request);
        let txn = self.database.begin().await.into_rpc_result()?;
        let i = invoice::Entity::find()
            .filter(
                Condition::all()
                    .add(invoice::Column::Hash.eq(request.hash.clone()))
                    .add(invoice::Column::MerchantId.eq(merchant_id.clone())),
            )
            .lock_exclusive()
            .all(&txn)
            .await
            .into_rpc_result()?
            .into_iter()
            .next()
            .ok_or(anyhow!("Failed load invoice"))
            .into_rpc_result()?;

        if i.invoice_status == invoice::InvoiceStatus::Funded {
            txn.rollback().await.into_rpc_result()?;
            return self
                .lookup(
                    merchant_id,
                    SaleLookupRequest::InvoiceHash {
                        invoice_hash: i.hash.to_string(),
                    },
                )
                .await?
                .ok_or(anyhow!("Invoice has already been captured but has no sale"))
                .into_rpc_result();
        }

        let program_find_start = Instant::now();

        let (p, p_ek_custodian) = self
            .load_program(&txn, merchant_id.clone(), request.program)
            .await
            .into_rpc_result()?;

        debug!(
            "Program found in {}ms",
            program_find_start.elapsed().as_millis()
        );

        let (w, w_ek_custodian) = self
            .load_wallet(&txn, merchant_id.clone(), i.wallet_hash.to_string())
            .await
            .into_rpc_result()?;

        let program_decrypt_start = Instant::now();
        let wasm_mod_bytes = p_ek_custodian
            .decrypt(&p.nonce, &p.blob)
            .into_rpc_result()?;

        debug!(
            "Program decrypted in {}ms",
            program_decrypt_start.elapsed().as_millis()
        );

        let wallet_bytes = w_ek_custodian
            .decrypt(&w.nonce, &w.blob)
            .into_rpc_result()?;

        let live_w: Wallet = serde_json::from_slice(&wallet_bytes).into_rpc_result()?;

        let confirmations = request.confirmations.unwrap_or(0);

        let program_run_start = Instant::now();
        let s: Sale = Runtime::exec(
            &wasm_mod_bytes,
            moonramp_lunar::EntryData::Sale {
                wallet: live_w,
                currency: i.currency.clone().into(),
                amount: i.amount,
                address: i.address.clone(),
                confirmations: confirmations as u64,
                user_data: request.user_data,
            },
            tokio::time::Duration::from_millis(55000),
            self.bitcoin_gateway_config.clone(),
        )
        .await?
        .try_into()
        .into_rpc_result()?;
        txn.commit().await.into_rpc_result()?;

        debug!(
            "Program ran in {}ms",
            program_run_start.elapsed().as_millis()
        );

        let ek = self
            .kek_custodian
            .lock(MerchantScopedSecret {
                merchant_id: merchant_id.clone(),
                secret: self.kek_custodian.gen_secret().into_rpc_result()?,
            })
            .into_rpc_result()?
            .insert(&self.database)
            .await
            .into_rpc_result()?;

        let ek_custodian = EncryptionKeyCustodian::new(
            self.kek_custodian
                .unlock(ek)
                .into_rpc_result()?
                .secret
                .to_vec(),
            Cipher::Aes256GcmSiv,
        )
        .into_rpc_result()?;

        let (nonce, ciphertext) = ek_custodian
            .encrypt(&serde_json::to_vec(&s.user_data).into_rpc_result()?)
            .into_rpc_result()?;

        let mut hasher = Sha3_256::new();
        hasher.update(request.uuid + &request.hash);
        let hash = Hash::try_from(hasher.finalize().to_vec()).into_rpc_result()?;

        if s.funded {
            debug!("Sale {} funded with amount {}", hash, s.amount);

            let mut i: invoice::ActiveModel = i.clone().into();
            i.updated_at = Set(Utc::now());
            i.invoice_status = Set(invoice::InvoiceStatus::Funded);
            i.update(&self.database).await.into_rpc_result()?;
        }

        let sale_res: SaleResponse = sale::ActiveModel {
            hash: Set(hash),
            merchant_id: Set(merchant_id),
            wallet_hash: Set(w.hash),
            invoice_hash: Set(i.hash),
            ticker: Set(i.ticker),
            currency: Set(i.currency),
            network: Set(i.network),
            pubkey: Set(i.pubkey),
            address: Set(i.address),
            amount: Set(s.amount),
            confirmations: Set(confirmations),
            encryption_key_id: Set(ek_custodian.id()),
            cipher: Set(Cipher::Aes256GcmSiv),
            blob: Set(ciphertext),
            nonce: Set(nonce),
            created_at: Set(Utc::now()),
        }
        .insert(&self.database)
        .await
        .into_rpc_result()?
        .into();
        Ok(sale_res.with_user_data(s.user_data))
    }

    async fn lookup(
        &self,
        merchant_id: String,
        request: SaleLookupRequest,
    ) -> RpcResult<Option<SaleResponse>> {
        debug!("sale.lookup {:?}", request);
        let s = match request {
            SaleLookupRequest::Hash { hash } => sale::Entity::find()
                .filter(
                    Condition::all()
                        .add(sale::Column::Hash.eq(hash))
                        .add(sale::Column::MerchantId.eq(merchant_id)),
                )
                .one(&self.database)
                .await
                .into_rpc_result()?,
            SaleLookupRequest::InvoiceHash { invoice_hash } => sale::Entity::find()
                .filter(
                    Condition::all()
                        .add(sale::Column::InvoiceHash.eq(invoice_hash))
                        .add(sale::Column::MerchantId.eq(merchant_id)),
                )
                .one(&self.database)
                .await
                .into_rpc_result()?,
        };

        match s {
            Some(s) => {
                let ek = encryption_key::Entity::find()
                    .filter(
                        Condition::all()
                            .add(encryption_key::Column::Id.eq(s.encryption_key_id.clone()))
                            .add(
                                encryption_key::Column::KeyEncryptionKeyId
                                    .eq(self.kek_custodian.id()),
                            ),
                    )
                    .one(&self.database)
                    .await
                    .into_rpc_result()?
                    .ok_or(anyhow!("Failed load invoice"))
                    .into_rpc_result()?;

                let ek_custodian = EncryptionKeyCustodian::new(
                    self.kek_custodian
                        .unlock(ek)
                        .into_rpc_result()?
                        .secret
                        .to_vec(),
                    s.cipher.clone(),
                )
                .into_rpc_result()?;

                let blob = ek_custodian.decrypt(&s.nonce, &s.blob).into_rpc_result()?;
                let user_data: Option<Vec<u8>> = serde_json::from_slice(&blob).into_rpc_result()?;
                let sale_res: SaleResponse = s.into();
                Ok(Some(sale_res.with_user_data(user_data)))
            }
            None => Ok(None),
        }
    }
}

pub struct SaleRpcService {
    node_id: NodeId,
    rx: Arc<RwLock<NetworkTunnelReceiver>>,
    rpc: RpcModule<SaleRpcImpl>,
}

impl SaleRpcService {
    pub fn new(
        node_id: NodeId,
        master_merchant_id: Arc<String>,
        kek_custodian: Arc<KeyEncryptionKeyCustodian>,
        database: DatabaseConnection,
        bitcoin_rpc_endpoint: String,
        bitcoin_rpc_auth: String,
        _network: Network,
    ) -> anyhow::Result<(NetworkTunnelSender, Arc<Self>)> {
        let (public_tx, public_network_rx) = mpsc::channel(1024);

        // Sale Rpc
        let bitcoin_gateway_config = BitcoinRpcConfig {
            endpoint: bitcoin_rpc_endpoint,
            basic_auth: Some(bitcoin_rpc_auth),
        };
        let rpc = SaleRpcImpl {
            master_merchant_id,
            kek_custodian,
            database,
            bitcoin_gateway_config,
        }
        .into_rpc();

        Ok((
            public_tx,
            Arc::new(SaleRpcService {
                node_id,
                rx: Arc::new(RwLock::new(public_network_rx)),
                rpc,
            }),
        ))
    }
}

#[async_trait]
impl RpcService<SaleRpcImpl> for SaleRpcService {
    fn log_target(&self) -> String {
        "moonramp_sale::rpc".to_string()
    }

    fn node_id(&self) -> NodeId {
        self.node_id.clone()
    }

    fn service_name(&self) -> TunnelName {
        TunnelName::Sale
    }

    fn rx(&self) -> Arc<RwLock<NetworkTunnelReceiver>> {
        self.rx.clone()
    }

    fn rpc(&self) -> RpcModule<SaleRpcImpl> {
        self.rpc.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::Database;
    use serde_json::json;

    use moonramp_encryption::MasterKeyEncryptionKeyCustodian;
    use moonramp_migration::testing::setup_testdb;
    use moonramp_wallet::{BitcoinWallet, Currency, Network, Ticker};

    async fn test_kek(
        database: &DatabaseConnection,
    ) -> anyhow::Result<Arc<KeyEncryptionKeyCustodian>> {
        let master_key_encryption_key = vec![0u8; 32];
        let master_custodian = MasterKeyEncryptionKeyCustodian::new(master_key_encryption_key)?;
        let secret = master_custodian.gen_secret()?;
        let kek = master_custodian.lock(secret)?.insert(database).await?;
        Ok(Arc::new(KeyEncryptionKeyCustodian::new(
            master_custodian.unlock(kek)?.to_vec(),
        )?))
    }

    async fn test_rpc(
        create_wallet: bool,
        create_invoice: bool,
    ) -> anyhow::Result<(String, Option<Hash>, Option<Hash>, RpcModule<SaleRpcImpl>)> {
        let database = Database::connect("sqlite::memory:")
            .await
            .expect("Failed to open in-memory sqlite db");
        let t = setup_testdb(&database)
            .await
            .expect("Failed to setup testdb");
        let kek_custodian = test_kek(&database)
            .await
            .expect("Invalid KeyEncryptionKeyCustodian");

        let ek = kek_custodian
            .lock(MerchantScopedSecret {
                merchant_id: t.merchant_id.clone(),
                secret: kek_custodian.gen_secret()?,
            })?
            .insert(&database)
            .await?;

        let ek_custodian =
            EncryptionKeyCustodian::new(kek_custodian.unlock(ek)?.secret.to_vec(), Cipher::Noop)?;

        let data = include_bytes!(
            "../../programs/test-sale/target/wasm32-wasi/debug/moonramp_program_test_sale.wasm"
        )
        .to_vec();

        let mut hasher = Sha3_256::new();
        hasher.update(&data);
        let hash = Hash::try_from(hasher.finalize().to_vec())?;

        let wasm_mod_bytes = Runtime::compile(&data)?;
        let (nonce, ciphertext) = ek_custodian.encrypt(&wasm_mod_bytes)?;

        program::ActiveModel {
            hash: Set(hash),
            merchant_id: Set(t.merchant_id.clone()),
            name: Set("moonramp-program-default-sale".to_string()),
            version: Set("0.1.0".to_string()),
            url: Set(None),
            description: Set(None),
            private: Set(true),
            revision: Set(0),
            encryption_key_id: Set(ek_custodian.id()),
            cipher: Set(Cipher::Noop),
            blob: Set(ciphertext),
            nonce: Set(nonce),
            created_at: Set(Utc::now()),
        }
        .insert(&database)
        .await?;

        let ek = kek_custodian
            .lock(MerchantScopedSecret {
                merchant_id: t.merchant_id.clone(),
                secret: kek_custodian.gen_secret()?,
            })?
            .insert(&database)
            .await?;

        let ek_custodian = EncryptionKeyCustodian::new(
            kek_custodian.unlock(ek)?.secret.to_vec(),
            Cipher::Aes256GcmSiv,
        )?;

        let wallet_hash = if create_wallet || create_invoice {
            let w = Wallet::Bitcoin(BitcoinWallet::new_hot(Ticker::BTC, Network::Testnet)?);
            let (nonce, ciphertext) = ek_custodian.encrypt(&serde_json::to_vec(&w)?)?;

            let mut hasher = Sha3_256::new();
            hasher.update(w.pubkey().as_bytes());
            let hash = Hash::try_from(hasher.finalize().to_vec())?;

            wallet::ActiveModel {
                hash: Set(hash.clone()),
                merchant_id: Set(t.merchant_id.clone()),
                ticker: Set(w.ticker().into()),
                network: Set(w.network().into()),
                wallet_type: Set(w.wallet_type().into()),
                pubkey: Set(w.pubkey().to_string()),
                encryption_key_id: Set(ek_custodian.id()),
                cipher: Set(Cipher::Aes256GcmSiv),
                blob: Set(ciphertext),
                nonce: Set(nonce),
                created_at: Set(Utc::now()),
            }
            .insert(&database)
            .await?;
            Some(hash)
        } else {
            None
        };

        let invoice_hash = if create_invoice {
            let address = "test_address".to_string();

            let user_data: Vec<u8> = vec![];
            let (nonce, ciphertext) = ek_custodian.encrypt(&serde_json::to_vec(&user_data)?)?;

            let mut hasher = Sha3_256::new();
            hasher.update(&address);
            let hash = Hash::try_from(hasher.finalize().to_vec())?;

            invoice::ActiveModel {
                hash: Set(hash.clone()),
                merchant_id: Set(t.merchant_id.clone()),
                wallet_hash: Set(wallet_hash.clone().expect("Invalid wallet hash")),
                ticker: Set(Ticker::BTC.into()),
                currency: Set(Currency::BTC.into()),
                network: Set(Network::Testnet.into()),
                invoice_status: Set(invoice::InvoiceStatus::Pending),
                pubkey: Set("12345".to_string()),
                address: Set(address.clone()),
                amount: Set(0.00001000),
                uri: Set(format!("bitcoin:{}", address)),
                encryption_key_id: Set(ek_custodian.id()),
                cipher: Set(Cipher::Aes256GcmSiv),
                blob: Set(ciphertext),
                nonce: Set(nonce),
                created_at: Set(Utc::now()),
                updated_at: Set(Utc::now()),
                expires_at: Set(Utc::now() + Duration::seconds(10)),
            }
            .insert(&database)
            .await?;
            Some(hash)
        } else {
            None
        };

        let bitcoin_gateway_config = BitcoinRpcConfig {
            endpoint: "http://localhost:18443".to_string(),
            basic_auth: None,
        };
        let rpc = SaleRpcImpl {
            master_merchant_id: Arc::new(t.merchant_id.clone()),
            kek_custodian,
            database,
            bitcoin_gateway_config,
        }
        .into_rpc();
        Ok((t.merchant_id, wallet_hash, invoice_hash, rpc))
    }

    #[tokio::test]
    async fn test_sale_invoice_ok() {
        let (merchant_id, wallet_hash, _, rpc) = test_rpc(true, false)
            .await
            .expect("Failed to create RpcModule<SaleRpcImpl>");
        let wallet_hash = wallet_hash.expect("Invalid wallet hash");
        let result = rpc
            .raw_json_request(
                &serde_json::to_string(&json!({
                    "jsonrpc": "2.0",
                    "method": "sale.invoice",
                    "params": {
                        "merchant_id": merchant_id,
                        "request": {
                            "hash": wallet_hash.to_string(),
                            "uuid": "12345",
                            "currency": "BTC",
                            "amount": 0.00001000,
                        },
                    },
                    "id": "12345",
                }))
                .expect("Invalid request"),
            )
            .await;
        assert!(result.is_ok());
        let (resp, _) = result.expect("Invalid response");
        let json_rpc: serde_json::Value =
            serde_json::from_str(&resp).expect("Invalid json response");
        assert_eq!(json_rpc["error"], serde_json::Value::Null);
        assert_ne!(json_rpc["result"]["hash"], serde_json::Value::Null);
        assert_eq!(
            json_rpc["result"]["ticker"],
            serde_json::Value::String("BTC".to_string())
        );
        assert_eq!(json_rpc["result"]["amount"], 0.00001000,);
        assert_eq!(
            json_rpc["result"]["address"],
            serde_json::Value::String("test_address".to_string())
        );
    }

    #[tokio::test]
    async fn test_sale_invoice_not_ok() {
        let (merchant_id, _, _, rpc) = test_rpc(false, false)
            .await
            .expect("Failed to create RpcModule<SaleRpcImpl>");

        let result = rpc
            .raw_json_request(
                &serde_json::to_string(&json!({
                    "jsonrpc": "2.0",
                    "method": "sale.invoice",
                    "params": {
                        "merchant_id": merchant_id,
                        "request": "Invalid",
                    },
                    "id": "12345",
                }))
                .expect("Invalid request"),
            )
            .await;
        assert!(result.is_ok());
        let (resp, _) = result.expect("Invalid response");
        let json_rpc: serde_json::Value =
            serde_json::from_str(&resp).expect("Invalid json response");
        assert_eq!(json_rpc["result"], serde_json::Value::Null);
        assert_eq!(
            json_rpc["error"],
            json!({"code": -32602, "message": "invalid type: string \"Invalid\", expected struct SaleInvoiceRequest at line 1 column 69"})
        );
    }

    #[tokio::test]
    async fn test_sale_invoice_lookup_ok() {
        let (merchant_id, wallet_hash, _, rpc) = test_rpc(true, false)
            .await
            .expect("Failed to create RpcModule<SaleRpcImpl>");
        let wallet_hash = wallet_hash.expect("Invalid wallet hash");
        let result = rpc
            .raw_json_request(
                &serde_json::to_string(&json!({
                    "jsonrpc": "2.0",
                    "method": "sale.invoice",
                    "params": {
                        "merchant_id": merchant_id,
                        "request": {
                            "hash": wallet_hash.to_string(),
                            "uuid": "12345",
                            "currency": "BTC",
                            "amount": 0.00001000,
                        },
                    },
                    "id": "12345",
                }))
                .expect("Invalid request"),
            )
            .await;
        assert!(result.is_ok());
        let (resp, _) = result.expect("Invalid response");
        let json_rpc: serde_json::Value =
            serde_json::from_str(&resp).expect("Invalid json response");
        assert_eq!(json_rpc["error"], serde_json::Value::Null);

        let result = rpc
            .raw_json_request(
                &serde_json::to_string(&json!({
                    "jsonrpc": "2.0",
                    "method": "sale.invoiceLookup",
                    "params": {
                        "merchant_id": merchant_id,
                        "request": {
                            "hash": json_rpc["result"]["hash"],
                        },
                    },
                    "id": "12345",
                }))
                .expect("Invalid request"),
            )
            .await;
        assert!(result.is_ok());
        let (resp, _) = result.expect("Invalid response");
        let json_rpc: serde_json::Value =
            serde_json::from_str(&resp).expect("Invalid json response");
        assert_eq!(json_rpc["error"], serde_json::Value::Null);
        assert_ne!(json_rpc["result"]["hash"], serde_json::Value::Null);
        assert_eq!(
            json_rpc["result"]["ticker"],
            serde_json::Value::String("BTC".to_string())
        );
        assert_eq!(json_rpc["result"]["amount"], 0.00001000,);
        assert_eq!(
            json_rpc["result"]["address"],
            serde_json::Value::String("test_address".to_string())
        );
    }

    #[tokio::test]
    async fn test_sale_invoice_lookup_not_ok() {
        let (merchant_id, _, _, rpc) = test_rpc(false, false)
            .await
            .expect("Failed to create RpcModule<SaleRpcImpl>");

        let result = rpc
            .raw_json_request(
                &serde_json::to_string(&json!({
                    "jsonrpc": "2.0",
                    "method": "sale.invoiceLookup",
                    "params": {
                        "merchant_id": merchant_id,
                        "request": {
                            "hash": "not_found",
                        },
                    },
                    "id": "12345",
                }))
                .expect("Invalid request"),
            )
            .await;
        assert!(result.is_ok());
        let (resp, _) = result.expect("Invalid response");
        let json_rpc: serde_json::Value =
            serde_json::from_str(&resp).expect("Invalid json response");
        assert_eq!(json_rpc["error"], serde_json::Value::Null);
        assert_eq!(json_rpc["result"], serde_json::Value::Null);
    }

    #[tokio::test]
    async fn test_sale_capture_ok() {
        let (merchant_id, _, invoice_hash, rpc) = test_rpc(true, true)
            .await
            .expect("Failed to create RpcModule<SaleRpcImpl>");
        let invoice_hash = invoice_hash.expect("Invalid wallet hash");

        let result = rpc
            .raw_json_request(
                &serde_json::to_string(&json!({
                    "jsonrpc": "2.0",
                    "method": "sale.capture",
                    "params": {
                        "merchant_id": merchant_id,
                        "request": {
                            "hash": invoice_hash.to_string(),
                            "uuid": "12345",
                            "amount": 0.00001000,
                        },
                    },
                    "id": "12345",
                }))
                .expect("Invalid request"),
            )
            .await;
        assert!(result.is_ok());
        let (resp, _) = result.expect("Invalid response");
        let json_rpc: serde_json::Value =
            serde_json::from_str(&resp).expect("Invalid json response");
        assert_eq!(json_rpc["error"], serde_json::Value::Null);
        assert_ne!(json_rpc["result"]["hash"], serde_json::Value::Null);
        assert_eq!(
            json_rpc["result"]["ticker"],
            serde_json::Value::String("BTC".to_string())
        );
        assert_eq!(json_rpc["result"]["amount"], 0.00001000,);
        assert_eq!(
            json_rpc["result"]["address"],
            serde_json::Value::String("test_address".to_string())
        );

        let result = rpc
            .raw_json_request(
                &serde_json::to_string(&json!({
                    "jsonrpc": "2.0",
                    "method": "sale.capture",
                    "params": {
                        "merchant_id": merchant_id,
                        "request": {
                            "hash": invoice_hash.to_string(),
                            "uuid": "12345",
                            "amount": 0.00001000,
                        },
                    },
                    "id": "12345",
                }))
                .expect("Invalid request"),
            )
            .await;
        assert!(result.is_ok());
        let (resp, _) = result.expect("Invalid response");
        let json_rpc: serde_json::Value =
            serde_json::from_str(&resp).expect("Invalid json response");
        assert_eq!(json_rpc["error"], serde_json::Value::Null);
        assert_ne!(json_rpc["result"]["hash"], serde_json::Value::Null);
    }

    #[tokio::test]
    async fn test_sale_capture_not_ok() {
        let (merchant_id, _, _, rpc) = test_rpc(false, false)
            .await
            .expect("Failed to create RpcModule<SaleRpcImpl>");

        let result = rpc
            .raw_json_request(
                &serde_json::to_string(&json!({
                    "jsonrpc": "2.0",
                    "method": "sale.capture",
                    "params": {
                        "merchant_id": merchant_id,
                        "request": "Invalid",
                    },
                    "id": "12345",
                }))
                .expect("Invalid request"),
            )
            .await;
        assert!(result.is_ok());
        let (resp, _) = result.expect("Invalid response");
        let json_rpc: serde_json::Value =
            serde_json::from_str(&resp).expect("Invalid json response");
        assert_eq!(json_rpc["result"], serde_json::Value::Null);
        assert_eq!(
            json_rpc["error"],
            json!({"code": -32602, "message": "invalid type: string \"Invalid\", expected struct SaleCaptureRequest at line 1 column 69"})
        );
    }

    #[tokio::test]
    async fn test_sale_lookup_ok() {
        let (merchant_id, _, invoice_hash, rpc) = test_rpc(true, true)
            .await
            .expect("Failed to create RpcModule<SaleRpcImpl>");
        let invoice_hash = invoice_hash.expect("Invalid wallet hash");

        let result = rpc
            .raw_json_request(
                &serde_json::to_string(&json!({
                    "jsonrpc": "2.0",
                    "method": "sale.capture",
                    "params": {
                        "merchant_id": merchant_id,
                        "request": {
                            "hash": invoice_hash.to_string(),
                            "uuid": "12345",
                            "amount": 0.00001000,
                        },
                    },
                    "id": "12345",
                }))
                .expect("Invalid request"),
            )
            .await;
        assert!(result.is_ok());
        let (resp, _) = result.expect("Invalid response");
        let json_rpc: serde_json::Value =
            serde_json::from_str(&resp).expect("Invalid json response");
        assert_eq!(json_rpc["error"], serde_json::Value::Null);

        let result = rpc
            .raw_json_request(
                &serde_json::to_string(&json!({
                    "jsonrpc": "2.0",
                    "method": "sale.lookup",
                    "params": {
                        "merchant_id": merchant_id,
                        "request": {
                            "hash": json_rpc["result"]["hash"],
                        },
                    },
                    "id": "12345",
                }))
                .expect("Invalid request"),
            )
            .await;
        assert!(result.is_ok());
        let (resp, _) = result.expect("Invalid response");
        let json_rpc: serde_json::Value =
            serde_json::from_str(&resp).expect("Invalid json response");
        assert_eq!(json_rpc["error"], serde_json::Value::Null);
        assert_ne!(json_rpc["result"]["hash"], serde_json::Value::Null);
        assert_eq!(
            json_rpc["result"]["ticker"],
            serde_json::Value::String("BTC".to_string())
        );
        assert_eq!(json_rpc["result"]["amount"], 0.00001000,);
        assert_eq!(
            json_rpc["result"]["address"],
            serde_json::Value::String("test_address".to_string())
        );
    }

    #[tokio::test]
    async fn test_sale_lookup_not_ok() {
        let (merchant_id, _, _, rpc) = test_rpc(false, false)
            .await
            .expect("Failed to create RpcModule<SaleRpcImpl>");

        let result = rpc
            .raw_json_request(
                &serde_json::to_string(&json!({
                    "jsonrpc": "2.0",
                    "method": "sale.lookup",
                    "params": {
                        "merchant_id": merchant_id,
                        "request": {
                            "hash": "not_found",
                        },
                    },
                    "id": "12345",
                }))
                .expect("Invalid request"),
            )
            .await;
        assert!(result.is_ok());
        let (resp, _) = result.expect("Invalid response");
        let json_rpc: serde_json::Value =
            serde_json::from_str(&resp).expect("Invalid json response");
        assert_eq!(json_rpc["error"], serde_json::Value::Null);
        assert_eq!(json_rpc["result"], serde_json::Value::Null);
    }
}
