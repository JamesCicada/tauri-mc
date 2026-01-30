use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Instance {
    pub id: String,
    pub name: String,
    pub version: String,
    pub state: InstanceState,
    pub created_at: u64,
    pub last_played: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub java_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub java_path_override: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_memory: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_memory: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub java_args: Option<String>,
    #[serde(default)]
    pub java_warning_ignored: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub loader: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub loader_version: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum InstanceState {
    NotInstalled,
    Installing,
    Ready,
    Running,
    Error,
}
