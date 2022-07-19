use std::{error::Error, fmt, ops::Deref, ops::DerefMut};

use actix_web::{
    dev::Server, get, guard, post, web, App, HttpRequest, HttpResponse, HttpServer, Responder,
};
use actix_web_httpauth::extractors::bearer::BearerAuth;
use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::time::{Duration, Instant};
use uuid::Uuid;

use moonramp_core::{
    actix_web, actix_web_httpauth, anyhow, sea_orm, serde, serde_json, tokio, uuid,
    NetworkTunnelSender, Sender, TunnelName,
};
use moonramp_entity::role;
use moonramp_http::{api_token, await_response, check_roles, network_tunnel, HttpError};

use crate::params::*;

pub struct ProgramHttpServer {
    inner: Server,
}

#[derive(Clone)]
pub struct ProgramHttpServerData {
    timeout: Duration,
    database: DatabaseConnection,
    registry_tx: NetworkTunnelSender,
}

impl ProgramHttpServer {
    pub async fn new(
        database: DatabaseConnection,
        registry_tx: NetworkTunnelSender,
        program_http_addr: &str,
    ) -> anyhow::Result<Self> {
        let data = web::Data::new(ProgramHttpServerData {
            timeout: Duration::from_millis(30000),
            database,
            registry_tx,
        });
        let inner = HttpServer::new(move || {
            App::new()
                .app_data(web::JsonConfig::default().limit(1024 * 1024 * 50))
                .service(
                    web::scope("/jsonrpc")
                        .app_data(data.clone())
                        .guard(guard::Header("content-type", "application/json"))
                        .service(jsonrpc),
                )
                .service(
                    web::scope("/program")
                        .app_data(data.clone())
                        .guard(guard::Header("content-type", "application/json"))
                        .service(program_version)
                        .service(program_post)
                        .service(program_get),
                )
                .service(ping)
        })
        .system_exit()
        .disable_signals()
        .shutdown_timeout(0)
        //.keep_alive(None)
        .bind(program_http_addr)?
        .run();

        Ok(ProgramHttpServer { inner })
    }

    pub async fn listen(self) -> Result<(), Box<dyn Error>> {
        Ok(self.inner.await?)
    }
}

#[post("")]
async fn jsonrpc(
    state: web::Data<ProgramHttpServerData>,
    req: HttpRequest,
    auth: BearerAuth,
    data: web::Json<serde_json::Value>,
) -> actix_web::Result<impl Responder> {
    let start = Instant::now();

    let token = auth.token();
    let t_r = api_token(token, &state.database)
        .await
        .map_err(|err| err.downcast().unwrap_or(HttpError::ServerError))?
        .ok_or(HttpError::Unauthorized)?;
    let (t, rs) = t_r;

    let mut data = data.into_inner();
    if data["params"]["merchant_id"] != serde_json::Value::Null {
        return Err(HttpError::Unauthorized)?;
    }
    data["params"]["merchant_id"] = serde_json::Value::String(t.merchant_id.clone());

    let allowed = match data["method"].as_str() {
        Some("program.version") => true,
        Some("program.create") | Some("program.update") => {
            check_roles(&rs, role::Resource::Program, role::Scope::Write)
        }
        Some("program.lookup") => check_roles(&rs, role::Resource::Program, role::Scope::Read),
        _ => false,
    };

    if !allowed {
        return Err(HttpError::Unauthorized)?;
    }

    let sender = req
        .peer_addr()
        .map(|addr| Sender::from(addr))
        .unwrap_or(Sender::Addr("UNKNOWN_PEER_ADDR".to_string()));
    let id = Uuid::new_v4().to_simple().to_string();
    let msg = network_tunnel(&id, sender, TunnelName::Program, data)
        .map_err(|err| err.downcast().unwrap_or(HttpError::ServerError))?;
    Ok(web::Json(
        await_response(
            "moonramp_program::http",
            state.timeout,
            start,
            &state.registry_tx,
            id,
            msg,
            "POST",
            "/jsonrpc",
        )
        .await
        .map_err(|err| err.downcast().unwrap_or(HttpError::ServerError))?,
    ))
}

