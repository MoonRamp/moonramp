#![deny(missing_docs)]

//! moonramp-core re-exports all pacakges shared by other moonramp crates and defines a number of core types.

#[cfg(feature = "http")]
pub use actix_cors;
#[cfg(feature = "http")]
pub use actix_rt;
#[cfg(feature = "http")]
pub use actix_web;
#[cfg(feature = "http")]
pub use actix_web_httpauth;
#[cfg(feature = "crypto")]
pub use aes_gcm_siv;
pub use anyhow;
#[cfg(feature = "async-core")]
pub use async_trait;
#[cfg(feature = "http")]
pub use awc;
#[cfg(feature = "crypto-currency-bitcoin")]
pub use bip39;
#[cfg(feature = "crypto-currency-bitcoin")]
pub use bitcoin;
#[cfg(feature = "crypto-currency-bitcoin-rpc")]
pub use bitcoincore_rpc_json;
#[cfg(feature = "crypto")]
pub use bs58;
#[cfg(feature = "crypto")]
pub use chacha20poly1305;
#[cfg(feature = "time")]
pub use chrono;
#[cfg(feature = "async-core")]
pub use futures;
#[cfg(feature = "http")]
pub use hyper;
#[cfg(feature = "jsonrpc")]
pub use jsonrpsee;
#[cfg(feature = "lib")]
pub use log;
#[cfg(feature = "random")]
pub use rand;
#[cfg(feature = "sql")]
pub use sea_orm;
#[cfg(feature = "serialization")]
pub use serde;
#[cfg(feature = "serialization")]
pub use serde_json;
#[cfg(feature = "crypto")]
pub use sha3;
//#[cfg(feature = "sql")]
//pub use sqlx;
#[cfg(feature = "async-core")]
pub use tokio;
#[cfg(feature = "random")]
pub use uuid;
#[cfg(feature = "wasm")]
pub use wasmtime;
#[cfg(feature = "wasm")]
pub use wasmtime_wasi;

//#[cfg(feature = "async-core")]
//pub use async_stream;
//#[cfg(feature = "async-core")]
//pub use tokio_serde;
//#[cfg(feature = "async-core")]
//pub use tokio_stream;
//#[cfg(feature = "async-core")]
//pub use tokio_util;

#[cfg(feature = "entity")]
pub use moonramp_entity as entity;
#[cfg(feature = "gossip")]
pub use moonramp_gossip as gossip;
#[cfg(feature = "raft")]
pub use moonramp_raft as raft;
#[cfg(feature = "transaction")]
pub use moonramp_transaction as transaction;
#[cfg(feature = "wallet")]
pub use moonramp_wallet as wallet;

mod types;
pub use types::*;
