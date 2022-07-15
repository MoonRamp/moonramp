use log::trace;

use moonramp_core::{
    anyhow, log, serde_json, NetworkTunnel, NetworkTunnelChannel, RpcTunnel, TunnelTopic,
};

pub async fn egress(
    log_target: &str,
    channel_name: &str,
    channel: NetworkTunnelChannel,
    topic: TunnelTopic,
    tunnel_msg: RpcTunnel,
) -> anyhow::Result<()> {
    let msg = NetworkTunnel {
        topic,
        tunnel_data: serde_json::to_vec(&tunnel_msg)?,
    };
    trace!(
        target: log_target,
        "[EGRESS {}] {} {} {} bytes Sender = {} Target = {:?}",
        channel_name,
        tunnel_msg.uuid,
        msg.topic,
        msg.tunnel_data.len(),
        tunnel_msg.sender,
        tunnel_msg.target,
    );
    egress_channel(channel, msg).await
}

async fn egress_channel(channel: NetworkTunnelChannel, msg: NetworkTunnel) -> anyhow::Result<()> {
    match channel {
        NetworkTunnelChannel::Oneshot(tx) => {
            let _ = tx.send(msg);
        }
        NetworkTunnelChannel::Mpsc(tx) => {
            tx.send(msg).await?;
        }
    }
    Ok(())
}
