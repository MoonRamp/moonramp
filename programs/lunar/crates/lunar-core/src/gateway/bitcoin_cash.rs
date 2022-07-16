use std::{os::raw::c_uchar, ptr};

use serde::{Deserialize, Serialize};

use moonramp_core::{serde, serde_json};

use crate::LunarError;

extern "C" {
    fn bitcoin_cash_gateway_scan(
        req_ptr: *mut c_uchar,
        req_len: usize,
        res_ptr: *mut c_uchar,
        res_len_ptr: *mut usize,
    );
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(crate = "moonramp_core::serde")]
pub enum BitcoinCashGatewayRequest {
    //ScanTxn { hash: String },
    ScanAddr { address: String },
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(crate = "moonramp_core::serde")]
pub enum BitcoinCashGatewayResponse {
    //ScanTxn { hash: String },
    ScanAddr {
        total_amount: String,
        txn_ids: Vec<String>,
    },
}

pub struct BitcoinCashGateway {}

impl BitcoinCashGateway {
    pub fn new() -> Self {
        BitcoinCashGateway {}
    }

    pub fn scan_addr(&self, address: String) -> Result<BitcoinCashGatewayResponse, LunarError> {
        let req = BitcoinCashGatewayRequest::ScanAddr { address };
        let mut req_json =
            serde_json::to_vec(&req).map_err(|e| LunarError::Serde(e.to_string()))?;
        let req_len = req_json.len();
        let req_ptr = req_json.as_mut_ptr();
        let res_ptr = ptr::null_mut();
        let res_len_ptr = ptr::null_mut();

        unsafe { bitcoin_gateway_scan(req_ptr as *mut c_uchar, req_len, res_ptr, res_len_ptr) };

        if res_ptr == ptr::null_mut() || res_len_ptr == ptr::null_mut() {
            Err(LunarError::Crash(
                "Call to bitcoin_gateway_scan failed".to_string(),
            ))
        } else {
            let res_json = unsafe { Vec::from_raw_parts(res_ptr, *res_len_ptr, *res_len_ptr) };
            Ok(serde_json::from_slice(&res_json).map_err(|e| LunarError::Serde(e.to_string()))?)
        }
    }
}
