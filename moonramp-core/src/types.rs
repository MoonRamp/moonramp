#[cfg(feature = "crypto")]
use std::{convert::TryFrom, ops::Deref};
use std::{fmt, hash::Hash as StdHash, net::SocketAddr};

use serde::{Deserialize, Serialize};
#[cfg(feature = "async-core")]
use tokio::sync::{mpsc, oneshot};

#[derive(Clone, Debug, StdHash, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
pub struct NodeId(pub String);

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<'a> From<&'a str> for NodeId {
    fn from(val: &str) -> NodeId {
        NodeId(val.to_string())
    }
}

impl From<String> for NodeId {
    fn from(val: String) -> NodeId {
        NodeId(val)
    }
}

#[derive(Clone, Copy, Debug, Eq, StdHash, PartialEq, Serialize, Deserialize)]
pub enum TunnelName {
    Plugin,
    Program,
    Sale,
    Test,
    Wallet,
}

impl fmt::Display for TunnelName {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum TunnelTopic {
    Drop,
    Private(TunnelName),
    Public(TunnelName),
}

impl fmt::Display for TunnelTopic {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TunnelTopic::Private(service_name) => write!(f, "Private-{}", service_name),
            TunnelTopic::Public(service_name) => {
                write!(f, "Public-{}", service_name)
            }
            TunnelTopic::Drop => write!(f, "Drop"),
        }
    }
}

impl From<&str> for TunnelTopic {
    fn from(val: &str) -> TunnelTopic {
        match val {
            "Private-Plugin" => TunnelTopic::Private(TunnelName::Plugin),
            "Public-Plugin" => TunnelTopic::Public(TunnelName::Plugin),
            "Private-Sale" => TunnelTopic::Private(TunnelName::Sale),
            "Public-Sale" => TunnelTopic::Public(TunnelName::Sale),
            "Private-Test" => TunnelTopic::Private(TunnelName::Test),
            "Public-Test" => TunnelTopic::Public(TunnelName::Test),
            "Private-Wallet" => TunnelTopic::Private(TunnelName::Wallet),
            "Public-Wallet" => TunnelTopic::Public(TunnelName::Wallet),
            _ => TunnelTopic::Drop,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub enum Sender {
    Node(NodeId),
    Addr(String),
}

impl fmt::Display for Sender {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Sender::Node(node_id) => write!(f, "Node({})", node_id),
            Sender::Addr(addr) => write!(f, "Addr({})", addr),
        }
    }
}

impl From<SocketAddr> for Sender {
    fn from(addr: SocketAddr) -> Sender {
        Sender::Addr(addr.to_string())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct RpcTunnel {
    pub uuid: String,
    pub sender: Sender,
    pub target: Option<Sender>,
    pub data: serde_json::Value,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
pub struct NetworkTunnel {
    pub topic: TunnelTopic,
    pub tunnel_data: Vec<u8>,
}

#[cfg(feature = "async-core")]
pub type NetworkTunnelSender = mpsc::Sender<(NetworkTunnelChannel, NetworkTunnel)>;
#[cfg(feature = "async-core")]
pub type NetworkTunnelReceiver = mpsc::Receiver<(NetworkTunnelChannel, NetworkTunnel)>;

#[cfg(feature = "async-core")]
#[derive(Debug)]
pub enum NetworkTunnelChannel {
    Oneshot(oneshot::Sender<NetworkTunnel>),
    Mpsc(mpsc::Sender<NetworkTunnel>),
}

#[cfg(feature = "crypto")]
#[derive(Clone, PartialEq, Eq, StdHash, Deserialize, Serialize)]
pub struct Hash(pub [u8; 32]);

#[cfg(feature = "crypto")]
impl Hash {
    pub fn new() -> Self {
        Hash([0; 32])
    }
}

#[cfg(feature = "crypto")]
impl Deref for Hash {
    type Target = [u8];
    fn deref(&self) -> &Self::Target {
        let Hash(inner) = self;
        &inner[..]
    }
}

#[cfg(feature = "crypto")]
impl AsRef<[u8]> for Hash {
    fn as_ref(&self) -> &[u8] {
        &self
    }
}

#[cfg(feature = "crypto")]
impl fmt::Debug for Hash {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Hash({})", bs58::encode(self).into_string())
    }
}

#[cfg(feature = "crypto")]
impl fmt::Display for Hash {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", bs58::encode(self).into_string())
    }
}

#[cfg(feature = "crypto")]
impl From<[u8; 32]> for Hash {
    fn from(val: [u8; 32]) -> Hash {
        Hash(val)
    }
}

//#[cfg(feature = "crypto")]
//impl From<&[u8]> for Hash {
//    fn from(val: &[u8]) -> Hash {
//        let buf: [u8; 32] = val.try_into().unwrap_or([0; 32]);
//        Hash(buf)
//    }
//}

#[cfg(feature = "crypto")]
impl From<Vec<u8>> for Hash {
    fn from(val: Vec<u8>) -> Hash {
        let buf: [u8; 32] = val.try_into().unwrap_or([0; 32]);
        Hash(buf)
    }
}

#[cfg(feature = "crypto")]
impl<'a> TryFrom<&'a str> for Hash {
    type Error = anyhow::Error;
    fn try_from(val: &str) -> anyhow::Result<Hash> {
        let mut buf = [0u8; 32];
        bs58::decode(val).into(&mut buf)?;
        Ok(Hash(buf))
    }
}

#[cfg(all(feature = "crypto", feature = "sql"))]
impl sea_orm::TryFromU64 for Hash {
    fn try_from_u64(_n: u64) -> Result<Self, sea_orm::DbErr> {
        Err(sea_orm::DbErr::Exec(
            "Hash cannot be converted from u64".to_string(),
        ))
    }
}

#[cfg(all(feature = "crypto", feature = "sql"))]
impl sea_orm::TryGetable for Hash {
    fn try_get(
        res: &sea_orm::QueryResult,
        pre: &str,
        col: &str,
    ) -> Result<Self, sea_orm::TryGetError> {
        let opt: Option<String> = res.try_get(pre, col).map_err(sea_orm::TryGetError::DbErr)?;
        match opt {
            Some(val) => Ok(Hash::try_from(val.as_ref())
                .map_err(|_| sea_orm::DbErr::Exec("Invalid Hash".to_string()))
                .map_err(sea_orm::TryGetError::DbErr)?),
            None => Err(sea_orm::TryGetError::Null),
        }
    }
}

#[cfg(all(feature = "crypto", feature = "sql"))]
impl sea_orm::sea_query::Nullable for Hash {
    fn null() -> sea_orm::Value {
        sea_orm::Value::String(None)
    }
}

#[cfg(all(feature = "crypto", feature = "sql"))]
impl sea_orm::sea_query::ValueType for Hash {
    fn try_from(v: sea_orm::Value) -> Result<Self, sea_orm::sea_query::ValueTypeErr> {
        match v {
            sea_orm::Value::String(Some(x)) => {
                Ok(TryFrom::try_from((*x).as_ref())
                    .map_err(|_| sea_orm::sea_query::ValueTypeErr)?)
            }
            _ => Err(sea_orm::sea_query::ValueTypeErr),
        }
    }

    fn type_name() -> String {
        "Hash".to_string()
    }

    fn column_type() -> sea_orm::sea_query::ColumnType {
        sea_orm::sea_query::ColumnType::Text
    }
}

#[cfg(all(feature = "crypto", feature = "sql"))]
impl From<Hash> for sea_orm::Value {
    fn from(h: Hash) -> sea_orm::Value {
        sea_orm::Value::from(h.to_string())
    }
}
