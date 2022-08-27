use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use jsonrpsee::{core::RpcResult, proc_macros::rpc, RpcModule};
use log::debug;
use sea_orm::{entity::*, query::*, DatabaseConnection};
use sha3::{Digest, Sha3_256};
use tokio::sync::{mpsc, RwLock};

use moonramp_core::{
    anyhow, async_trait, chrono, log, sea_orm, serde_json, sha3, tokio, Hash,
    NetworkTunnelReceiver, NetworkTunnelSender, NodeId, TunnelName,
};
use moonramp_encryption::{
    EncryptionKeyCustodian, KeyCustodian, KeyEncryptionKeyCustodian, MerchantScopedSecret,
};
use moonramp_entity::{cipher::Cipher, wallet};
use moonramp_rpc::{IntoRpcResult, RpcService};

use crate::{params::*, BitcoinWallet, MoneroWallet, Network, Ticker, Wallet};

#[rpc(server)]
pub trait WalletRpc {
    #[method(name = "wallet.version")]
    fn version(&self) -> RpcResult<String>;

    #[method(name = "wallet.create")]
    async fn create(
        &self,
        merchant_hash: Hash,
        request: WalletCreateRequest,
    ) -> RpcResult<WalletResponse>;

    #[method(name = "wallet.lookup")]
    async fn lookup(
        &self,
        merchant_hash: Hash,
        request: WalletLookupRequest,
    ) -> RpcResult<Option<WalletResponse>>;
}

#[derive(Clone)]
pub struct WalletRpcImpl {
    kek_custodian: Arc<KeyEncryptionKeyCustodian>,
    database: DatabaseConnection,
    network: Network,
}

#[async_trait]
impl WalletRpcServer for WalletRpcImpl {
    fn version(&self) -> RpcResult<String> {
        Ok(env!("CARGO_PKG_VERSION").to_string())
    }

