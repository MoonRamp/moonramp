use std::{collections::HashMap, fmt::Display, sync::Arc};

use anyhow::anyhow;
use async_trait::async_trait;
use jsonrpsee::{
    core::{Error as RpcError, RpcResult},
    types::error::CallError,
    RpcModule,
};
use log::{debug, info, trace, warn};
use tokio::{
    sync::{mpsc, watch, RwLock},
    time::{interval, Duration, Instant},
};

use moonramp_core::{
    anyhow, async_trait, jsonrpsee, log, serde_json, tokio, NetworkTunnel, NetworkTunnelChannel,
    NetworkTunnelReceiver, NodeId, RpcTunnel, Sender, TunnelName, TunnelTopic,
};

mod egress;
mod ingress;
mod runner;

pub use egress::egress;
pub use ingress::ingress;
pub use runner::RpcRunner;

pub trait IntoRpcResult<T> {
    fn into_rpc_result(self) -> RpcResult<T>;
}

impl<T, E: Display> IntoRpcResult<T> for Result<T, E> {
    fn into_rpc_result(self) -> RpcResult<T> {
        self.map_err(|e| RpcError::Call(CallError::Failed(anyhow!("{}", e))))
    }
}

#[derive(Clone)]
pub struct RpcServiceState<R: 'static + Clone + Send + Sync> {
    log_target: Arc<String>,
    node_id: Arc<NodeId>,
    service_name: Arc<TunnelName>,
    liveliness: Arc<RwLock<HashMap<NodeId, Instant>>>,
    response_handlers: Arc<RwLock<HashMap<String, NetworkTunnelChannel>>>,
    average_request_per_second: Arc<RwLock<f64>>,
    request_per_second: Arc<RwLock<f64>>,
    rpc: RpcModule<R>,
}

#[async_trait]
pub trait RpcService<R: 'static + Clone + Send + Sync>: 'static + Send + Sync {
    fn log_target(&self) -> String;
    fn node_id(&self) -> NodeId;
    fn service_name(&self) -> TunnelName;
    fn private_network_tx(&self) -> mpsc::Sender<NetworkTunnel>;
    fn rx(
        &self,
    ) -> Arc<
        RwLock<(
            NetworkTunnelReceiver,
            mpsc::Receiver<NetworkTunnel>,
            NetworkTunnelReceiver,
        )>,
    >;
    fn rpc(&self) -> RpcModule<R>;

    async fn boot_initialize(&self) -> anyhow::Result<()> {
        Ok(())
    }

    async fn metrics_behavior(&self, state: RpcServiceState<R>) -> anyhow::Result<()> {
        let mut request_per_second = state.request_per_second.write().await;
        let mut average_request_per_second = state.average_request_per_second.write().await;
        *average_request_per_second = (*average_request_per_second + *request_per_second) / 2.0;
        *request_per_second = 0.0;
        Ok(())
    }

    async fn housekeeping_behavior(&self, state: RpcServiceState<R>) -> anyhow::Result<()> {
        state
            .response_handlers
            .write()
            .await
            .retain(|_, chan| match chan {
                NetworkTunnelChannel::Oneshot(tx) => !tx.is_closed(),
                NetworkTunnelChannel::Mpsc(tx) => !tx.is_closed(),
            });
        state
            .liveliness
            .write()
            .await
            .retain(|_, last_ping| last_ping.elapsed() <= Duration::from_secs(10));
        Ok(())
    }

    async fn stats_behavior(&self, state: RpcServiceState<R>) -> anyhow::Result<()> {
        info!(
            target: &state.log_target,
            "Node {} status( LAST_MSG = {:?} PENDING_OUTBOUND_REQ = {} AVG_RPS = {:.2} )",
            state.node_id,
            state.liveliness
                .read()
                .await
                .iter()
                .map(|(k, v)| {
                    let elapsed = v.elapsed();
                    (
                        k,
                        format!("{}.{}", elapsed.as_secs(), elapsed.subsec_millis()),
                    )
                })
                .collect::<HashMap<&NodeId, String>>(),
            state.response_handlers.read().await.keys().count(),
            state.average_request_per_second.read().await,
        );
        Ok(())
    }

