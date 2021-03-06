use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use jsonrpsee::{core::RpcResult, proc_macros::rpc, RpcModule};
use log::debug;
use sea_orm::{entity::*, query::*, DatabaseConnection};
use sha3::{Digest, Sha3_256};
use tokio::sync::{mpsc, RwLock};

use moonramp_core::{
    anyhow, async_trait, chrono, log, sea_orm, serde_json, sha3, tokio, Hash, NetworkTunnel,
    NetworkTunnelReceiver, NetworkTunnelSender, NodeId, TunnelName,
};
use moonramp_encryption::{
    EncryptionKeyCustodian, KeyCustodian, KeyEncryptionKeyCustodian, MerchantScopedSecret,
};
use moonramp_entity::{cipher::Cipher, wallet};
use moonramp_rpc::{IntoRpcResult, RpcService};

use crate::{params::*, BitcoinWallet, Network, Ticker, Wallet};

#[rpc(server)]
pub trait WalletRpc {
    #[method(name = "wallet.version")]
    fn version(&self) -> RpcResult<String>;

    #[method(name = "wallet.create")]
    async fn create(
        &self,
        merchant_id: String,
        request: WalletCreateRequest,
    ) -> RpcResult<WalletResponse>;

    #[method(name = "wallet.lookup")]
    async fn lookup(
        &self,
        merchant_id: String,
        request: WalletLookupRequest,
    ) -> RpcResult<Option<WalletResponse>>;
}

#[derive(Clone)]
pub struct WalletRpcImpl {
    kek_custodian: Arc<KeyEncryptionKeyCustodian>,
    database: DatabaseConnection,
    #[allow(dead_code)]
    internal_tx: NetworkTunnelSender,
    network: Network,
}

#[async_trait]
impl WalletRpcServer for WalletRpcImpl {
    fn version(&self) -> RpcResult<String> {
        Ok(env!("CARGO_PKG_VERSION").to_string())
    }

    async fn create(
        &self,
        merchant_id: String,
        request: WalletCreateRequest,
    ) -> RpcResult<WalletResponse> {
        debug!("wallet.create {:?}", request);

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

        let network = self.network.clone();
        let w = match request {
            WalletCreateRequest::BtcHot => {
                Wallet::Bitcoin(BitcoinWallet::new_hot(Ticker::BTC, network).into_rpc_result()?)
            }
            WalletCreateRequest::BtcCold { pubkey, cold_type } => Wallet::Bitcoin(
                BitcoinWallet::new_cold(Ticker::BTC, network, pubkey, cold_type)
                    .into_rpc_result()?,
            ),
            WalletCreateRequest::BchHot => {
                Wallet::Bitcoin(BitcoinWallet::new_hot(Ticker::BCH, network).into_rpc_result()?)
            }
            WalletCreateRequest::BchCold { pubkey, cold_type } => Wallet::Bitcoin(
                BitcoinWallet::new_cold(Ticker::BCH, network, pubkey, cold_type)
                    .into_rpc_result()?,
            ),
        };

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
            .encrypt(&serde_json::to_vec(&w).into_rpc_result()?)
            .into_rpc_result()?;

        let mut hasher = Sha3_256::new();
        hasher.update(w.pubkey().as_bytes());
        let hash = Hash::try_from(hasher.finalize().to_vec()).into_rpc_result()?;

        Ok(wallet::ActiveModel {
            hash: Set(hash),
            merchant_id: Set(merchant_id),
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
        .insert(&self.database)
        .await
        .into_rpc_result()?
        .into())
    }

    async fn lookup(
        &self,
        merchant_id: String,
        request: WalletLookupRequest,
    ) -> RpcResult<Option<WalletResponse>> {
        debug!("wallet.lookup {:?}", request);

        Ok(match request {
            WalletLookupRequest::Hash { hash } => wallet::Entity::find()
                .filter(
                    Condition::all()
                        .add(wallet::Column::Hash.eq(hash))
                        .add(wallet::Column::MerchantId.eq(merchant_id.clone())),
                )
                .one(&self.database)
                .await
                .into_rpc_result()?
                .map(|w| w.into()),
            WalletLookupRequest::Pubkey { pubkey } => wallet::Entity::find()
                .filter(
                    Condition::all()
                        .add(wallet::Column::Pubkey.eq(pubkey))
                        .add(wallet::Column::MerchantId.eq(merchant_id.clone())),
                )
                .one(&self.database)
                .await
                .into_rpc_result()?
                .map(|w| w.into()),
        })
    }
}

pub struct WalletRpcService {
    node_id: NodeId,
    rx: Arc<
        RwLock<(
            NetworkTunnelReceiver,
            mpsc::Receiver<NetworkTunnel>,
            NetworkTunnelReceiver,
        )>,
    >,
    private_network_tx: mpsc::Sender<NetworkTunnel>,
    rpc: RpcModule<WalletRpcImpl>,
}

impl WalletRpcService {
    pub fn new(
        node_id: NodeId,
        kek_custodian: Arc<KeyEncryptionKeyCustodian>,
        database: DatabaseConnection,
        private_network_tx: mpsc::Sender<NetworkTunnel>,
    ) -> anyhow::Result<(mpsc::Sender<NetworkTunnel>, NetworkTunnelSender, Arc<Self>)> {
        let (internal_tx, internal_rx) = mpsc::channel(32);
        let (private_tx, private_network_rx) = mpsc::channel(1024);
        let (public_tx, public_network_rx) = mpsc::channel(1024);

        // Wallet Rpc
        let rpc = WalletRpcImpl {
            kek_custodian,
            database,
            internal_tx,
            network: Network::Regtest,
        }
        .into_rpc();

        Ok((
            private_tx,
            public_tx,
            Arc::new(WalletRpcService {
                node_id,
                rx: Arc::new(RwLock::new((
                    internal_rx,
                    private_network_rx,
                    public_network_rx,
                ))),
                private_network_tx,
                rpc,
            }),
        ))
    }
}

#[async_trait]
impl RpcService<WalletRpcImpl> for WalletRpcService {
    fn log_target(&self) -> String {
        "moonramp_wallet_rpc::rpc".to_string()
    }