#[get("/version")]
async fn program_version(
    state: web::Data<ProgramHttpServerData>,
    req: HttpRequest,
) -> actix_web::Result<impl Responder> {
    let start = Instant::now();

    let id = Uuid::new_v4().to_simple().to_string();
    let data = json!({
        "jsonrpc": "2.0",
        "method": "program.version",
        "params": {},
        "id": id,
    });

    let sender = req
        .peer_addr()
        .map(|addr| Sender::from(addr))
        .unwrap_or(Sender::Addr("UNKNOWN_PEER_ADDR".to_string()));
    let msg = network_tunnel(&id, sender, TunnelName::Program, data)
        .map_err(|err| err.downcast().unwrap_or(HttpError::ServerError))?;
    Ok(web::Json(
        await_response(
            "moonramp_program::http",
            state.timeout,
            start,
            &state.registry_tx,
            id,
            msg,
            "GET",
            "/program/version",
        )
        .await
        .map_err(|err| err.downcast().unwrap_or(HttpError::ServerError))?,
    ))
}

#[post("")]
async fn program_post(
    state: web::Data<ProgramHttpServerData>,
    req: HttpRequest,
    auth: BearerAuth,
    create_req: web::Json<ProgramCreateRequest>,
) -> actix_web::Result<impl Responder> {
    let start = Instant::now();

    let token = auth.token();
    let t_r = api_token(token, &state.database)
        .await
        .map_err(|err| err.downcast().unwrap_or(HttpError::ServerError))?
        .ok_or(HttpError::Unauthorized)?;
    let (t, rs) = t_r;

    if !check_roles(&rs, role::Resource::Program, role::Scope::Write) {
        return Err(HttpError::Unauthorized)?;
    }

    let id = Uuid::new_v4().to_simple().to_string();
    let data = json!({
        "jsonrpc": "2.0",
        "method": "program.create",
        "params": {
            "merchant_id": t.merchant_id,
            "request": create_req,
        },
        "id": id,
    });

    let sender = req
        .peer_addr()
        .map(|addr| Sender::from(addr))
        .unwrap_or(Sender::Addr("UNKNOWN_PEER_ADDR".to_string()));
    let msg = network_tunnel(&id, sender, TunnelName::Program, data)
        .map_err(|err| err.downcast().unwrap_or(HttpError::ServerError))?;
    Ok(web::Json(
        await_response(
            "moonramp_program::http",
            state.timeout,
            start,
            &state.registry_tx,
            id,
            msg,
            "POST",
            "/program",
        )
        .await
        .map_err(|err| err.downcast().unwrap_or(HttpError::ServerError))?,
    ))
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(crate = "moonramp_core::serde", rename_all = "camelCase")]
pub enum HashOrName {
    Hash,
    Name,
}

impl fmt::Display for HashOrName {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", format!("{:?}", self).to_lowercase())
    }
}

#[get("/{hash_or_name}/{program_hash}")]
async fn program_get(
    state: web::Data<ProgramHttpServerData>,
    req: HttpRequest,
    auth: BearerAuth,
    path: web::Path<(HashOrName, String)>,
) -> actix_web::Result<impl Responder> {
    let start = Instant::now();

    let token = auth.token();
    let t_r = api_token(token, &state.database)
        .await
        .map_err(|err| err.downcast().unwrap_or(HttpError::ServerError))?
        .ok_or(HttpError::Unauthorized)?;
    let (t, rs) = t_r;

    if !check_roles(&rs, role::Resource::Program, role::Scope::Read) {
        return Err(HttpError::Unauthorized)?;
    }

    let (hash_or_name, program_hash_or_name) = path.into_inner();
    let id = Uuid::new_v4().to_simple().to_string();
    let data = json!({
        "jsonrpc": "2.0",
        "method": "program.lookup",
        "params": {
            "merchant_id": t.merchant_id,
            "request": {
                hash_or_name.to_string() : program_hash_or_name,
            },
        },
        "id": id,
    });

    let sender = req
        .peer_addr()
        .map(|addr| Sender::from(addr))
        .unwrap_or(Sender::Addr("UNKNOWN_PEER_ADDR".to_string()));
    let msg = network_tunnel(&id, sender, TunnelName::Program, data)
        .map_err(|err| err.downcast().unwrap_or(HttpError::ServerError))?;
    Ok(web::Json(
        await_response(
            "moonramp_program::http",
            state.timeout,
            start,
            &state.registry_tx,
            id,
            msg,
            "GET",
            &format!("/{}/{}", hash_or_name, program_hash_or_name),
        )
        .await
        .map_err(|err| err.downcast().unwrap_or(HttpError::ServerError))?,
    ))
}

