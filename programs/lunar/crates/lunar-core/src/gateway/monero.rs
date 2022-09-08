use std::{os::raw::c_uchar, ptr};

use monero_rpc::{GetBlockCountResponse, GetBlockResponse};
use serde::{Deserialize, Serialize};

use moonramp_core::{monero_rpc, serde, serde_json};

use crate::{lunar_ptr_len, LunarError};

extern "C" {
    fn monero_gateway(req_ptr: *mut c_uchar, req_len: usize) -> *mut c_uchar;
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(crate = "moonramp_core::serde")]
pub enum MoneroGatewayRequest {
    GetBlockCount,
    GetBlock(u64),
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(crate = "moonramp_core::serde")]
pub enum MoneroGatewayResponse {
    GetBlockCount(GetBlockCountResponse),
    GetBlock(GetBlockResponse),
}

pub struct MoneroGateway {}

impl MoneroGateway {
    pub fn new() -> Self {
        MoneroGateway {}
    }

    pub fn current_height(&self) -> Result<MoneroGatewayResponse, LunarError> {
        let req = MoneroGatewayRequest::GetBlockCount;
        let mut req_json =
            serde_json::to_vec(&req).map_err(|e| LunarError::Serde(e.to_string()))?;
        let req_len = req_json.len();
        let req_ptr = req_json.as_mut_ptr();

        let res_ptr = unsafe { monero_gateway(req_ptr as *mut c_uchar, req_len) };

        if res_ptr == ptr::null_mut() {
            Err(LunarError::Crash(
                "Call to bitcoin_gateway failed".to_string(),
            ))
        } else {
            let res_json = unsafe {
                let res_len = lunar_ptr_len(res_ptr as *mut c_uchar);
                Vec::from_raw_parts(res_ptr, res_len, res_len)
            };
            Ok(serde_json::from_slice(&res_json).map_err(|e| LunarError::Serde(e.to_string()))?)
        }
    }

    pub fn scan_blocks(&self, height: u64) -> Result<MoneroGatewayResponse, LunarError> {
        let req = MoneroGatewayRequest::GetBlock(height);
        let mut req_json =
            serde_json::to_vec(&req).map_err(|e| LunarError::Serde(e.to_string()))?;
        let req_len = req_json.len();
        let req_ptr = req_json.as_mut_ptr();

        let res_ptr = unsafe { monero_gateway(req_ptr as *mut c_uchar, req_len) };

        if res_ptr == ptr::null_mut() {
            Err(LunarError::Crash(
                "Call to bitcoin_gateway failed".to_string(),
            ))
        } else {
            let res_json = unsafe {
                let res_len = lunar_ptr_len(res_ptr as *mut c_uchar);
                Vec::from_raw_parts(res_ptr, res_len, res_len)
            };
            Ok(serde_json::from_slice(&res_json).map_err(|e| LunarError::Serde(e.to_string()))?)
        }
    }
}
