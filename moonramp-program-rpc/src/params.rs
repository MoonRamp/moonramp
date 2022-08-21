use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use moonramp_core::{chrono, serde, Hash};
use moonramp_entity::program;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(crate = "moonramp_core::serde", rename_all = "camelCase")]
pub struct ProgramCreateRequest {
    pub name: String,
    pub version: String,
    pub url: Option<String>,
    pub description: Option<String>,
    pub data: Vec<u8>,
    pub private: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(crate = "moonramp_core::serde", rename_all = "camelCase")]
pub struct ProgramUpdateRequest {
    pub name: String,
    pub version: String,
    pub url: Option<String>,
    pub description: Option<String>,
    pub data: Vec<u8>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(crate = "moonramp_core::serde", rename_all = "camelCase", untagged)]
pub enum ProgramLookupRequest {
    Hash { hash: Hash },
    Name { name: String },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(crate = "moonramp_core::serde", rename_all = "camelCase")]
pub struct ProgramResponse {
    pub hash: Hash,
    pub name: String,
    pub version: String,
    pub url: Option<String>,
    pub description: Option<String>,
    pub private: bool,
    pub revision: i64,
    pub created_at: DateTime<Utc>,
}

impl From<program::Model> for ProgramResponse {
    fn from(model: program::Model) -> ProgramResponse {
        ProgramResponse {
            hash: model.hash,
            name: model.name,
            version: model.version,
            url: model.url,
            description: model.description,
            private: model.private,
            revision: model.revision,
            created_at: model.created_at,
        }
    }
}