#[get("/ping")]
async fn ping() -> impl Responder {
    HttpResponse::Ok().body("pong\r\n")
}

impl Deref for ProgramHttpServer {
    type Target = Server;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for ProgramHttpServer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{
        body::MessageBody,
        http::{
            self,
            header::{ContentType, AUTHORIZATION},
        },
        test,
        web::Bytes,
    };
    use sea_orm::Database;
    use tokio::sync::mpsc;

    use moonramp_core::{
        NetworkTunnel, NetworkTunnelChannel, NetworkTunnelReceiver, NodeId, RpcTunnel, TunnelName,
        TunnelTopic,
    };
    use moonramp_migration::testing::setup_testdb;

    async fn stub_registry(mut r_rx: NetworkTunnelReceiver) {
        tokio::spawn(async move {
            if let Some((response_tx, _msg)) = r_rx.recv().await {
                let tunnel_msg = RpcTunnel {
                    uuid: "12345".to_string(),
                    sender: Sender::Node(NodeId::from("test")),
                    target: None,
                    data: json!({
                        "jsonrpc": "2.0",
                        "result": true,
                        "id": "12345",
                    }),
                };
                let msg = NetworkTunnel {
                    topic: TunnelTopic::Private(TunnelName::Program),
                    tunnel_data: serde_json::to_vec(&tunnel_msg).expect("Invalid tunnel_msg"),
                };
                match response_tx {
                    NetworkTunnelChannel::Oneshot(tx) => tx.send(msg).expect("oneshot failed"),
                    NetworkTunnelChannel::Mpsc(tx) => tx.send(msg).await.expect("mpsc failed"),
                }
            }
        });
    }

