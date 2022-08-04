use std::{
    error::Error,
    fmt, mem,
    os::raw::{c_uchar, c_void},
};

use serde::{Deserialize, Serialize};

pub use moonramp_core;
pub use moonramp_wallet;

pub use wee_alloc;

use moonramp_core::{serde, serde_json};
use moonramp_wallet::{Currency, Wallet};

pub mod gateway;

extern "C" {
    fn lunar_ptr_len(ptr: *mut c_uchar) -> usize;
    fn lunar_exit(exit_data_ptr: *mut c_uchar, size: usize);
}

pub enum LunarExitCode {
    Success = 0,
    Failed = 1,
    Panic = -1,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(crate = "moonramp_core::serde")]
pub enum LunarError {
    Crash(String),
    Serde(String),
    Wallet(String),
}

impl fmt::Display for LunarError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Error for LunarError {}

#[derive(Debug, Deserialize, Serialize)]
#[serde(crate = "moonramp_core::serde")]
pub enum EntryData {
    Invoice {
        wallet: Wallet,
        currency: Currency,
        amount: u64,
        user_data: Option<Vec<u8>>,
    },
    Sale {
        wallet: Wallet,
        currency: Currency,
        amount: u64,
        address: String,
        confirmations: i64,
        user_data: Option<Vec<u8>>,
    },
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(crate = "moonramp_core::serde")]
pub enum ExitData {
    Invoice {
        wallet: Wallet,
        pubkey: String,
        address: String,
        uri: String,
        user_data: Option<Vec<u8>>,
    },
    Sale {
        funded: bool,
        amount: u64,
        user_data: Option<Vec<u8>>,
    },
}

pub trait Program: Default {
    fn launch(self, entry_data: EntryData) -> Result<ExitData, LunarError>;
}

pub fn run_lunar_program<P: Program>(
    entry_data_json: &[u8],
    prgm: P,
) -> Result<ExitData, LunarError> {
    let entry_data: EntryData =
        serde_json::from_slice(entry_data_json).map_err(|e| LunarError::Serde(e.to_string()))?;
    prgm.launch(entry_data)
}

pub fn lunar_core_main<P: Program>(entry_data_ptr: *mut c_uchar, size: usize, prgm: P) -> i32 {
    let entry_data = unsafe { Vec::from_raw_parts(entry_data_ptr, size, size) };
    let res = run_lunar_program(entry_data.as_ref(), prgm);

    match (res.is_ok(), serde_json::to_vec(&res)) {
        (true, Ok(mut exit_data_json)) => {
            let exit_data_len = exit_data_json.len();
            let exit_data_ptr = exit_data_json.as_mut_ptr();
            mem::forget(exit_data_json);
            unsafe { lunar_exit(exit_data_ptr as *mut c_uchar, exit_data_len) };
            LunarExitCode::Success as i32
        }
        (false, Ok(mut exit_data_json)) => {
            let exit_data_len = exit_data_json.len();
            let exit_data_ptr = exit_data_json.as_mut_ptr();
            mem::forget(exit_data_json);
            unsafe { lunar_exit(exit_data_ptr as *mut c_uchar, exit_data_len) };
            LunarExitCode::Failed as i32
        }
        (_, Err(err)) => {
            println!("Lunar-Program Res {:?} Error {:?}", res, err);
            LunarExitCode::Failed as i32
        }
    }
}

pub fn lunar_core_allocate(size: usize) -> *mut c_void {
    let mut buf = Vec::with_capacity(size);
    let ptr = buf.as_mut_ptr();
    mem::forget(buf);
    ptr as *mut c_void
}

pub fn lunar_core_deallocate(ptr: *mut c_void, size: usize) {
    unsafe {
        let _drop = Vec::from_raw_parts(ptr as *mut c_uchar, size, size);
    }
}
