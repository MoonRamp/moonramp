use std::io::Write;

use anyhow::anyhow;
use bitcoincore_rpc_json::ScanTxOutResult;
use hyper::{
    http::header::{AUTHORIZATION, CONTENT_TYPE},
    Body, Client, Method, Request,
};
use log::{debug, trace};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;
use wasmtime::{Extern, FuncType, Linker, Trap, Val, ValType};
use wasmtime_wasi::WasiCtx;

use moonramp_core::{
    anyhow, bitcoincore_rpc_json, hyper, log, serde, serde_json, uuid, wasmtime, wasmtime_wasi,
};

#[derive(Debug, Clone)]
pub struct BitcoinRpcConfig {
    pub endpoint: String,
    pub basic_auth: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(crate = "moonramp_core::serde")]
struct JsonRpcOneDotZeroResult<T> {
    id: String,
    result: Option<T>,
    error: Option<serde_json::Value>,
}

impl<T> JsonRpcOneDotZeroResult<T> {
    fn inner(self) -> anyhow::Result<T> {
        match (self.result, self.error) {
            (Some(res), None) => Ok(res),
            (None, Some(err)) => Err(anyhow!(err.to_string())),
            _ => Err(anyhow!("Invalid Response")),
        }
    }
}

pub fn add_to_linker(config: BitcoinRpcConfig, linker: &mut Linker<WasiCtx>) -> anyhow::Result<()> {
    linker.func_new_async(
        "env",
        "bitcoin_gateway",
        FuncType::new([ValType::I32, ValType::I32], [ValType::I32]),
        move |mut caller, params, returns| {
            let config = config.clone();
            Box::new(async move {
                if let (Some(Val::I32(req_ptr)), Some(Val::I32(req_len))) =
                    (params.get(0), params.get(1))
                {
                    debug!("BitcoinGatewayRequest: 0x{:02X} {} bytes", req_ptr, req_len);
                    let memory = match caller.get_export("memory") {
                        Some(Extern::Memory(mem)) => mem,
                        _ => return Err(Trap::new("failed to find memory")),
                    };
                    let data = memory
                        .data(&caller)
                        .get(*req_ptr as usize..)
                        .and_then(|arr| arr.get(..*req_len as usize));
                    let req: lunar::gateway::BitcoinGatewayRequest = match data {
                        Some(data) => match serde_json::from_slice(&data) {
                            Ok(exit_data) => exit_data,
                            Err(err) => {
                                return Err(Trap::new(format!(
                                    "Invalid BitcoinGatewayRequest: {}",
                                    err
                                )))
                            }
                        },
                        None => return Err(Trap::new("pointer/length out of bounds")),
                    };

                    let res = match &req {
                        lunar::gateway::BitcoinGatewayRequest::ScanTxOut(req) => {
                            let res: anyhow::Result<JsonRpcOneDotZeroResult<ScanTxOutResult>> =
                                json_rpc_request(
                                    &config,
                                    "scantxoutset",
                                    json!({
                                        "action": "start",
                                        "scanobjects": req,
                                    }),
                                )
                                .await;
                            trace!("REQUEST {:?}", res);
                            lunar::gateway::BitcoinGatewayResponse::ScanTxOut(
                                res?.inner().map_err(|err| {
                                    debug!("REQUEST ERROR {:?}", err);
                                    Trap::new(err.to_string())
                                })?,
                            )
                        }
                    };

                    debug!("RESPONSE {:?}", res);

                    let lunar_alloc_fn = caller
                        .get_export("lunar_allocate")
                        .ok_or(anyhow!("lunar_allocate not found"))?
                        .into_func()
                        .ok_or(anyhow!("lunar_allocate not func"))?
                        .typed::<i32, i32, _>(&mut caller)?;

                    let res_json =
                        serde_json::to_vec(&res).map_err(|err| Trap::new(err.to_string()))?;
                    let res_json_ptr = lunar_alloc_fn
                        .call_async(&mut caller, res_json.len() as i32)
                        .await?;

                    let data = memory
                        .data_mut(&mut caller)
                        .get_mut(res_json_ptr as usize..)
                        .and_then(|arr| arr.get_mut(..res_json.len()));
                    match data {
                        Some(mut data) => {
                            data.write(&res_json)
                                .map_err(|err| Trap::new(err.to_string()))?;
                        }
                        None => return Err(Trap::new("pointer/length out of bounds")),
                    }

                    debug!("Res JSON Ptr 0x{:02X}", res_json_ptr);
                    returns[0] = Val::I32(res_json_ptr);
                    let wasi: &mut WasiCtx = caller.data_mut();
                    wasi.table()
                        .insert_at(res_json_ptr as u32, Box::new(res_json.len() as i32));
                }
                Ok(())
            })
        },
    )?;

    Ok(())
}

async fn json_rpc_request<T: for<'a> serde::de::Deserialize<'a>>(
    config: &BitcoinRpcConfig,
    method: &str,
    params: serde_json::Value,
) -> anyhow::Result<T> {
    let id = Uuid::new_v4().to_simple().to_string();
    let json_rpc = json!({
        "jsonrpc": "1.0",
        "method": method,
        "params": params,
        "id": id,
    });
    trace!("REQUEST {}", json_rpc);

    let json_bytes = serde_json::to_vec(&json_rpc)?;
    let req = Request::builder()
        .method(Method::POST)
        .uri(&config.endpoint)
        .header(
            AUTHORIZATION,
            format!(
                "Basic {}",
                config.basic_auth.as_ref().unwrap_or(&"".to_string())
            ),
        )
        .header(CONTENT_TYPE, "application/json")
        .body(Body::from(json_bytes))?;

    let res = Client::new().request(req).await?;
    trace!("RESPONSE {}", res.status());
    Ok(serde_json::from_slice(
        &hyper::body::to_bytes(res.into_body()).await?,
    )?)
}