    #[actix_web::test]
    async fn test_jsonrpc_ok() {
        let database = Database::connect("sqlite::memory:")
            .await
            .expect("Failed to open in-memory sqlite db");
        let t = setup_testdb(&database)
            .await
            .expect("Failed to setup testdb");

        let (r_tx, r_rx) = mpsc::channel(1);

        let test_data = web::Data::new(ProgramHttpServerData {
            timeout: Duration::from_millis(5),
            database,
            registry_tx: r_tx,
        });

        let app = test::init_service(
            App::new().service(
                web::scope("/jsonrpc")
                    .app_data(test_data)
                    .guard(guard::Header("content-type", "application/json"))
                    .service(jsonrpc),
            ),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/jsonrpc")
            .insert_header((AUTHORIZATION, format!("Bearer {}", t.token)))
            .set_json(json!({
                "jsonrpc": "2.0",
                "method": "program.lookup",
                "params": {},
                "id": "12345",
            }))
            .to_request();

        stub_registry(r_rx).await;

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), http::StatusCode::OK);
        assert_eq!(
            resp.into_body().try_into_bytes().ok(),
            Some(Bytes::from(
                "{\"id\":\"12345\",\"jsonrpc\":\"2.0\",\"result\":true}"
            ))
        );
    }

    #[actix_web::test]
    async fn test_jsonrpc_not_ok() {
        let database = Database::connect("sqlite::memory:")
            .await
            .expect("Failed to open in-memory sqlite db");
        let t = setup_testdb(&database)
            .await
            .expect("Failed to setup testdb");

        let (r_tx, _r_rx) = mpsc::channel(1);

        let test_data = web::Data::new(ProgramHttpServerData {
            timeout: Duration::from_millis(5),
            database,
            registry_tx: r_tx,
        });

        let app = test::init_service(
            App::new().service(
                web::scope("/jsonrpc")
                    .app_data(test_data)
                    .guard(guard::Header("content-type", "application/json"))
                    .service(jsonrpc),
            ),
        )
        .await;

        let req = test::TestRequest::post()
            .uri("/jsonrpc")
            .insert_header((AUTHORIZATION, format!("Bearer {}", t.token)))
            .set_json(json!({
                "jsonrpc": "2.0",
                "method": "program.test",
                "params": {},
                "id": "12345",
            }))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), http::StatusCode::UNAUTHORIZED);
        assert_eq!(
            resp.into_body().try_into_bytes().ok(),
            Some(Bytes::from(
                "{\"error\":\"unauthorized\",\"id\":\"-\",\"jsonrpc\":\"2.0\"}"
            ))
        );

        let req = test::TestRequest::post()
            .uri("/jsonrpc")
            .insert_header((AUTHORIZATION, "Bearer BAD_TOKEN".to_string()))
            .set_json(json!({
                "jsonrpc": "2.0",
                "method": "program.test",
                "params": {},
                "id": "12345",
            }))
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), http::StatusCode::UNAUTHORIZED);
        assert_eq!(
            resp.into_body().try_into_bytes().ok(),
            Some(Bytes::from(
                "{\"error\":\"unauthorized\",\"id\":\"-\",\"jsonrpc\":\"2.0\"}"
            ))
        );
    }

    #[actix_web::test]
    async fn test_version_ok() {
        let database = Database::connect("sqlite::memory:")
            .await
            .expect("Failed to open in-memory sqlite db");
        let _t = setup_testdb(&database)
            .await
            .expect("Failed to setup testdb");

        let (r_tx, r_rx) = mpsc::channel(1);

        let test_data = web::Data::new(ProgramHttpServerData {
            timeout: Duration::from_millis(5),
            database,
            registry_tx: r_tx,
        });

        let app = test::init_service(
            App::new().service(
                web::scope("/program")
                    .app_data(test_data)
                    .guard(guard::Header("content-type", "application/json"))
                    .service(program_version),
            ),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/program/version")
            .set_json(json!({
                "jsonrpc": "2.0",
                "method": "program.version",
                "params": {},
                "id": "12345",
            }))
            .to_request();

        stub_registry(r_rx).await;

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), http::StatusCode::OK);
        assert_eq!(
            resp.into_body().try_into_bytes().ok(),
            Some(Bytes::from(
                "{\"id\":\"12345\",\"jsonrpc\":\"2.0\",\"result\":true}"
            ))
        );
    }

    #[actix_web::test]
    async fn test_post_ok() {
        let database = Database::connect("sqlite::memory:")
            .await
            .expect("Failed to open in-memory sqlite db");
        let t = setup_testdb(&database)
            .await
            .expect("Failed to setup testdb");

        let (r_tx, r_rx) = mpsc::channel(1);

        let test_data = web::Data::new(ProgramHttpServerData {
            timeout: Duration::from_millis(5),
            database,
            registry_tx: r_tx,
        });

        let app = test::init_service(
            App::new().service(
                web::scope("/program")
                    .app_data(test_data)
                    .guard(guard::Header("content-type", "application/json"))
                    .service(program_post),
            ),
        )
        .await;

        let data = br#"
            (module
               (func $hello (import "" "hello"))
                 (func (export "run") (call $hello))
            )
        "#;
        let req = test::TestRequest::post()
            .uri("/program")
            .insert_header((AUTHORIZATION, format!("Bearer {}", t.token)))
            .set_json(
                serde_json::to_value(ProgramCreateRequest {
                    name: "test".to_string(),
                    version: "0.1.0".to_string(),
                    url: None,
                    description: None,
                    data: data.to_vec(),
                    private: true,
                })
                .expect("Invalid ProgramCreateRequest"),
            )
            .to_request();

        stub_registry(r_rx).await;

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), http::StatusCode::OK);
        assert_eq!(
            resp.into_body().try_into_bytes().ok(),
            Some(Bytes::from(
                "{\"id\":\"12345\",\"jsonrpc\":\"2.0\",\"result\":true}"
            ))
        );
    }

    #[actix_web::test]
    async fn test_post_not_ok() {
        let database = Database::connect("sqlite::memory:")
            .await
            .expect("Failed to open in-memory sqlite db");
        let _t = setup_testdb(&database)
            .await
            .expect("Failed to setup testdb");

        let (r_tx, _r_rx) = mpsc::channel(1);

        let test_data = web::Data::new(ProgramHttpServerData {
            timeout: Duration::from_millis(5),
            database,
            registry_tx: r_tx,
        });

        let app = test::init_service(
            App::new().service(
                web::scope("/program")
                    .app_data(test_data)
                    .guard(guard::Header("content-type", "application/json"))
                    .service(program_post),
            ),
        )
        .await;

        let data = br#"
            (module
               (func $hello (import "" "hello"))
                 (func (export "run") (call $hello))
            )
        "#;
        let req = test::TestRequest::post()
            .uri("/program")
            .insert_header((AUTHORIZATION, "Bearer BAD_TOKEN".to_string()))
            .set_json(
                serde_json::to_value(ProgramCreateRequest {
                    name: "test".to_string(),
                    version: "0.1.0".to_string(),
                    url: None,
                    description: None,
                    data: data.to_vec(),
                    private: true,
                })
                .expect("Invalid ProgramCreateRequest"),
            )
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), http::StatusCode::UNAUTHORIZED);
        assert_eq!(
            resp.into_body().try_into_bytes().ok(),
            Some(Bytes::from(
                "{\"error\":\"unauthorized\",\"id\":\"-\",\"jsonrpc\":\"2.0\"}"
            ))
        );
    }

    #[actix_web::test]
    async fn test_get_ok() {
        let database = Database::connect("sqlite::memory:")
            .await
            .expect("Failed to open in-memory sqlite db");
        let t = setup_testdb(&database)
            .await
            .expect("Failed to setup testdb");

        let (r_tx, r_rx) = mpsc::channel(1);

        let test_data = web::Data::new(ProgramHttpServerData {
            timeout: Duration::from_millis(5),
            database,
            registry_tx: r_tx,
        });

        let app = test::init_service(
            App::new().service(
                web::scope("/program")
                    .app_data(test_data)
                    .guard(guard::Header("content-type", "application/json"))
                    .service(program_get),
            ),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/program/hash/12345")
            .insert_header((AUTHORIZATION, format!("Bearer {}", t.token)))
            .insert_header(ContentType::json())
            .to_request();

        stub_registry(r_rx).await;

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), http::StatusCode::OK);
        assert_eq!(
            resp.into_body().try_into_bytes().ok(),
            Some(Bytes::from(
                "{\"id\":\"12345\",\"jsonrpc\":\"2.0\",\"result\":true}"
            ))
        );
    }

    #[actix_web::test]
    async fn test_get_not_ok() {
        let database = Database::connect("sqlite::memory:")
            .await
            .expect("Failed to open in-memory sqlite db");
        let t = setup_testdb(&database)
            .await
            .expect("Failed to setup testdb");

        let (r_tx, _r_rx) = mpsc::channel(1);

        let test_data = web::Data::new(ProgramHttpServerData {
            timeout: Duration::from_millis(5),
            database,
            registry_tx: r_tx,
        });

        let app = test::init_service(
            App::new().service(
                web::scope("/program")
                    .app_data(test_data)
                    .guard(guard::Header("content-type", "application/json"))
                    .service(program_get),
            ),
        )
        .await;

        let req = test::TestRequest::get()
            .uri("/program/hash/12345")
            .insert_header((AUTHORIZATION, format!("Bearer {}", t.token)))
            .insert_header(ContentType::json())
            .to_request();

        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), http::StatusCode::GATEWAY_TIMEOUT);
        assert_eq!(
            resp.into_body().try_into_bytes().ok(),
            Some(Bytes::from(
                "{\"error\":\"timeout\",\"id\":\"-\",\"jsonrpc\":\"2.0\"}"
            ))
        );
    }

    #[actix_web::test]
    async fn test_ping_ok() {
        let app = test::init_service(App::new().service(ping)).await;

        let req = test::TestRequest::get()
            .uri("/ping")
            .insert_header(ContentType::plaintext())
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), http::StatusCode::OK);
        assert_eq!(
            resp.into_body().try_into_bytes().ok(),
            Some(Bytes::from("pong\r\n"))
        );
    }
}
