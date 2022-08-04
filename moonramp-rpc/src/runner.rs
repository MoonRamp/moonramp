use std::sync::Arc;

use jsonrpsee::RpcModule;
use log::warn;

use moonramp_core::{
    anyhow, jsonrpsee, log, serde_json, NetworkTunnelChannel, NodeId, RpcTunnel, Sender,
    TunnelName, TunnelTopic,
};

pub struct RpcRunner<R: 'static + Send + Sync> {
    pub node_id: Arc<NodeId>,
    pub topic: Arc<TunnelName>,
    pub rpc: RpcModule<R>,
    pub log_target: Arc<String>,
    pub channel_name: String,
    pub channel: NetworkTunnelChannel,
}

impl<R: 'static + Send + Sync> RpcRunner<R> {
    pub async fn run(self, tunnel_msg: RpcTunnel) -> anyhow::Result<()> {
        match self
            .rpc
            .raw_json_request(&serde_json::to_string(&tunnel_msg.data)?)
            .await
        {
            Ok((resp, _)) => {
                let resp = RpcTunnel {
                    uuid: tunnel_msg.uuid,
                    sender: Sender::Node((*self.node_id).clone()),
                    target: Some(tunnel_msg.sender),
                    data: serde_json::from_str(&resp)?,
                };
                super::egress(
                    &self.log_target,
                    &self.channel_name,
                    self.channel,
                    TunnelTopic::Private((*self.topic).clone()),
                    resp,
                )
                .await?;
            }
            Err(err) => {
                warn!(target: &self.log_target, "{} Failed to process Sender = {} {:?}", tunnel_msg.uuid, tunnel_msg.sender, err);
            }
        }

        Ok(())
    }
}
