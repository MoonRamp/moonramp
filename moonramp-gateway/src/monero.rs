use std::io::Write;

use anyhow::anyhow;
use hyper::{
    http::header::{AUTHORIZATION, CONTENT_TYPE},
    Body, Client, Method, Request,
};
use jsonrpsee::{core::RpcResult, http_client::HttpClientBuilder, proc_macros::rpc};
use log::{debug, trace};
use monero_rpc::{GetBlockCountResponse, GetBlockResponse};
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;
use wasmtime::{Extern, FuncType, Linker, Trap, Val, ValType};
use wasmtime_wasi::WasiCtx;

use moonramp_core::{
    anyhow, hyper, jsonrpsee, log, monero_rpc, serde, serde_json, uuid, wasmtime, wasmtime_wasi,
};

#[rpc(client)]
pub trait MonerodRpc {
    #[method(name = "get_block_count")]
    async fn getBlockCount(&self) -> RpcResult<GetBlockCountResponse>;
    #[method(name = "get_block")]
    async fn getBlock(
        &self,
        hash: Option<String>,
        height: Option<u64>,
    ) -> RpcResult<GetBlockResponse>;
}

#[derive(Debug, Clone)]
pub struct MoneroRpcConfig {
    pub endpoint: String,
    pub basic_auth: Option<String>,
}

pub fn add_to_linker(config: MoneroRpcConfig, linker: &mut Linker<WasiCtx>) -> anyhow::Result<()> {
    linker.func_new_async(
        "env",
        "monero_gateway",
        FuncType::new([ValType::I32, ValType::I32], [ValType::I32]),
        move |mut caller, params, returns| {
            let config = config.clone();
            Box::new(async move {
                if let (Some(Val::I32(req_ptr)), Some(Val::I32(req_len))) =
                    (params.get(0), params.get(1))
                {
                    debug!("MoneroGatewayRequest: 0x{:02X} {} bytes", req_ptr, req_len);
                    let memory = match caller.get_export("memory") {
                        Some(Extern::Memory(mem)) => mem,
                        _ => return Err(Trap::new("failed to find memory")),
                    };
                    let data = memory
                        .data(&caller)
                        .get(*req_ptr as usize..)
                        .and_then(|arr| arr.get(..*req_len as usize));
                    let req: moonramp_lunar::gateway::MoneroGatewayRequest = match data {
                        Some(data) => match serde_json::from_slice(&data) {
                            Ok(exit_data) => exit_data,
                            Err(err) => {
                                return Err(Trap::new(format!(
                                    "Invalid MoneroGatewayRequest: {}",
                                    err
                                )))
                            }
                        },
                        None => return Err(Trap::new("pointer/length out of bounds")),
                    };

                    let client = HttpClientBuilder::default()
                        .build(&config.endpoint)
                        .map_err(|e| Trap::new(format!("Failed to build HttpClient {}", e)))?;
                    let res = match &req {
                        moonramp_lunar::gateway::MoneroGatewayRequest::GetBlockCount => {
                            let res: RpcResult<GetBlockCountResponse> =
                                client.getBlockCount().await;
                            trace!("REQUEST {:?}", res);
                            moonramp_lunar::gateway::MoneroGatewayResponse::GetBlockCount(
                                res.map_err(|err| {
                                    debug!("REQUEST ERROR {:?}", err);
                                    Trap::new(err.to_string())
                                })?,
                            )
                        }
                        moonramp_lunar::gateway::MoneroGatewayRequest::GetBlock(height) => {
                            let res: RpcResult<GetBlockResponse> =
                                client.getBlock(None, Some(*height)).await;
                            trace!("REQUEST {:?}", res);
                            moonramp_lunar::gateway::MoneroGatewayResponse::GetBlock(res.map_err(
                                |err| {
                                    debug!("REQUEST ERROR {:?}", err);
                                    Trap::new(err.to_string())
                                },
                            )?)
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
