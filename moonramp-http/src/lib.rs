#![deny(missing_docs)]

//! moonramp-http contains helper http structs and methods used by moonramp http servers

use std::{error::Error, fmt};

use actix_web::{http::StatusCode, HttpResponse, ResponseError};
use anyhow::anyhow;
use argon2::{
    password_hash::{PasswordHash, PasswordVerifier},
    Argon2,
};
use log::{debug, warn};
use sea_orm::{entity::*, query::*, DatabaseConnection};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::{
    sync::oneshot,
    time::{sleep, Duration, Instant},
};

use moonramp_core::{
    actix_web, anyhow, argon2, log, sea_orm, serde, serde_json, tokio, ApiCredential,
    NetworkTunnel, NetworkTunnelChannel, NetworkTunnelSender, RpcTunnel, Sender, TunnelName,
    TunnelTopic,
};
use moonramp_encryption::{EncryptionKeyCustodian, KeyCustodian, KeyEncryptionKeyCustodian};
use moonramp_entity::{api_token, encryption_key, role};

/// Http Error Codes
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(crate = "moonramp_core::serde", rename_all = "camelCase")]
pub enum HttpError {
    /// actix::http::StatusCode::NOT_FOUND 404
    NotFound,
    /// actix::http::StatusCode::NOT_FOUND 500
    ServerError,
    /// actix::http::StatusCode::NOT_FOUND 504
    Timeout,
    /// actix::http::StatusCode::NOT_FOUND 401
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

/// Serializes an incoming json blob into a NetworkTunnel
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

/// Validates a given role contains a resource scope pair
pub fn check_roles(roles: &[role::Model], resource: role::Resource, scope: role::Scope) -> bool {
    roles
        .iter()
        .find(|r| match (&r.resource, &r.scope, &r.api_group) {
            (r, s, _) if *r == resource && *s == scope => true,
            _ => false,
        })
        .is_some()
}

/// Retrieves the api_token::Model and role::Model from the data store
pub async fn api_token(
    bearer_token: &str,
    kek_custodian: &KeyEncryptionKeyCustodian,
    database: &DatabaseConnection,
) -> anyhow::Result<Option<(api_token::Model, Vec<role::Model>)>> {
    let api_credential = ApiCredential::from_bearer(bearer_token)?;
    let t_r = api_token::Entity::find()
        .filter(api_token::Column::Hash.eq(api_credential.hash))
        .find_with_related(role::Entity)
        .all(database)
        .await?
        .pop();

    match t_r {
        Some((token, roles)) => {
            let t_ek = encryption_key::Entity::find()
                .filter(
                    Condition::all()
                        .add(encryption_key::Column::Hash.eq(token.encryption_key_hash.clone()))
                        .add(encryption_key::Column::KeyEncryptionKeyHash.eq(kek_custodian.hash())),
                )
                .all(database)
                .await?
                .into_iter()
                .next();

            if let Some(t_ek) = t_ek {
                let t_ek_custodian = EncryptionKeyCustodian::new(
                    kek_custodian.unlock(t_ek)?.secret.to_vec(),
                    token.cipher.clone(),
                )?;
                let parsed_hash = t_ek_custodian.decrypt(&token.nonce, &token.blob)?;
                let parsed_hash = PasswordHash::new(std::str::from_utf8(&parsed_hash)?)?;

                if Argon2::default()
                    .verify_password(&api_credential.secret, &parsed_hash)
                    .is_ok()
                {
                    Ok(Some((token, roles)))
                } else {
                    Ok(None)
                }
            } else {
                Ok(None)
            }
        }
        None => Ok(None),
    }
}

/// Sends a request to the registry channel, awaits a response, and deserializes the response into a json blob
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
