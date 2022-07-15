use std::{error::Error, fmt};

use actix_web::{http::StatusCode, HttpResponse, ResponseError};
use anyhow::anyhow;
use log::{debug, warn};
use sea_orm::{entity::*, query::*, DatabaseConnection};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::{
    sync::oneshot,
    time::{sleep, Duration, Instant},
};

use moonramp_core::{
    actix_web, anyhow, log, sea_orm, serde, serde_json, tokio, NetworkTunnel, NetworkTunnelChannel,
    NetworkTunnelSender, RpcTunnel, Sender, TunnelName, TunnelTopic,
};
use moonramp_entity::{api_token, role};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(crate = "moonramp_core::serde", rename_all = "camelCase")]
pub enum HttpError {
    NotFound,
    ServerError,
    Timeout,
    Unauthorized,
}

impl fmt::Display for HttpError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Error for HttpError {}

impl ResponseError for HttpError {
    fn error_response(&self) -> HttpResponse {
        let body = json!({
            "id": "-",
            "jsonrpc": "2.0",
            "error": self,
        });

        HttpResponse::build(self.status_code()).json(body)
    }

    fn status_code(&self) -> StatusCode {
        match self {
            HttpError::NotFound => StatusCode::NOT_FOUND,
            HttpError::ServerError => StatusCode::INTERNAL_SERVER_ERROR,
            HttpError::Timeout => StatusCode::GATEWAY_TIMEOUT,
            HttpError::Unauthorized => StatusCode::UNAUTHORIZED,
        }
    }
}

pub fn network_tunnel(
    id: &str,
    sender: Sender,
    sevice_name: TunnelName,
    data: serde_json::Value,
) -> anyhow::Result<NetworkTunnel> {
    let tunnel_msg = RpcTunnel {
        uuid: id.to_string(),
        sender,
        target: None,
        data,
    };
    Ok(NetworkTunnel {
        topic: TunnelTopic::Public(sevice_name),
        tunnel_data: serde_json::to_vec(&tunnel_msg)?,
    })
}

pub fn check_roles(roles: &[role::Model], resource: role::Resource, scope: role::Scope) -> bool {
    roles
        .iter()
        .find(|r| match (&r.resource, &r.scope, &r.api_group) {
            (r, s, _) if *r == resource && *s == scope => true,
            _ => false,
        })
        .is_some()
}

pub async fn api_token(
    token: &str,
    database: &DatabaseConnection,
) -> anyhow::Result<Option<(api_token::Model, Vec<role::Model>)>> {
    Ok(api_token::Entity::find()
        .filter(api_token::Column::Token.eq(token))
        .find_with_related(role::Entity)
        .all(database)
        .await?
        .pop())
}

pub async fn await_response(
    log_target: &str,
    timeout: Duration,
    start: Instant,
    registry_tx: &NetworkTunnelSender,
    id: String,
    msg: NetworkTunnel,
    method: &str,
    path: &str,
) -> anyhow::Result<serde_json::Value> {
    let (response_tx, response_rx) = oneshot::channel();
    registry_tx
        .send((NetworkTunnelChannel::Oneshot(response_tx), msg))
        .await
        .map_err(|_| HttpError::ServerError)?;
    let res_timeout = sleep(timeout);
    tokio::pin!(res_timeout);
    tokio::select! {
        _ = &mut res_timeout => {
            warn!(
                target: log_target,
                "{} {} TIMEOUT {}ms {}",
                id,
                method,
                start.elapsed().as_millis(),
                path
            );
            Err(anyhow!(HttpError::Timeout))
        }
        Ok(res) = response_rx => {
            let tunnel_msg: RpcTunnel = serde_json::from_slice(&res.tunnel_data)
                .map_err(|_| HttpError::ServerError)?;
            debug!(
                target: log_target,
                "{} {} OK({}) {}ms {}",
                id,
                method,
                tunnel_msg.data["result"] != serde_json::Value::Null,
                start.elapsed().as_millis(),
                path
            );
            Ok(tunnel_msg.data)
        }
    }
}
