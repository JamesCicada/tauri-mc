use std::fs;
use std::path::PathBuf;
use tauri::AppHandle;
use tauri::Manager;

use crate::download::download_text;
use crate::download::download_to_file;
use crate::rules::rules_allow;
use crate::version::VersionJson;

use crate::assets::{AssetIndexJson, AssetObject};

pub async fn install_libraries(app: &AppHandle, version_json: &VersionJson) -> Result<(), String> {
    let base = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("minecraft")
        .join("libraries");

    for lib in &version_json.libraries {
        if !rules_allow(&lib.rules) {
            continue;
        }

        // 1. Download main artifact
        if let Some(artifact) = &lib.downloads.artifact {
            let target = base.join(&artifact.path);
            if !target.exists() {
                println!("Downloading library {}", lib.name);
                download_to_file(&artifact.url, &target).await?;
            }
        }

        // 2. Download natives if any
        let os_key = if cfg!(target_os = "windows") {
            "windows"
        } else if cfg!(target_os = "macos") {
            "osx"
        } else {
            "linux"
        };

        if let Some(classifier) = lib.natives.get(os_key) {
            if let Some(artifact) = lib.downloads.classifiers.get(classifier) {
                let target = base.join(&artifact.path);
                if !target.exists() {
                    println!("Downloading native library {} ({})", lib.name, classifier);
                    download_to_file(&artifact.url, &target).await?;
                }
            }
        }
    }

    Ok(())
}

pub async fn install_client_jar(
    app: &AppHandle,
    id: &str,
    version: &VersionJson,
) -> Result<(), String> {
    let jar_path = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("minecraft")
        .join("versions")
        .join(id)
        .join(format!("{}.jar", id));

    if jar_path.exists() {
        return Ok(());
    }

    println!("Downloading client jar {}", id);
    download_to_file(&version.downloads.client.url, &jar_path).await
}

const ASSET_BASE_URL: &str = "https://resources.download.minecraft.net";

pub async fn install_assets(app: &AppHandle, version: &VersionJson) -> Result<(), String> {
    let base = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("minecraft")
        .join("assets");

    let indexes = base.join("indexes");
    let objects = base.join("objects");

    fs::create_dir_all(&indexes).map_err(|e| e.to_string())?;
    fs::create_dir_all(&objects).map_err(|e| e.to_string())?;

    // 1️⃣ Download asset index
    let index_text = download_text(&version.assetIndex.url).await?;
    let index_path = indexes.join(format!("{}.json", version.assetIndex.id));
    fs::write(&index_path, &index_text).map_err(|e| e.to_string())?;

    let index: AssetIndexJson = serde_json::from_str(&index_text).map_err(|e| e.to_string())?;

    // 2️⃣ Download objects
    for (_name, obj) in index.objects {
        download_asset_object(&objects, &obj).await?;
    }

    Ok(())
}

async fn download_asset_object(objects_dir: &PathBuf, obj: &AssetObject) -> Result<(), String> {
    let hash = &obj.hash;
    let subdir = &hash[0..2];

    let target = objects_dir.join(subdir).join(hash);

    if target.exists() {
        return Ok(());
    }

    let url = format!("{}/{}/{}", ASSET_BASE_URL, subdir, hash);

    download_to_file(&url, &target).await
}
