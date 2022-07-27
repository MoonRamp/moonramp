use std::sync::Arc;

use anyhow::anyhow;
use async_trait::async_trait;
use chrono::Utc;
use jsonrpsee::{core::RpcResult, proc_macros::rpc, RpcModule};
use log::debug;
use sea_orm::{entity::*, query::*, DatabaseConnection};
use sha3::{Digest, Sha3_256};
use tokio::sync::{mpsc, RwLock};

use moonramp_core::{
    anyhow, async_trait, chrono, log, sea_orm, sha3, tokio, Hash, NetworkTunnelReceiver,
    NetworkTunnelSender, NodeId, TunnelName,
};
use moonramp_encryption::{
    EncryptionKeyCustodian, KeyCustodian, KeyEncryptionKeyCustodian, MerchantScopedSecret,
};
use moonramp_entity::{cipher::Cipher, encryption_key, program};
use moonramp_program::Runtime;
use moonramp_rpc::{IntoRpcResult, RpcService};

use crate::params::*;

#[rpc(server)]
pub trait ProgramRpc {
    #[method(name = "program.version")]
    fn version(&self) -> RpcResult<String>;

    #[method(name = "program.create")]
    async fn create(
        &self,
        merchant_id: String,
        request: ProgramCreateRequest,
    ) -> RpcResult<ProgramResponse>;

    #[method(name = "program.update")]
    async fn update(
        &self,
        merchant_id: String,
        request: ProgramUpdateRequest,
    ) -> RpcResult<ProgramResponse>;

    #[method(name = "program.lookup")]
    async fn lookup(
        &self,
        merchant_id: String,
        request: ProgramLookupRequest,
    ) -> RpcResult<Option<ProgramResponse>>;
}

#[derive(Clone)]
pub struct ProgramRpcImpl {
    kek_custodian: Arc<KeyEncryptionKeyCustodian>,
    database: DatabaseConnection,
}

#[async_trait]
impl ProgramRpcServer for ProgramRpcImpl {
    fn version(&self) -> RpcResult<String> {
        Ok(env!("CARGO_PKG_VERSION").to_string())
    }

    async fn create(
        &self,
        merchant_id: String,
        request: ProgramCreateRequest,
    ) -> RpcResult<ProgramResponse> {
        debug!("program.create {:?}", request);

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
            Cipher::ChaCha20Poly1305,
        )
        .into_rpc_result()?;

        let mut hasher = Sha3_256::new();
        hasher.update(&request.data);
        let hash = Hash::try_from(hasher.finalize().to_vec()).into_rpc_result()?;

        let wasm_mod_bytes = Runtime::compile(&request.data)?;
        let (nonce, ciphertext) = ek_custodian.encrypt(&wasm_mod_bytes).into_rpc_result()?;

