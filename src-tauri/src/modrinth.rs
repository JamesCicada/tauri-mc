use reqwest::header::{HeaderMap, HeaderValue, USER_AGENT};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Read;
use std::path::Path;
use tauri::AppHandle;

const MODRINTH_API: &str = "https://api.modrinth.com/v2";

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModrinthProject {
    pub id: String,
    pub title: String,
    pub description: String,
    pub body: String,
    pub icon_url: Option<String>,
    pub author: String,
    pub categories: Vec<String>,
    pub versions: Vec<String>,
    pub follows: u32,
    pub downloads: u32,
    pub project_type: String,
}

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
    pub gallery: Option<Vec<String>>,
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModpackIndex {
    #[serde(default)]
    pub format_version: Option<u32>,
    pub game: String,
    /// Some packs use `version_Id` — accept it as an alias for the mc version id
    #[serde(alias = "version_Id")]
    pub version_id: Option<String>,
    pub name: String,
    pub summary: Option<String>,
    pub files: Vec<ModpackFile>,
    pub dependencies: std::collections::HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModpackFile {
    pub path: String,
    pub hashes: std::collections::HashMap<String, String>,
    pub env: Option<ModpackEnv>,
    pub downloads: Vec<String>,
    // Some modpack indexes may omit file sizes or use different keys; make optional
    pub file_size: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModpackEnv {
    pub client: String,
    pub server: String,
}

fn get_client() -> reqwest::Client {
    let mut headers = HeaderMap::new();
    headers.insert(
        USER_AGENT,
        HeaderValue::from_static("tauri-mc-launcher/1.0.0 (contact@example.com)"),
    );
    reqwest::Client::builder()
        .default_headers(headers)
        .build()
        .unwrap()
}

pub async fn search_projects(
    query: &str,
    project_type: &str,
) -> Result<ModrinthSearchResult, String> {
    let url = format!(
        "{}/search?query={}&facets=[[\"project_type:{}\"]]",
        MODRINTH_API, query, project_type
    );
    let client = get_client();
    let res = client.get(&url).send().await.map_err(|e| e.to_string())?;
    res.json::<ModrinthSearchResult>()
        .await
        .map_err(|e| e.to_string())
}

pub async fn get_project_versions(project_id: &str) -> Result<Vec<ModrinthVersion>, String> {
    let url = format!("{}/project/{}/version", MODRINTH_API, project_id);
    let client = get_client();
    let res = client.get(&url).send().await.map_err(|e| e.to_string())?;
    res.json::<Vec<ModrinthVersion>>()
        .await
        .map_err(|e| e.to_string())
}

pub async fn get_version(version_id: &str) -> Result<ModrinthVersion, String> {
    let url = format!("{}/version/{}", MODRINTH_API, version_id);
    let client = get_client();
    let res = client.get(&url).send().await.map_err(|e| e.to_string())?;
    res.json::<ModrinthVersion>()
        .await
        .map_err(|e| e.to_string())
}

pub async fn search_popular_mods(limit: usize) -> Result<ModrinthSearchResult, String> {
    // Use Modrinth search with facets for project_type:mod and sort by downloads
    let url = format!(
        "{}/search?query=&facets=[[\"project_type:mod\"]]&index=0&limit={}&sort=downloads",
        MODRINTH_API, limit
    );
    let client = get_client();
    let res = client.get(&url).send().await.map_err(|e| e.to_string())?;
    res.json::<ModrinthSearchResult>()
        .await
        .map_err(|e| e.to_string())
}

pub async fn install_mrpack(
    app: &AppHandle,
    instance_id: &str,
    mrpack_path: &Path,
) -> Result<ModpackIndex, String> {
    let file = fs::File::open(mrpack_path).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;

    // 1. Read modrinth.index.json
    let mut index_content = String::new();
    {
        let mut index_file = archive
            .by_name("modrinth.index.json")
            .map_err(|e| e.to_string())?;
        index_file
            .read_to_string(&mut index_content)
            .map_err(|e| e.to_string())?;
    }
    let index: ModpackIndex = serde_json::from_str(&index_content).map_err(|e| e.to_string())?;

    let root = crate::commands::instance_dir(app, instance_id)?;
    let mc_dir = root.join(".minecraft");

    // 2. Download mods
    for file in &index.files {
        let is_client = file
            .env
            .as_ref()
            .map(|e| e.client != "unsupported")
            .unwrap_or(true);
        if !is_client {
            continue;
        }

        let target = mc_dir.join(&file.path);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }

        // Modrinth files can have multiple download URLs, try them
        let mut downloaded = false;
        for url in &file.downloads {
            if let Ok(_) = crate::download::download_to_file(url, &target).await {
                downloaded = true;
                break;
            }
        }

        if !downloaded {
            return Err(format!("Failed to download mod: {}", file.path));
        }
    }

    // 3. Extract overrides
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| e.to_string())?;
        let name = file.name().to_string();

        if name.starts_with("overrides/") {
            let relative_path = &name["overrides/".len()..];
            if relative_path.is_empty() {
                continue;
            }
            let target = mc_dir.join(relative_path);

            if file.is_dir() {
                fs::create_dir_all(target).map_err(|e| e.to_string())?;
            } else {
                if let Some(parent) = target.parent() {
                    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                }
                let mut outfile = fs::File::create(target).map_err(|e| e.to_string())?;
                std::io::copy(&mut file, &mut outfile).map_err(|e| e.to_string())?;
            }
        } else if name.starts_with("client-overrides/") {
            let relative_path = &name["client-overrides/".len()..];
            if relative_path.is_empty() {
                continue;
            }
            let target = mc_dir.join(relative_path);

            if file.is_dir() {
                fs::create_dir_all(target).map_err(|e| e.to_string())?;
            } else {
                if let Some(parent) = target.parent() {
                    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                }
                let mut outfile = fs::File::create(target).map_err(|e| e.to_string())?;
                std::io::copy(&mut file, &mut outfile).map_err(|e| e.to_string())?;
            }
        }
    }

    Ok(index)
}

/// Parse and return the `modrinth.index.json` from a .mrpack without extracting files.
pub fn parse_mrpack_index(mrpack_path: &Path) -> Result<ModpackIndex, String> {
    let file = fs::File::open(mrpack_path).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;

    let mut index_content = String::new();
    {
        let mut index_file = archive
            .by_name("modrinth.index.json")
            .map_err(|e| e.to_string())?;
        index_file
            .read_to_string(&mut index_content)
            .map_err(|e| e.to_string())?;
    }

    let index: ModpackIndex = serde_json::from_str(&index_content).map_err(|e| e.to_string())?;
    Ok(index)
}