    async fn listen(self: Arc<Self>, mut shutdown_rx: watch::Receiver<bool>) -> anyhow::Result<()> {
        self.boot_initialize().await?;

        let state = RpcServiceState {
            log_target: Arc::new(self.log_target()),
            node_id: Arc::new(self.node_id()),
            service_name: Arc::new(self.service_name()),
            liveliness: Arc::new(RwLock::new(HashMap::new())),
            response_handlers: Arc::new(RwLock::new(HashMap::new())),
            average_request_per_second: Arc::new(RwLock::new(0.0)),
            request_per_second: Arc::new(RwLock::new(0.0)),
            rpc: self.rpc(),
        };

        info!(target: &state.log_target, "RpcService running...");
        let mut metrics_interval = interval(Duration::from_secs(1));
        let mut housekeeping_interval = interval(Duration::from_secs(5));
        let mut stats_interval = interval(Duration::from_secs(15));
        let rx = self.rx();
        let private_network_tx = self.private_network_tx();
        let (internal_rx, private_network_rx, public_network_rx) = &mut *rx.write().await;
        loop {
            tokio::select! {
                _ = shutdown_rx.changed() => {
                    return Ok(());
                }
                _ = metrics_interval.tick() => {
                    self.metrics_behavior(state.clone()).await?;
                }
                _ = housekeeping_interval.tick() => {
                    self.housekeeping_behavior(state.clone()).await?;
                }
                _ = stats_interval.tick() => {
                    self.stats_behavior(state.clone()).await?;
                }
                Some((response_tx, msg)) = internal_rx.recv() => {
                    let state = state.clone();
                    let private_network_tx = private_network_tx.clone();
                    tokio::spawn( async move {
                        let res = Self::handle_internal_network_rx(
                            state,
                            response_tx,
                            private_network_tx,
                            msg,
                        ).await;
                        debug!("handle_internal_network_rx {:?}", res);
                    });
                }
                Some(msg) = private_network_rx.recv() => {
                    let state = state.clone();
                    let private_network_tx = private_network_tx.clone();
                    tokio::spawn(async move {
                        let _ = Self::handle_private_network_rx(
                            state,
                            private_network_tx,
                            msg,
                        ).await;
                    });
                }
                Some((response_tx, msg)) = public_network_rx.recv() => {
                    let state = state.clone();
                    tokio::spawn( async move {
                        let _ = Self::handle_public_network_rx(
                            state,
                            response_tx,
                            msg,
                        ).await;
                    });
                }
            }
        }
    }

    async fn handle_internal_network_rx(
        state: RpcServiceState<R>,
        response_tx: NetworkTunnelChannel,
        private_network_tx: mpsc::Sender<NetworkTunnel>,
        msg: NetworkTunnel,
    ) -> anyhow::Result<()> {
        let tunnel_msg = ingress(&state.log_target, "INTERNAL NETWORK", &msg).await?;
        state
            .response_handlers
            .write()
            .await
            .insert(tunnel_msg.uuid.clone(), response_tx);
        let topic = TunnelTopic::Private((*state.service_name).clone());
        egress(
            &state.log_target,
            "NODE NETWORK",
            NetworkTunnelChannel::Mpsc(private_network_tx),
            topic,
            tunnel_msg,
        )
        .await?;
        Ok(())
    }

    async fn handle_private_network_rx(
        state: RpcServiceState<R>,
        private_network_tx: mpsc::Sender<NetworkTunnel>,
        msg: NetworkTunnel,
    ) -> anyhow::Result<()> {
        let tunnel_msg = ingress(&state.log_target, "NODE NETWORK", &msg).await?;
        let target = tunnel_msg.target.clone();
        if target.unwrap_or(Sender::Node((*state.node_id).clone()))
            == Sender::Node((*state.node_id).clone())
        {
            if tunnel_msg.data["result"] != serde_json::Value::Null
                || tunnel_msg.data["error"] != serde_json::Value::Null
            {
                Self::handle_response(state, msg, tunnel_msg).await?;
            } else {
                *state.request_per_second.write().await += 1.0;
                match &msg.topic {
                    TunnelTopic::Private(service_name) if service_name == &*state.service_name => {
                        let runner = RpcRunner {
                            node_id: state.node_id.clone(),
                            topic: state.service_name.clone(),
                            rpc: state.rpc.clone(),
                            log_target: state.log_target.clone(),
                            channel_name: "NODE NETWORK".to_string(),
                            channel: NetworkTunnelChannel::Mpsc(private_network_tx),
                        };
                        if let Err(err) = runner.run(tunnel_msg).await {
                            warn!(target: &state.log_target, "Runner failed with {:?}", err);
                        }
                    }
                    _ => return Ok(()),
                }
            }
        } else {
            trace!(target: &state.log_target, "Dropping msg");
            return Ok(());
        }
        Ok(())
    }

    async fn handle_public_network_rx(
        state: RpcServiceState<R>,
        response_tx: NetworkTunnelChannel,
        msg: NetworkTunnel,
    ) -> anyhow::Result<()> {
        *state.request_per_second.write().await += 1.0;
        let tunnel_msg = ingress(&state.log_target, "PUBLIC NETWORK", &msg).await?;

        let runner = RpcRunner {
            node_id: state.node_id.clone(),
            topic: state.service_name.clone(),
            rpc: state.rpc.clone(),
            log_target: state.log_target.clone(),
            channel_name: "PUBLIC NETWORK".to_string(),
            channel: response_tx,
        };
        if let Err(err) = runner.run(tunnel_msg).await {
            warn!(target: &state.log_target, "Runner failed with {:?}", err);
        }

        Ok(())
    }