        Ok(program::ActiveModel {
            hash: Set(hash),
            merchant_id: Set(merchant_id),
            name: Set(request.name),
            version: Set(request.version),
            url: Set(request.url),
            description: Set(request.description),
            private: Set(request.private),
            revision: Set(0),
            encryption_key_id: Set(ek_custodian.id()),
            cipher: Set(Cipher::ChaCha20Poly1305),
            blob: Set(ciphertext),
            nonce: Set(nonce),
            created_at: Set(Utc::now()),
        }
        .insert(&self.database)
        .await
        .into_rpc_result()?
        .into())
    }

    async fn update(
        &self,
        merchant_id: String,
        request: ProgramUpdateRequest,
    ) -> RpcResult<ProgramResponse> {
        debug!("program.update {:?}", request);
        let p = program::Entity::find()
            .filter(
                Condition::all()
                    .add(program::Column::Name.eq(request.name))
                    .add(program::Column::MerchantId.eq(merchant_id.clone())),
            )
            .order_by_desc(program::Column::Revision)
            .one(&self.database)
            .await
            .into_rpc_result()?
            .ok_or(anyhow!("Failed find program"))
            .into_rpc_result()?;

        let ek = encryption_key::Entity::find()
            .filter(
                Condition::all()
                    .add(encryption_key::Column::Id.eq(p.encryption_key_id.clone()))
                    .add(encryption_key::Column::MerchantId.eq(merchant_id.clone()))
                    .add(encryption_key::Column::KeyEncryptionKeyId.eq(self.kek_custodian.id())),
            )
            .one(&self.database)
            .await
            .into_rpc_result()?
            .ok_or(anyhow!("Failed load program"))
            .into_rpc_result()?;

        let ek_custodian = EncryptionKeyCustodian::new(
            self.kek_custodian
                .unlock(ek)
                .into_rpc_result()?
                .secret
                .to_vec(),
            Cipher::ChaCha20Poly1305,
        )
        .into_rpc_result()?;

        let mut hasher = Sha3_256::new();
        hasher.update(&request.data);
        let hash = Hash::try_from(hasher.finalize().to_vec()).into_rpc_result()?;

        let wasm_mod_bytes = Runtime::compile(&request.data)?;
        let (nonce, ciphertext) = ek_custodian.encrypt(&wasm_mod_bytes).into_rpc_result()?;

        Ok(program::ActiveModel {
            hash: Set(hash),
            merchant_id: Set(merchant_id),
            name: Set(p.name),
            version: Set(request.version),
            url: Set(request.url),
            description: Set(request.description),
            private: Set(p.private),
            revision: Set(p.revision + 1),
            encryption_key_id: Set(ek_custodian.id()),
            cipher: Set(Cipher::ChaCha20Poly1305),
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
        request: ProgramLookupRequest,
    ) -> RpcResult<Option<ProgramResponse>> {
        debug!("program.lookup {:?}", request);

        Ok(match request {
            ProgramLookupRequest::Hash { hash } => program::Entity::find()
                .filter(
                    Condition::all()
                        .add(program::Column::Hash.eq(hash))
                        .add(program::Column::MerchantId.eq(merchant_id.clone())),
                )
                .one(&self.database)
                .await
                .into_rpc_result()?
                .map(|p| p.into()),
            ProgramLookupRequest::Name { name } => program::Entity::find()
                .filter(
                    Condition::all()
                        .add(program::Column::Name.eq(name))
                        .add(program::Column::MerchantId.eq(merchant_id.clone())),
                )
                .order_by_desc(program::Column::Revision)
                .one(&self.database)
                .await
                .into_rpc_result()?
                .map(|p| p.into()),
        })
    }
}

pub struct ProgramRpcService {
    node_id: NodeId,
    rx: Arc<RwLock<NetworkTunnelReceiver>>,
    rpc: RpcModule<ProgramRpcImpl>,
}

impl ProgramRpcService {
    pub fn new(
        node_id: NodeId,
        kek_custodian: Arc<KeyEncryptionKeyCustodian>,
        database: DatabaseConnection,
    ) -> anyhow::Result<(NetworkTunnelSender, Arc<Self>)> {
        let (public_tx, public_network_rx) = mpsc::channel(1024);

        // Program Rpc
        let rpc = ProgramRpcImpl {
            kek_custodian,
            database,
        }
        .into_rpc();

        Ok((
            public_tx,
            Arc::new(ProgramRpcService {
                node_id,
                rx: Arc::new(RwLock::new(public_network_rx)),
                rpc,
            }),
        ))
    }
}

#[async_trait]
impl RpcService<ProgramRpcImpl> for ProgramRpcService {
    fn log_target(&self) -> String {
        "moonramp_program::rpc".to_string()
    }

    fn node_id(&self) -> NodeId {
        self.node_id.clone()
    }

    fn service_name(&self) -> TunnelName {
        TunnelName::Program
    }

    fn rx(&self) -> Arc<RwLock<NetworkTunnelReceiver>> {
        self.rx.clone()
    }

    fn rpc(&self) -> RpcModule<ProgramRpcImpl> {
        self.rpc.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::Database;
    use serde_json::json;

    use moonramp_core::serde_json;
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

    async fn test_rpc() -> anyhow::Result<(String, RpcModule<ProgramRpcImpl>)> {
        let database = Database::connect("sqlite::memory:")
            .await
            .expect("Failed to open in-memory sqlite db");
        let t = setup_testdb(&database)
            .await
            .expect("Failed to setup testdb");
        let kek_custodian = test_kek(&database)
            .await
            .expect("Invalid KeyEncryptionKeyCustodian");

        let rpc = ProgramRpcImpl {
            kek_custodian,
            database,
        }
        .into_rpc();
        Ok((t.merchant_id, rpc))
    }

    #[tokio::test]
    async fn test_program_create_ok() {
        let (merchant_id, rpc) = test_rpc()
            .await
            .expect("Failed to create RpcModule<ProgramRpcImpl>");

        let data = br#"
            (module
               (func $hello (import "" "hello"))
                 (func (export "run") (call $hello))
            )
        "#;
        let result = rpc
            .raw_json_request(
                &serde_json::to_string(&json!({
                    "jsonrpc": "2.0",
                    "method": "program.create",
                    "params": {
                        "merchant_id": merchant_id,
                        "request": {
                            "name": "test",
                            "version": "0.1.0",
                            "data": data.to_vec(),
                            "private": true,
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
            json_rpc["result"]["name"],
            serde_json::Value::String("test".to_string()),
        );
        assert_eq!(
            json_rpc["result"]["version"],
            serde_json::Value::String("0.1.0".to_string()),
        );
        assert_eq!(
            json_rpc["result"]["revision"],
            serde_json::Value::Number(0.into()),
        );
    }

    #[tokio::test]
    async fn test_program_create_not_ok() {
        let (merchant_id, rpc) = test_rpc()
            .await
            .expect("Failed to create RpcModule<ProgramRpcImpl>");

        let result = rpc
            .raw_json_request(
                &serde_json::to_string(&json!({
                    "jsonrpc": "2.0",
                    "method": "program.create",
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
            json!({"code": -32602, "message": "invalid type: string \"Invalid\", expected struct ProgramCreateRequest at line 1 column 69"})
        );
    }

    #[tokio::test]
    async fn test_program_lookup_ok() {
        let (merchant_id, rpc) = test_rpc()
            .await
            .expect("Failed to create RpcModule<ProgramRpcImpl>");
        let data = br#"
            (module
               (func $hello (import "" "hello"))
                 (func (export "run") (call $hello))
            )
        "#;
        let result = rpc
            .raw_json_request(
                &serde_json::to_string(&json!({
                    "jsonrpc": "2.0",
                    "method": "program.create",
                    "params": {
                        "merchant_id": merchant_id,
                        "request": {
                            "name": "test",
                            "version": "0.1.0",
                            "data": data.to_vec(),
                            "private": true,
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
                    "method": "program.lookup",
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
            json_rpc["result"]["name"],
            serde_json::Value::String("test".to_string()),
        );
        assert_eq!(
            json_rpc["result"]["version"],
            serde_json::Value::String("0.1.0".to_string()),
        );
        assert_eq!(
            json_rpc["result"]["revision"],
            serde_json::Value::Number(0.into()),
        );
    }

    #[tokio::test]
    async fn test_program_lookup_not_ok() {
        let (merchant_id, rpc) = test_rpc()
            .await
            .expect("Failed to create RpcModule<ProgramRpcImpl>");

        let result = rpc
            .raw_json_request(
                &serde_json::to_string(&json!({
                    "jsonrpc": "2.0",
                    "method": "program.lookup",
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
