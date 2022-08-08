use std::{io::Write, sync::Arc};

use anyhow::anyhow;
use log::{debug, warn};
use tokio::time::{sleep, Duration, Instant};
use wasmtime::{Config, Engine, Extern, FuncType, Linker, Module, Store, Trap, Val, ValType};
use wasmtime_wasi::{tokio::WasiCtxBuilder, WasiCtx};

use moonramp_core::{anyhow, log, serde_json, tokio, wasmtime, wasmtime_wasi};
pub use moonramp_gateway::bitcoin::BitcoinRpcConfig;

const TABLE_EXIT_DATA: u32 = 10;

struct State {
    engine: Engine,
    module: Module,
    linker: Arc<Linker<WasiCtx>>,
}

impl State {
    pub fn new(
        wasm_mod_bytes: &[u8],
        bitcoin_gateway_config: BitcoinRpcConfig,
    ) -> anyhow::Result<Self> {
        let config = State::config();
        let engine = Engine::new(&config)?;
        let module = unsafe { Module::deserialize(&engine, &wasm_mod_bytes)? };

        let mut linker = Linker::new(&engine);
        wasmtime_wasi::tokio::add_to_linker(&mut linker, |cx| cx)?;

        linker.func_new_async(
            "env",
            "lunar_ptr_len",
            FuncType::new([ValType::I32], [ValType::I32]),
            |mut caller, params, results| {
                Box::new(async move {
                    if let Some(Val::I32(ptr)) = params.get(0) {
                        debug!("Ptr: 0x{:02X}", ptr);
                        let wasi: &mut WasiCtx = caller.data_mut();
                        let ptr_len: i32 = wasi
                            .table()
                            .delete(*ptr as u32)
                            .and_then(|ptr| ptr.downcast::<i32>().ok())
                            .map(|ptr| *ptr)
                            .ok_or(Trap::new("Invalid ptr"))?;
                        results[0] = Val::I32(ptr_len);
                    }
                    Ok(())
                })
            },
        )?;

        linker.func_new_async(
            "env",
            "lunar_exit",
            FuncType::new([ValType::I32, ValType::I32], None),
            |mut caller, params, _| {
                Box::new(async move {
                    if let (Some(Val::I32(exit_data_ptr)), Some(Val::I32(exit_data_len))) =
                        (params.get(0), params.get(1))
                    {
                        debug!("ExitData: 0x{:02X} {} bytes", exit_data_ptr, exit_data_len);
                        let memory = match caller.get_export("memory") {
                            Some(Extern::Memory(mem)) => mem,
                            _ => return Err(Trap::new("Failed to find memory")),
                        };
                        let data = memory
                            .data(&caller)
                            .get(*exit_data_ptr as usize..)
                            .and_then(|arr| arr.get(..*exit_data_len as usize));
                        let prgm_exit: Result<
                            moonramp_lunar::ExitData,
                            moonramp_lunar::LunarError,
                        > = match data {
                            Some(data) => match serde_json::from_slice(&data) {
                                Ok(exit_data) => exit_data,
                                Err(err) => {
                                    return Err(Trap::new(format!("Invalid ExitData: {}", err)))
                                }
                            },
                            None => return Err(Trap::new("pointer/length out of bounds")),
                        };
                        let wasi: &mut WasiCtx = caller.data_mut();
                        wasi.table().insert_at(TABLE_EXIT_DATA, Box::new(prgm_exit));
                    }
                    Ok(())
                })
            },
        )?;

        moonramp_gateway::bitcoin::add_to_linker(bitcoin_gateway_config, &mut linker)?;

        Ok(State {
            engine,
            module,
            linker: Arc::new(linker),
        })
    }

    pub fn config() -> Config {
        let mut config = Config::new();
        config.async_support(true);
        config.consume_fuel(true);
        config
    }
}

pub struct Runtime;

impl Runtime {
    pub fn compile(wasm_bytes: &[u8]) -> anyhow::Result<Vec<u8>> {
        let config = State::config();
        let engine = Engine::new(&config)?;
        let module = Module::new(&engine, wasm_bytes)?;
        Ok(module.serialize()?)
    }

    pub async fn exec(
        wasm_mod_bytes: &[u8],
        entry_data: moonramp_lunar::EntryData,
        timeout: Duration,
        bitcoin_gateway_config: BitcoinRpcConfig,
    ) -> anyhow::Result<moonramp_lunar::ExitData> {
        let state = State::new(wasm_mod_bytes, bitcoin_gateway_config)?;

        let wasi = WasiCtxBuilder::new().inherit_stdout().build();
        let mut store = Store::new(&state.engine, wasi);
        store.out_of_fuel_async_yield(100, 10_000_000);

        let instance = state
            .linker
            .instantiate_async(&mut store, &state.module)
            .await?;

        let memory = instance
            .get_memory(&mut store, "memory")
            .ok_or(anyhow!("Program does not export memory"))?;

        let moonramp_lunar_main_fn =
            instance.get_typed_func::<(i32, i32), i32, _>(&mut store, "lunar_main")?;
        let moonramp_lunar_alloc_fn =
            instance.get_typed_func::<i32, i32, _>(&mut store, "lunar_allocate")?;
        let _moonramp_lunar_dealloc_fn =
            instance.get_typed_func::<(i32, i32), (), _>(&mut store, "lunar_deallocate")?;

        let start = Instant::now();
        let entry_data_json = serde_json::to_vec(&entry_data)?;
        let entry_data_ptr = moonramp_lunar_alloc_fn
            .call_async(&mut store, entry_data_json.len() as i32)
            .await?;

        let data = memory
            .data_mut(&mut store)
            .get_mut(entry_data_ptr as usize..)
            .and_then(|arr| arr.get_mut(..entry_data_json.len()));
        match data {
            Some(mut data) => {
                data.write(&entry_data_json)?;
            }
            None => return Err(anyhow!("pointer/length out of bounds")),
        }

        let res_timeout = sleep(timeout);
        tokio::pin!(res_timeout);
        tokio::select! {
            _ = &mut res_timeout => {
                warn!(
                    "Program TIMEOUT {}ms Fuel {}",
                    start.elapsed().as_millis(),
                    store.fuel_consumed().unwrap_or(0),
                );
                Err(anyhow!("Program TIMEOUT"))
            }
            Ok(res) = moonramp_lunar_main_fn.call_async(&mut store, (entry_data_ptr, entry_data_json.len() as i32)) => {
                debug!(
                    "Program exit code: {:?} {}ms Fuel {}",
                    res,
                    start.elapsed().as_millis(),
                    store.fuel_consumed().unwrap_or(0),
                );
                let prgm_exit: Result<moonramp_lunar::ExitData, moonramp_lunar::LunarError> = store
                    .data_mut()
                    .table()
                    .delete(TABLE_EXIT_DATA)
                    .and_then(|prgm_exit| {
                        prgm_exit
                            .downcast::<Result<moonramp_lunar::ExitData, moonramp_lunar::LunarError>>()
                            .ok()
                    })
                    .map(|prgm_exit| *prgm_exit)
                    .ok_or(anyhow!("Program did not write exit data"))?;
                Ok(prgm_exit.map_err(|err| anyhow!(err))?)
            }
        }
    }
}
