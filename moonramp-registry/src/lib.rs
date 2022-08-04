use std::{collections::HashMap, error::Error};

use log::{info, trace, warn};
use tokio::sync::mpsc;

use moonramp_core::{
    log, tokio, NetworkTunnelReceiver, NetworkTunnelSender, TunnelName, TunnelTopic,
};

struct RegistryEntry {
    service_tx: NetworkTunnelSender,
}

pub struct Registry {
    registry_rx: NetworkTunnelReceiver,
    registry: HashMap<TunnelName, RegistryEntry>,
}

impl Registry {
    pub fn new() -> (NetworkTunnelSender, Self) {
        let (registry_tx, registry_rx) = mpsc::channel(1024);
        (
            registry_tx,
            Registry {
                registry_rx,
                registry: HashMap::new(),
            },
        )
    }

    pub fn register(&mut self, service_name: TunnelName, service_tx: NetworkTunnelSender) {
        info!("Adding {} to registry", service_name);
        self.registry
            .insert(service_name, RegistryEntry { service_tx });
    }

    pub async fn run(&mut self) -> Result<(), Box<dyn Error>> {
        info!("Starting registry");
        loop {
            self.inner_run().await?;
        }
    }
    async fn inner_run(&mut self) -> Result<(), Box<dyn Error>> {
        if let Some((response_tx, msg)) = self.registry_rx.recv().await {
            trace!("Registry received {:?}", msg);
            match msg.topic {
                TunnelTopic::Public(name) => {
                    if let Some(backend) = self.registry.get(&name) {
                        backend.service_tx.send((response_tx, msg)).await?;
                    }
                }
                _ => warn!("Backend for topic {} not found", msg.topic),
            }
        }
        Ok(())
    }
}

#[tokio::test]
async fn test_registry() {
    use moonramp_core::{NetworkTunnel, NetworkTunnelChannel};

    let (w_tx, mut w_rx) = mpsc::channel(1);
    let (r_rx, mut registry) = Registry::new();
    registry.register(TunnelName::Wallet, w_tx);

    let (res_tx, _res_rx) = mpsc::channel(1);
    let msg = NetworkTunnel {
        topic: TunnelTopic::Public(TunnelName::Wallet),
        tunnel_data: vec![0x01, 0x03, 0x03, 0x07],
    };
    assert!(r_rx
        .send((NetworkTunnelChannel::Mpsc(res_tx), msg))
        .await
        .is_ok());
    assert!(registry.inner_run().await.is_ok());
    assert!(w_rx.recv().await.is_some());
}
