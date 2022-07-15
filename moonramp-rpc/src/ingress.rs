use log::trace;

use moonramp_core::{anyhow, log, serde_json, NetworkTunnel, RpcTunnel};

pub async fn ingress(
    log_target: &str,
    channel_name: &str,
    msg: &NetworkTunnel,
) -> anyhow::Result<RpcTunnel> {
    let tunnel_msg: RpcTunnel = serde_json::from_slice(&msg.tunnel_data)?;
    trace!(
        target: log_target,
        "[INGRESS {}] {} {} {} bytes Sender = {} Target = {:?}",
        channel_name,
        tunnel_msg.uuid,
        msg.topic,
        msg.tunnel_data.len(),
        tunnel_msg.sender,
        tunnel_msg.target,
    );
    Ok(tunnel_msg)
}