    async fn handle_response(
        state: RpcServiceState<R>,
        msg: NetworkTunnel,
        tunnel_msg: RpcTunnel,
    ) -> anyhow::Result<()> {
        trace!(target: &state.log_target, "{} Response {:?}", tunnel_msg.uuid, tunnel_msg.data);
        let response_handler = state
            .response_handlers
            .write()
            .await
            .remove(&tunnel_msg.uuid);
        if let Some(response_handler) = response_handler {
            match response_handler {
                NetworkTunnelChannel::Oneshot(response_tx) => {
                    if !response_tx.is_closed() {
                        let _ = response_tx.send(msg);
                    }
                }
                NetworkTunnelChannel::Mpsc(response_tx) => {
                    let _ = response_tx.send(msg).await?;
                    if !response_tx.is_closed() {
                        state.response_handlers.write().await.insert(
                            tunnel_msg.uuid.clone(),
                            NetworkTunnelChannel::Mpsc(response_tx),
                        );
                    }
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jsonrpsee::proc_macros::rpc;
    use serde_json::json;
    use tokio::sync::oneshot;

    use moonramp_core::NetworkTunnelSender;

    #[rpc(server)]
    trait TestRpc {
        #[method(name = "test.ping")]
        fn ping(&self) -> RpcResult<String>;
    }

    #[derive(Clone)]
    struct TestRpcImpl;

    impl TestRpcServer for TestRpcImpl {
        fn ping(&self) -> RpcResult<String> {
            Ok("pong".to_string())
        }
    }

    struct TestRpcService {
        node_id: NodeId,
        rx: Arc<
            RwLock<(
                NetworkTunnelReceiver,
                mpsc::Receiver<NetworkTunnel>,
                NetworkTunnelReceiver,
            )>,
        >,
        private_network_tx: mpsc::Sender<NetworkTunnel>,
        rpc: RpcModule<TestRpcImpl>,
    }

    impl TestRpcService {
        fn new(
            private_network_tx: mpsc::Sender<NetworkTunnel>,
        ) -> (mpsc::Sender<NetworkTunnel>, NetworkTunnelSender, Self) {
            let (_internal_tx, internal_rx) = mpsc::channel(1);
            let (private_tx, private_network_rx) = mpsc::channel(1);
            let (public_tx, public_network_rx) = mpsc::channel(1);

            let rpc = TestRpcImpl.into_rpc();

            (
                private_tx,
                public_tx,
                TestRpcService {
                    node_id: NodeId::from("test".to_string()),
                    rx: Arc::new(RwLock::new((
                        internal_rx,
                        private_network_rx,
                        public_network_rx,
                    ))),
                    private_network_tx,
                    rpc,
                },
            )
        }
    }

    #[async_trait]
    impl RpcService<TestRpcImpl> for TestRpcService {
        fn log_target(&self) -> String {
            "test::rpc".to_string()
        }

        fn node_id(&self) -> NodeId {
            self.node_id.clone()
        }

        fn service_name(&self) -> TunnelName {
            TunnelName::Test
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

        fn rpc(&self) -> RpcModule<TestRpcImpl> {
            self.rpc.clone()
        }
    }

    #[tokio::test]
    async fn test_handle_internal_network_rx() {
        let (p_tx, mut pri_rx) = mpsc::channel(1);
        let (_, _, rpc) = TestRpcService::new(p_tx);
        let state = RpcServiceState {
            log_target: Arc::new(rpc.log_target()),
            node_id: Arc::new(rpc.node_id()),
            service_name: Arc::new(rpc.service_name()),
            liveliness: Arc::new(RwLock::new(HashMap::new())),
            response_handlers: Arc::new(RwLock::new(HashMap::new())),
            average_request_per_second: Arc::new(RwLock::new(0.0)),
            request_per_second: Arc::new(RwLock::new(0.0)),
            rpc: rpc.rpc(),
        };

        let (r_tx, _) = oneshot::channel();
        let tunnel_msg = RpcTunnel {
            uuid: "12345".to_string(),
            sender: Sender::Node(rpc.node_id()),
            target: None,
            data: json!({}),
        };
        let msg = NetworkTunnel {
            topic: TunnelTopic::Private(TunnelName::Test),
            tunnel_data: serde_json::to_vec(&tunnel_msg).expect("Invalid RpcTunnel"),
        };
        let res = TestRpcService::handle_internal_network_rx(
            state.clone(),
            NetworkTunnelChannel::Oneshot(r_tx),
            rpc.private_network_tx(),
            msg.clone(),
        )
        .await;
        assert!(res.is_ok());
        assert!(state.response_handlers.read().await.contains_key("12345"));
        assert_eq!(pri_rx.recv().await, Some(msg));
    }

    #[tokio::test]
    async fn test_handle_private_network_rx() {
        let (p_tx, mut pri_rx) = mpsc::channel(1);
        let (_, _, rpc) = TestRpcService::new(p_tx);
        let state = RpcServiceState {
            log_target: Arc::new(rpc.log_target()),
            node_id: Arc::new(rpc.node_id()),
            service_name: Arc::new(rpc.service_name()),
            liveliness: Arc::new(RwLock::new(HashMap::new())),
            response_handlers: Arc::new(RwLock::new(HashMap::new())),
            average_request_per_second: Arc::new(RwLock::new(0.0)),
            request_per_second: Arc::new(RwLock::new(0.0)),
            rpc: rpc.rpc(),
        };

        let tunnel_msg = RpcTunnel {
            uuid: "12345".to_string(),
            sender: Sender::Node(NodeId::from("test2".to_string())),
            target: Some(Sender::Node(rpc.node_id())),
            data: json!({
                "jsonrpc": "2.0",
                "method": "test.ping",
                "params": {},
                "id": "12345",
            }),
        };
        let msg = NetworkTunnel {
            topic: TunnelTopic::Private(TunnelName::Test),
            tunnel_data: serde_json::to_vec(&tunnel_msg).expect("Invalid RpcTunnel"),
        };

        let res =
            TestRpcService::handle_private_network_rx(state.clone(), rpc.private_network_tx(), msg)
                .await;
        assert!(res.is_ok());
        assert_eq!(*state.request_per_second.read().await, 1.0);
        let resp = pri_rx.recv().await.expect("Invalid response");
        let tunnel_msg: RpcTunnel =
            serde_json::from_slice(&resp.tunnel_data).expect("Invalid RpcTunnel");
        let expected_tunnel_msg = RpcTunnel {
            uuid: "12345".to_string(),
            sender: Sender::Node(rpc.node_id()),
            target: Some(Sender::Node(NodeId::from("test2".to_string()))),
            data: json!({
                "jsonrpc": "2.0",
                "result": "pong",
                "id": "12345",
            }),
        };
        assert_eq!(tunnel_msg, expected_tunnel_msg);
    }

    #[tokio::test]
    async fn test_handle_public_network_rx() {
        let (p_tx, _) = mpsc::channel(1);
        let (_, _, rpc) = TestRpcService::new(p_tx);
        let state = RpcServiceState {
            log_target: Arc::new(rpc.log_target()),
            node_id: Arc::new(rpc.node_id()),
            service_name: Arc::new(rpc.service_name()),
            liveliness: Arc::new(RwLock::new(HashMap::new())),
            response_handlers: Arc::new(RwLock::new(HashMap::new())),
            average_request_per_second: Arc::new(RwLock::new(0.0)),
            request_per_second: Arc::new(RwLock::new(0.0)),
            rpc: rpc.rpc(),
        };

        let tunnel_msg = RpcTunnel {
            uuid: "12345".to_string(),
            sender: Sender::Node(NodeId::from("test2".to_string())),
            target: Some(Sender::Node(rpc.node_id())),
            data: json!({
                "jsonrpc": "2.0",
                "method": "test.ping",
                "params": {},
                "id": "12345",
            }),
        };
        let msg = NetworkTunnel {
            topic: TunnelTopic::Private(TunnelName::Test),
            tunnel_data: serde_json::to_vec(&tunnel_msg).expect("Invalid RpcTunnel"),
        };

        let (r_tx, r_rx) = oneshot::channel();
        let res = TestRpcService::handle_public_network_rx(
            state.clone(),
            NetworkTunnelChannel::Oneshot(r_tx),
            msg,
        )
        .await;
        assert!(res.is_ok());
        assert_eq!(*state.request_per_second.read().await, 1.0);
        let resp = r_rx.await.expect("Invalid response");
        let tunnel_msg: RpcTunnel =
            serde_json::from_slice(&resp.tunnel_data).expect("Invalid RpcTunnel");
        let expected_tunnel_msg = RpcTunnel {
            uuid: "12345".to_string(),
            sender: Sender::Node(rpc.node_id()),
            target: Some(Sender::Node(NodeId::from("test2".to_string()))),
            data: json!({
                "jsonrpc": "2.0",
                "result": "pong",
                "id": "12345",
            }),
        };
        assert_eq!(tunnel_msg, expected_tunnel_msg);
    }
}
