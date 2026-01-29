use std::fs;
use std::path::PathBuf;
use std::process::Command;

use tauri::{AppHandle, Manager};

use crate::download::download_text;
use crate::install::{install_assets, install_client_jar, install_libraries};
use crate::instance::{Instance, InstanceState};
use crate::launch::build_classpath;
use crate::minecraft::VersionManifest;
use crate::version::VersionJson;

/* ============================================================
 * Constants
 * ============================================================ */

const MANIFEST_URL: &str = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";

const MANIFEST_FILE: &str = "version_manifest_v2.json";

/* ============================================================
 * Path helpers
 * ============================================================ */

fn app_data(app: &AppHandle) -> Result<PathBuf, String> {
    app.path().app_data_dir().map_err(|e| e.to_string())
}

fn minecraft_root(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(app_data(app)?.join("minecraft"))
}

fn versions_root(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(minecraft_root(app)?.join("versions"))
}

fn instances_root(app: &AppHandle) -> Result<PathBuf, String> {
    Ok(minecraft_root(app)?.join("instances"))
}

fn instance_dir(app: &AppHandle, id: &str) -> Result<PathBuf, String> {
    Ok(instances_root(app)?.join(id))
}

fn instance_meta_path(app: &AppHandle, id: &str) -> Result<PathBuf, String> {
    Ok(instance_dir(app, id)?.join("instance.json"))
}

fn manifest_cache_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app_data(app)?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join(MANIFEST_FILE))
}

/* ============================================================
 * Version manifest
 * ============================================================ */

#[tauri::command]
pub async fn get_version_manifest(app: AppHandle) -> Result<VersionManifest, String> {
    let path = manifest_cache_path(&app)?;

    if path.exists() {
        let text = fs::read_to_string(&path).map_err(|e| e.to_string())?;
        return serde_json::from_str(&text).map_err(|e| e.to_string());
    }

    let res = reqwest::get(MANIFEST_URL)
        .await
        .map_err(|e| e.to_string())?;

    let text = res.text().await.map_err(|e| e.to_string())?;
    fs::write(&path, &text).map_err(|e| e.to_string())?;

    serde_json::from_str(&text).map_err(|e| e.to_string())
}

fn load_cached_manifest(app: &AppHandle) -> Result<VersionManifest, String> {
    let path = manifest_cache_path(app)?;
    let text = fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_json::from_str(&text).map_err(|e| e.to_string())
}

/* ============================================================
 * Instance management
 * ============================================================ */

#[tauri::command]
pub async fn create_instance(
    app: AppHandle,
    name: String,
    version: String,
) -> Result<String, String> {
    let id = uuid::Uuid::new_v4().to_string();
    let dir = instance_dir(&app, &id)?;

    fs::create_dir_all(dir.join(".minecraft")).map_err(|e| e.to_string())?;

    let instance = Instance {
        id,
        name,
        version,
        state: InstanceState::Installing,
        created_at: chrono::Utc::now().timestamp() as u64,
        last_played: None,
    };

    fs::write(
        instance_meta_path(&app, &instance.id)?,
        serde_json::to_string_pretty(&instance).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;

    Ok((instance.id).to_string())
}

#[tauri::command]
pub fn list_instances(app: AppHandle) -> Result<Vec<Instance>, String> {
    let root = instances_root(&app)?;
    if !root.exists() {
        return Ok(Vec::new());
    }

    let mut out = Vec::new();

    for entry in fs::read_dir(root).map_err(|e| e.to_string())? {
        let path = entry.map_err(|e| e.to_string())?.path();
        let meta = path.join("instance.json");

        if meta.exists() {
            let text = fs::read_to_string(meta).map_err(|e| e.to_string())?;
            let inst: Instance = serde_json::from_str(&text).map_err(|e| e.to_string())?;
            out.push(inst);
        }
    }

    Ok(out)
}

/* ============================================================
 * Version installation (shared payload)
 * ============================================================ */

#[tauri::command]
pub async fn download_version(
    app: AppHandle,
    instance_id: String,
    version_id: String,
) -> Result<(), String> {
    let manifest = load_cached_manifest(&app)?;

    let entry = manifest
        .versions
        .iter()
        .find(|v| v.id == version_id)
        .ok_or("Version not found in manifest")?;

    let version_dir = versions_root(&app)?.join(&version_id);
    fs::create_dir_all(&version_dir).map_err(|e| e.to_string())?;

    let json_path = version_dir.join(format!("{version_id}.json"));

    if !json_path.exists() {
        let text = download_text(entry.url.as_str()).await?;
        fs::write(&json_path, &text).map_err(|e| e.to_string())?;
    }
    let text = fs::read_to_string(&json_path).map_err(|e| e.to_string())?;

    let version: VersionJson = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    let result: Result<(), String> = async {
        install_client_jar(&app, &version_id, &version).await?;
        install_libraries(&app, &version).await?;
        install_assets(&app, &version).await?;
        Ok(())
    }
    .await;
    // Mark instance as ready
    let meta_path = instance_meta_path(&app, &instance_id)?;
    if !meta_path.exists() {
        return Err(format!(
            "instance.json not found for instance {}",
            instance_id
        ));
    }
    let mut instance: Instance =
        serde_json::from_str(&fs::read_to_string(&meta_path).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;

    instance.state = match result {
        Ok(_) => InstanceState::Ready,
        Err(_) => InstanceState::Error,
    };

    fs::write(
        &meta_path,
        serde_json::to_string_pretty(&instance).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;

    Ok(())
}

/* ============================================================
 * Launch (instance-aware)
 * ============================================================ */

#[tauri::command]
pub async fn launch_instance(app: AppHandle, instance_id: String) -> Result<(), String> {
    let instance_root = instance_dir(&app, &instance_id)?;
    let game_dir = instance_root.join(".minecraft");

    fs::create_dir_all(&game_dir).map_err(|e| e.to_string())?;

    let meta_text =
        fs::read_to_string(instance_root.join("instance.json")).map_err(|e| e.to_string())?;

    let instance: Instance = serde_json::from_str(&meta_text).map_err(|e| e.to_string())?;

    let version_id = &instance.version;

    let version_json_path = versions_root(&app)?
        .join(version_id)
        .join(format!("{version_id}.json"));

    let text = fs::read_to_string(&version_json_path).map_err(|e| e.to_string())?;

    let version: VersionJson = serde_json::from_str(&text).map_err(|e| e.to_string())?;

    let classpath = build_classpath(&app, version_id, &version)?;
    let mc_root = minecraft_root(&app)?;

    Command::new("java")
        .arg("-Xmx2G")
        .arg("-cp")
        .arg(classpath)
        .arg(&version.mainClass)
        .arg("--username")
        .arg("Player")
        .arg("--uuid")
        .arg("00000000-0000-0000-0000-000000000000")
        .arg("--accessToken")
        .arg("0")
        .arg("--userType")
        .arg("offline")
        .arg("--version")
        .arg(version_id)
        .arg("--gameDir")
        .arg(game_dir.to_string_lossy().to_string())
        .arg("--assetsDir")
        .arg(mc_root.join("assets").to_string_lossy().to_string())
        .arg("--assetIndex")
        .arg(&version.assetIndex.id)
        .spawn()
        .map_err(|e| e.to_string())?;

    Ok(())
}