    fn node_id(&self) -> NodeId {
        self.node_id.clone()
    }

    fn service_name(&self) -> TunnelName {
        TunnelName::Wallet
    }

    fn private_network_tx(&self) -> mpsc::Sender<NetworkTunnel> {
        self.private_network_tx.clone()
    }

    fn rx(
        &self,
    ) -> Arc<
        RwLock<(
            NetworkTunnelReceiver,
            mpsc::Receiver<NetworkTunnel>,
            NetworkTunnelReceiver,
        )>,
    > {
        self.rx.clone()
    }

    fn rpc(&self) -> RpcModule<WalletRpcImpl> {
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

    async fn test_rpc() -> anyhow::Result<(String, RpcModule<WalletRpcImpl>)> {
        let database = Database::connect("sqlite::memory:")
            .await
            .expect("Failed to open in-memory sqlite db");
        let t = setup_testdb(&database)
            .await
            .expect("Failed to setup testdb");
        let kek_custodian = test_kek(&database)
            .await
            .expect("Invalid KeyEncryptionKeyCustodian");
        let (internal_tx, _internal_rx) = mpsc::channel(1);

        let rpc = WalletRpcImpl {
            kek_custodian,
            database,
            internal_tx,
            network: Network::Testnet,
        }
        .into_rpc();
        Ok((t.merchant_id, rpc))
    }

    #[tokio::test]
    async fn test_wallet_create_ok() {
        let (merchant_id, rpc) = test_rpc()
            .await
            .expect("Failed to create RpcModule<WalletRpcImpl>");
        let result = rpc
            .raw_json_request(
                &serde_json::to_string(&json!({
                    "jsonrpc": "2.0",
                    "method": "wallet.create",
                    "params": {
                        "merchant_id": merchant_id,
                        "request": "btcHot",
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
        assert_ne!(json_rpc["result"]["pubkey"], serde_json::Value::Null);
        assert_eq!(
            json_rpc["result"]["ticker"],
            serde_json::Value::String("BTC".to_string())
        );
        assert_eq!(
            json_rpc["result"]["walletType"],
            serde_json::Value::String("Hot".to_string())
        );

        let result = rpc
            .raw_json_request(
                &serde_json::to_string(&json!({
                    "jsonrpc": "2.0",
                    "method": "wallet.create",
                    "params": {
                        "merchant_id": merchant_id,
                        "request": {
                            "btcCold": {
                                "pubkey": "new_pubkey",
                                "coldType": "XPUBKEY",
                            },
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
        assert_eq!(
            json_rpc["result"]["hash"],
            serde_json::Value::String("7NcdhjfT5KdoZZBBX3f9Ne5DNU9L8LqVX9G9CtdiNdaq".to_string())
        );
        assert_eq!(
            json_rpc["result"]["pubkey"],
            serde_json::Value::String("new_pubkey".to_string())
        );
        assert_eq!(
            json_rpc["result"]["ticker"],
            serde_json::Value::String("BTC".to_string())
        );
        assert_eq!(
            json_rpc["result"]["walletType"],
            serde_json::Value::String("Cold".to_string())
        );
    }

    #[tokio::test]
    async fn test_wallet_create_not_ok() {
        let (merchant_id, rpc) = test_rpc()
            .await
            .expect("Failed to create RpcModule<WalletRpcImpl>");

        let result = rpc
            .raw_json_request(
                &serde_json::to_string(&json!({
                    "jsonrpc": "2.0",
                    "method": "wallet.create",
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
            json!({"code": -32602, "message": "unknown variant `Invalid`, expected one of `btcHot`, `btcCold`, `bchHot`, `bchCold` at line 1 column 69"})
        );
    }

    #[tokio::test]
    async fn test_wallet_lookup_ok() {
        let (merchant_id, rpc) = test_rpc()
            .await
            .expect("Failed to create RpcModule<WalletRpcImpl>");
        let result = rpc
            .raw_json_request(
                &serde_json::to_string(&json!({
                    "jsonrpc": "2.0",
                    "method": "wallet.create",
                    "params": {
                        "merchant_id": merchant_id,
                        "request": "btcHot",
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
                    "method": "wallet.lookup",
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
        assert_ne!(json_rpc["result"]["pubkey"], serde_json::Value::Null);
        assert_eq!(
            json_rpc["result"]["ticker"],
            serde_json::Value::String("BTC".to_string())
        );
        assert_eq!(
            json_rpc["result"]["walletType"],
            serde_json::Value::String("Hot".to_string())
        );
    }

    #[tokio::test]
    async fn test_wallet_lookup_not_ok() {
        let (merchant_id, rpc) = test_rpc()
            .await
            .expect("Failed to create RpcModule<WalletRpcImpl>");

        let result = rpc
            .raw_json_request(
                &serde_json::to_string(&json!({
                    "jsonrpc": "2.0",
                    "method": "wallet.lookup",
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
