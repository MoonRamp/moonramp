use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use serde_json::json;

use moonramp_core::{anyhow, serde, serde_json};

#[derive(Debug, Deserialize, Serialize)]
#[serde(crate = "moonramp_core::serde")]
pub struct JsonRpcOneDotZeroResult<T> {
    id: String,
    result: Option<T>,
    error: Option<serde_json::Value>,
}

impl<T> JsonRpcOneDotZeroResult<T> {
    pub fn inner(self) -> anyhow::Result<T> {
        match (self.result, self.error) {
            (Some(res), None) => Ok(res),
            (None, Some(err)) => Err(anyhow!(err.to_string())),
            _ => Err(anyhow!("Invalid Response")),
        }
    }
}
