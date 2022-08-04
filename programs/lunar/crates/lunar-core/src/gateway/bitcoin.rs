use std::{os::raw::c_uchar, ptr};

use bitcoincore_rpc_json::{ScanTxOutRequest, ScanTxOutResult};
use serde::{Deserialize, Serialize};

use moonramp_core::{bitcoincore_rpc_json, serde, serde_json};

use crate::{lunar_ptr_len, LunarError};

extern "C" {
    fn bitcoin_gateway(req_ptr: *mut c_uchar, req_len: usize) -> *mut c_uchar;
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(crate = "moonramp_core::serde")]
pub enum BitcoinGatewayRequest {
    ScanTxOut(Vec<ScanTxOutRequest>),
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(crate = "moonramp_core::serde")]
pub enum BitcoinGatewayResponse {
    //ScanTxn { hash: String },
    ScanTxOut(ScanTxOutResult),
}

pub struct BitcoinGateway {}

impl BitcoinGateway {
    pub fn new() -> Self {
        BitcoinGateway {}
    }

    pub fn scan_tx_out(
        &self,
        descriptors: Vec<String>,
    ) -> Result<BitcoinGatewayResponse, LunarError> {
        let req = BitcoinGatewayRequest::ScanTxOut(
            descriptors
                .into_iter()
                .map(|descriptor| ScanTxOutRequest::Single(descriptor))
                .collect(),
        );
        let mut req_json =
            serde_json::to_vec(&req).map_err(|e| LunarError::Serde(e.to_string()))?;
        let req_len = req_json.len();
        let req_ptr = req_json.as_mut_ptr();

        let res_ptr = unsafe { bitcoin_gateway(req_ptr as *mut c_uchar, req_len) };

        if res_ptr == ptr::null_mut() {
            Err(LunarError::Crash(
                "Call to bitcoin_gateway_scan failed".to_string(),
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
