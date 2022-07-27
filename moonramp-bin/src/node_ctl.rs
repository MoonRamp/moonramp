use std::sync::Arc;

use log::{debug, error};
use sea_orm::{entity::*, query::*, DatabaseConnection};

use moonramp_core::{anyhow, log, sea_orm, tokio, NodeId, TunnelName};
use moonramp_encryption::{
    KeyCustodian, KeyEncryptionKeyCustodian, MasterKeyEncryptionKeyCustodian,
};
use moonramp_entity::key_encryption_key;
use moonramp_rpc::RpcService;

pub struct NodeCtl {
    node_id: NodeId,
    kek_custodian: Arc<KeyEncryptionKeyCustodian>,
    database: DatabaseConnection,
    wallet_http_addr: String,
    sale_http_addr: String,
    program_http_addr: String,
    master_merchant_id: Arc<String>,
}

impl NodeCtl {
    pub async fn new(
        node_id: NodeId,
        program_http_addr: String,
        sale_http_addr: String,
        wallet_http_addr: String,
        master_merchant_id: String,
        master_key_encryption_key: Vec<u8>,
        db_url: String,
    ) -> anyhow::Result<Self> {
        let database = moonramp_entity::database_connection_pool(&db_url).await?;

        let kek_custodian = {
            let master_custodian = MasterKeyEncryptionKeyCustodian::new(master_key_encryption_key)?;
            let kek = if let Some(kek) = key_encryption_key::Entity::find()
                .filter(
                    key_encryption_key::Column::MasterKeyEncryptionKeyId.eq(master_custodian.id()),
                )
                .order_by_desc(key_encryption_key::Column::CreatedAt)
                .one(&database)
                .await?
            {
                kek
            } else {
                let secret = master_custodian.gen_secret()?;
                master_custodian.lock(secret)?.insert(&database).await?
            };
            Arc::new(KeyEncryptionKeyCustodian::new(
                master_custodian.unlock(kek)?.to_vec(),
            )?)
        };

        Ok(NodeCtl {
            node_id,
            kek_custodian,
            database,
            program_http_addr,
            sale_http_addr,
            wallet_http_addr,
            master_merchant_id: Arc::new(master_merchant_id),
        })
    }

    pub async fn run(&mut self) -> anyhow::Result<()> {
        // Internal registry network
        let (registry_tx, mut registry) = moonramp_registry::Registry::new();

        // Program
        debug!("Creating program service");
        let (program_public_tx, program_rpc_service) =
            moonramp_program_rpc::ProgramRpcService::new(
                self.node_id.clone(),
                self.kek_custodian.clone(),
                self.database.clone(),
            )?;
        registry.register(TunnelName::Program, program_public_tx);
        let program_http = moonramp_program_rpc::ProgramHttpServer::new(
            self.database.clone(),
            registry_tx.clone(),
            &self.program_http_addr,
        )
        .await?;

        // Sale
        debug!("Creating sale service");
        let (sale_public_tx, sale_rpc_service) = moonramp_sale_rpc::SaleRpcService::new(
            self.node_id.clone(),
            self.master_merchant_id.clone(),
            self.kek_custodian.clone(),
            self.database.clone(),
        )?;
        registry.register(TunnelName::Sale, sale_public_tx);
        let sale_http = moonramp_sale_rpc::SaleHttpServer::new(
            self.database.clone(),
            registry_tx.clone(),
            &self.sale_http_addr,
        )
        .await?;

        // Wallet
        debug!("Creating wallet service");
        let (wallet_public_tx, wallet_rpc_service) = moonramp_wallet_rpc::WalletRpcService::new(
            self.node_id.clone(),
            self.kek_custodian.clone(),
            self.database.clone(),
        )?;
        registry.register(TunnelName::Wallet, wallet_public_tx);
        let wallet_http = moonramp_wallet_rpc::WalletHttpServer::new(
            self.database.clone(),
            registry_tx.clone(),
            &self.wallet_http_addr,
        )
        .await?;

        debug!("Running services...");
        // Registry
        let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
        tokio::spawn(async move {
            let res = registry.run().await;
            error!("Registry exited {:?}", res);
        });

        // Program
        let program_shutdown_rx = shutdown_rx.clone();
        tokio::spawn(async move {
            let res = program_rpc_service.listen(program_shutdown_rx).await;
            error!("Program RPC exited {:?}", res);
        });
        tokio::spawn(async move {
            let res = program_http.listen().await;
            error!("Program HTTP exited {:?}", res);
        });
        // Sale
        let sale_shutdown_rx = shutdown_rx.clone();
        tokio::spawn(async move {
            let res = sale_rpc_service.listen(sale_shutdown_rx).await;
            error!("Sale RPC exited {:?}", res);
        });
        tokio::spawn(async move {
            let res = sale_http.listen().await;
            error!("Sale HTTP exited {:?}", res);
        });
        // Wallet
        let wallet_shutdown_rx = shutdown_rx.clone();
        tokio::spawn(async move {
            let res = wallet_rpc_service.listen(wallet_shutdown_rx).await;
            error!("Wallet RPC exited {:?}", res);
        });
        tokio::spawn(async move {
            let res = wallet_http.listen().await;
            error!("Wallet HTTP exited {:?}", res);
        });

        tokio::signal::ctrl_c().await?;
        shutdown_tx.send(true)?;
        Ok(())
    }
}
