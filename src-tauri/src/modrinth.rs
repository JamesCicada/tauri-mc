use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::Path;
use tauri::AppHandle;

const MODRINTH_API: &str = "https://api.modrinth.com/v2";

/// ----------------------------
/// Loader handling
/// ----------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModLoader {
    Fabric,
    Quilt,
    Forge,
    NeoForge,
}

impl ModLoader {
    pub fn as_str(&self) -> &'static str {
        match self {
            ModLoader::Fabric => "fabric",
            ModLoader::Quilt => "quilt",
            ModLoader::Forge => "forge",
            ModLoader::NeoForge => "neoforge",
        }
    }
}

/// ----------------------------
/// Modrinth API models
/// ----------------------------

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModrinthSearchResult {
    pub hits: Vec<ModrinthProjectHit>,
    pub total_hits: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModrinthProjectHit {
    pub project_id: String,
    pub title: String,
    pub description: String,
    pub icon_url: Option<String>,
    pub author: String,
    pub categories: Vec<String>,
    pub project_type: String,
    pub latest_version: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModrinthVersion {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub version_number: String,
    pub dependencies: Vec<ModrinthDependency>,
    pub game_versions: Vec<String>,
    pub loaders: Vec<String>,
    pub files: Vec<ModrinthFile>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModrinthDependency {
    pub version_id: Option<String>,
    pub project_id: Option<String>,
    pub dependency_type: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModrinthFile {
    pub url: String,
    pub filename: String,
    pub primary: bool,
    pub size: u32,
}

/// ----------------------------
/// Modpack (.mrpack) models
/// ----------------------------

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModpackIndex {
    pub format_version: Option<u32>,
    pub game: String,
    #[serde(alias = "version_Id")]
    pub version_id: Option<String>,
    pub name: String,
    pub summary: Option<String>,
    pub files: Vec<ModpackFile>,
    pub dependencies: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModpackFile {
    pub path: String,
    pub hashes: HashMap<String, String>,
    pub env: Option<ModpackEnv>,
    pub downloads: Vec<String>,
    pub file_size: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModpackEnv {
    pub client: String,
    pub server: String,
}

/// ----------------------------
/// HTTP client
/// ----------------------------

fn get_client() -> reqwest::Client {
    let mut headers = HeaderMap::new();
    headers.insert(
        USER_AGENT,
        HeaderValue::from_static("tauri-mc-launcher/1.0.0"),
    );

    reqwest::Client::builder()
        .default_headers(headers)
        .build()
        .unwrap()
}

/// ----------------------------
/// Modrinth search & fetch
/// ----------------------------

pub async fn search_projects(
    query: &str,
    project_type: &str,
) -> Result<ModrinthSearchResult, String> {
    let url = format!(
        "{}/search?query={}&facets=[[\"project_type:{}\"]]",
        MODRINTH_API, query, project_type
    );

    get_client()
        .get(url)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())
}

pub async fn get_project_versions(project_id: &str) -> Result<Vec<ModrinthVersion>, String> {
    let url = format!("{}/project/{}/version", MODRINTH_API, project_id);

    get_client()
        .get(url)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())
}

/// Fetch a single version by its Modrinth version ID.
pub async fn get_version(version_id: &str) -> Result<ModrinthVersion, String> {
    let url = format!("{}/version/{}", MODRINTH_API, version_id);

    get_client()
        .get(url)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())
}

/// Fetch popular mods (sorted by downloads). Used for discovery.
pub async fn get_popular_mods(limit: usize) -> Result<ModrinthSearchResult, String> {
    let limit = limit.min(100);
    let url = format!(
        "{}/search?facets=[[\"project_type:mod\"]]&limit={}&index=downloads",
        MODRINTH_API, limit
    );

    get_client()
        .get(url)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())
}

/// ----------------------------
/// Version resolution logic
/// ----------------------------

pub fn filter_compatible_versions(
    versions: Vec<ModrinthVersion>,
    mc_version: &str,
    loader: ModLoader,
) -> Vec<ModrinthVersion> {
    versions
        .into_iter()
        .filter(|v| {
            v.game_versions.iter().any(|gv| gv == mc_version)
                && v.loaders.iter().any(|l| l == loader.as_str())
        })
        .collect()
}

pub fn pick_best_version(versions: &[ModrinthVersion]) -> Option<ModrinthVersion> {
    versions.first().cloned()
}

pub async fn resolve_mod_version(
    project_id: &str,
    mc_version: &str,
    loader: ModLoader,
) -> Result<ModrinthVersion, String> {
    let versions = get_project_versions(project_id).await?;
    let compatible = filter_compatible_versions(versions, mc_version, loader);

    pick_best_version(&compatible).ok_or_else(|| "No compatible mod version found".to_string())
}

pub async fn list_compatible_versions(
    project_id: &str,
    mc_version: &str,
    loader: ModLoader,
) -> Result<Vec<ModrinthVersion>, String> {
    let versions = get_project_versions(project_id).await?;
    Ok(filter_compatible_versions(versions, mc_version, loader))
}

/// ----------------------------
/// File selection
/// ----------------------------

pub fn select_primary_file(version: &ModrinthVersion) -> Result<&ModrinthFile, String> {
    version
        .files
        .iter()
        .find(|f| f.primary)
        .or_else(|| version.files.first())
        .ok_or_else(|| "No downloadable file found".to_string())
}

/// ----------------------------
/// .mrpack handling (unchanged logic, cleaned)
/// ----------------------------

pub fn parse_mrpack_index(mrpack_path: &Path) -> Result<ModpackIndex, String> {
    let file = fs::File::open(mrpack_path).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;

    let mut index_content = String::new();
    archive
        .by_name("modrinth.index.json")
        .map_err(|e| e.to_string())?
        .read_to_string(&mut index_content)
        .map_err(|e| e.to_string())?;

    serde_json::from_str(&index_content).map_err(|e| e.to_string())
}

pub async fn install_mrpack(
    app: &AppHandle,
    instance_id: &str,
    mrpack_path: &Path,
) -> Result<ModpackIndex, String> {
    let index = parse_mrpack_index(mrpack_path)?;
    let root = crate::commands::instance_dir(app, instance_id)?;
    let mc_dir = root.join(".minecraft");

    for file in &index.files {
        let client_ok = file
            .env
            .as_ref()
            .map(|e| e.client != "unsupported")
            .unwrap_or(true);

        if !client_ok {
            continue;
        }

        let target = mc_dir.join(&file.path);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }

        let mut success = false;
        for url in &file.downloads {
            if crate::download::download_to_file(url, &target)
                .await
                .is_ok()
            {
                success = true;
                break;
            }
        }

        if !success {
            return Err(format!("Failed to download {}", file.path));
        }
    }

    Ok(index)
}
