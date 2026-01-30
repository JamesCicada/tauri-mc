use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AssetIndex {
    pub id: String,
    pub url: String,
    pub sha1: String,
    pub size: u64,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct VersionJson {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inheritsFrom: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub releaseTime: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<serde_json::Value>,
    pub libraries: Vec<Library>,
    pub downloads: Downloads,
    pub mainClass: String,
    pub assetIndex: AssetIndex,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Library {
    pub name: String,
    pub downloads: LibraryDownloads,
    #[serde(default)]
    pub natives: std::collections::HashMap<String, String>,
    #[serde(default)]
    pub rules: Vec<Rule>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct LibraryDownloads {
    pub artifact: Option<Artifact>,
    #[serde(default)]
    pub classifiers: std::collections::HashMap<String, Artifact>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Artifact {
    pub path: String,
    pub url: String,
    pub sha1: String,
    pub size: u64,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Rule {
    pub action: String,
    #[serde(default)]
    pub os: Option<OsRule>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct OsRule {
    pub name: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Downloads {
    pub client: DownloadInfo,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct DownloadInfo {
    pub url: String,
    pub sha1: String,
    pub size: u64,
}