    async fn create(
        &self,
        merchant_hash: Hash,
        request: WalletCreateRequest,
    ) -> RpcResult<WalletResponse> {
        debug!("wallet.create {:?}", request);

        let ek = self
            .kek_custodian
            .lock(MerchantScopedSecret {
                merchant_hash: merchant_hash.clone(),
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
            WalletCreateRequest::XmrHot => {
                Wallet::Monero(MoneroWallet::new_hot(network).into_rpc_result()?)
            }
            WalletCreateRequest::XmrCold {
                view_key,
                cold_type,
            } => todo!(),
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
            merchant_hash: Set(merchant_hash),
            ticker: Set(w.ticker().into()),
            network: Set(w.network().into()),
            wallet_type: Set(w.wallet_type().into()),
            pubkey: Set(w.pubkey().to_string()),
            encryption_key_hash: Set(ek_custodian.hash()),
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
        merchant_hash: Hash,
        request: WalletLookupRequest,
    ) -> RpcResult<Option<WalletResponse>> {
        debug!("wallet.lookup {:?}", request);

        // TODO balance lookup

        Ok(match request {
            WalletLookupRequest::Hash { hash } => wallet::Entity::find()
                .filter(
                    Condition::all()
                        .add(wallet::Column::Hash.eq(hash))
                        .add(wallet::Column::MerchantHash.eq(merchant_hash.clone())),
                )
                .one(&self.database)
                .await
                .into_rpc_result()?
                .map(|w| w.into()),
            WalletLookupRequest::Pubkey { pubkey } => wallet::Entity::find()
                .filter(
                    Condition::all()
                        .add(wallet::Column::Pubkey.eq(pubkey))
                        .add(wallet::Column::MerchantHash.eq(merchant_hash.clone())),
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
    rx: Arc<RwLock<NetworkTunnelReceiver>>,
    rpc: RpcModule<WalletRpcImpl>,
}

impl WalletRpcService {
    pub fn new(
        node_id: NodeId,
        kek_custodian: Arc<KeyEncryptionKeyCustodian>,
        database: DatabaseConnection,
        _bitcoin_rpc_endpoint: String,
        _bitcoin_rpc_auth: String,
        network: Network,
    ) -> anyhow::Result<(NetworkTunnelSender, Arc<Self>)> {
        let (public_tx, public_network_rx) = mpsc::channel(1024);

        // Wallet Rpc
        let rpc = WalletRpcImpl {
            kek_custodian,
            database,
            network,
        }
        .into_rpc();

        Ok((
            public_tx,
            Arc::new(WalletRpcService {
                node_id,
                rx: Arc::new(RwLock::new(public_network_rx)),
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

    fn rx(&self) -> Arc<RwLock<NetworkTunnelReceiver>> {
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

    use moonramp_migration::testing::setup_testdb;

    async fn test_rpc() -> anyhow::Result<(Hash, RpcModule<WalletRpcImpl>)> {
        let database = Database::connect("sqlite::memory:")
            .await
            .expect("Failed to open in-memory sqlite db");
        let (kek_custodian, _, t) = setup_testdb(&database, "moonramp")
            .await
            .expect("Failed to setup testdb");

        let rpc = WalletRpcImpl {
            kek_custodian,
            database,
            network: Network::Regtest,
        }
        .into_rpc();
        Ok((t.merchant_hash, rpc))
    }

    #[tokio::test]
    async fn test_wallet_create_ok() {
        let (merchant_hash, rpc) = test_rpc()
            .await
            .expect("Failed to create RpcModule<WalletRpcImpl>");
        let result = rpc
            .raw_json_request(
                &serde_json::to_string(&json!({
                    "jsonrpc": "2.0",
                    "method": "wallet.create",
                    "params": {
                        "merchant_hash": merchant_hash,
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

        let w =
            BitcoinWallet::new_hot(Ticker::BTC, Network::Testnet).expect("Invalid BitcoinWallet");
        let mut hasher = Sha3_256::new();
        hasher.update(w.pubkey().as_bytes());
        let wallet_hash = Hash::try_from(hasher.finalize().to_vec()).expect("Invalid Hash");
        let result = rpc
            .raw_json_request(
                &serde_json::to_string(&json!({
                    "jsonrpc": "2.0",
                    "method": "wallet.create",
                    "params": {
                        "merchant_hash": merchant_hash,
                        "request": {
                            "btcCold": {
                                "pubkey": w.addr().expect("Invalid BitcoinWallet").0.to_string(),
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
            serde_json::Value::String(wallet_hash.to_string())
        );
        assert_eq!(
            json_rpc["result"]["pubkey"],
            serde_json::Value::String(w.pubkey())
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
        let (merchant_hash, rpc) = test_rpc()
            .await
            .expect("Failed to create RpcModule<WalletRpcImpl>");

        let result = rpc
            .raw_json_request(
                &serde_json::to_string(&json!({
                    "jsonrpc": "2.0",
                    "method": "wallet.create",
                    "params": {
                        "merchant_hash": merchant_hash,
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
            json!({"code": -32602, "message": "unknown variant `Invalid`, expected one of `btcHot`, `btcCold`, `bchHot`, `bchCold`, `xmrHot`, `xmrCold` at line 1 column 83"})
        );
    }

    #[tokio::test]
    async fn test_wallet_lookup_ok() {
        let (merchant_hash, rpc) = test_rpc()
            .await
            .expect("Failed to create RpcModule<WalletRpcImpl>");
        let result = rpc
            .raw_json_request(
                &serde_json::to_string(&json!({
                    "jsonrpc": "2.0",
                    "method": "wallet.create",
                    "params": {
                        "merchant_hash": merchant_hash,
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
                        "merchant_hash": merchant_hash,
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
        let (merchant_hash, rpc) = test_rpc()
            .await
            .expect("Failed to create RpcModule<WalletRpcImpl>");

        let result = rpc
            .raw_json_request(
                &serde_json::to_string(&json!({
                    "jsonrpc": "2.0",
                    "method": "wallet.lookup",
                    "params": {
                        "merchant_hash": merchant_hash,
                        "request": {
                            "hash": merchant_hash,
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
