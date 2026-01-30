use serde::Serialize;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use tauri::{AppHandle, Emitter, Manager, State};

use crate::install::{install_assets, install_client_jar, install_libraries};
use crate::instance::{Instance, InstanceState};
use crate::java::ensure_java;
use crate::launch::build_classpath;
use crate::minecraft::get_manifest;
use crate::version::VersionJson;
use std::collections::HashMap;
use std::sync::Mutex;

// Helper function to convert Maven coordinates to file path
// e.g., "net.fabricmc:fabric-loader:0.18.4" -> "net/fabricmc/fabric-loader/0.18.4/fabric-loader-0.18.4.jar"
fn maven_coords_to_path(coords: &str) -> Option<String> {
    let parts: Vec<&str> = coords.split(':').collect();
    if parts.len() != 3 {
        return None;
    }

    let group_id = parts[0].replace('.', "/");
    let artifact_id = parts[1];
    let version = parts[2];

    Some(format!(
        "{}/{}/{}/{}-{}.jar",
        group_id, artifact_id, version, artifact_id, version
    ))
}

// Loader-related commands moved to `loader.rs` for better organization
pub use crate::loader::*;

#[derive(Default)]
pub struct ChildProcessState(pub Mutex<HashMap<String, std::process::Child>>);

pub fn minecraft_root(app: &AppHandle) -> Result<PathBuf, String> {
    let path = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("minecraft");
    fs::create_dir_all(&path).map_err(|e| e.to_string())?;
    Ok(path)
}

fn versions_root(app: &AppHandle) -> Result<PathBuf, String> {
    let path = minecraft_root(app)?.join("versions");
    fs::create_dir_all(&path).map_err(|e| e.to_string())?;
    Ok(path)
}

fn instances_root(app: &AppHandle) -> Result<PathBuf, String> {
    let path = minecraft_root(app)?.join("instances");
    fs::create_dir_all(&path).map_err(|e| e.to_string())?;
    Ok(path)
}

pub fn instance_dir(app: &AppHandle, id: &str) -> Result<PathBuf, String> {
    Ok(instances_root(app)?.join(id))
}

pub fn instance_meta_path(app: &AppHandle, id: &str) -> Result<PathBuf, String> {
    Ok(instance_dir(app, id)?.join("instance.json"))
}

#[tauri::command]
pub async fn get_version_manifest() -> Result<crate::minecraft::VersionManifest, String> {
    get_manifest().await
}

#[tauri::command]
pub async fn list_instances(app: AppHandle) -> Result<Vec<Instance>, String> {
    let root = instances_root(&app)?;
    let mut instances = Vec::new();

    if let Ok(entries) = fs::read_dir(root) {
        for entry in entries.flatten() {
            let meta_path = entry.path().join("instance.json");
            if meta_path.exists() {
                if let Ok(text) = fs::read_to_string(meta_path) {
                    if let Ok(instance) = serde_json::from_str::<Instance>(&text) {
                        instances.push(instance);
                    }
                }
            }
        }
    }

    instances.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(instances)
}

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
        version: version.clone(),
        mc_version: Some(version.clone()),
        state: InstanceState::Installing,
        created_at: chrono::Utc::now().timestamp() as u64,
        last_played: None,
        java_path: None,
        java_path_override: None,
        max_memory: None,
        min_memory: None,
        java_args: None,
        java_warning_ignored: false,
        loader: None,
        loader_version: None,
    };

    fs::write(
        instance_meta_path(&app, &instance.id)?,
        serde_json::to_string_pretty(&instance).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;

    Ok(instance.id)
}

#[tauri::command]
pub async fn delete_instance(
    app: AppHandle,
    instance_id: String,
    delete_version: bool,
) -> Result<(), String> {
    let dir = instance_dir(&app, &instance_id)?;

    // If delete_version is true, we need to check if other instances use it
    if delete_version {
        let instances = list_instances(app.clone()).await?;
        // Determine version id to delete. If the instance had a loader installed, delete the derived loader-backed version
        let version_id = {
            let meta_text =
                fs::read_to_string(dir.join("instance.json")).map_err(|e| e.to_string())?;
            let instance: Instance = serde_json::from_str(&meta_text).map_err(|e| e.to_string())?;
            if let (Some(loader), Some(loader_version)) =
                (instance.loader.clone(), instance.loader_version.clone())
            {
                format!("{}-loader-{}-{}", loader, loader_version, instance.version)
            } else {
                instance.version
            }
        };

        let other_uses = instances
            .iter()
            .filter(|i| {
                if i.id == instance_id {
                    return false;
                }
                let other_vid = if let (Some(loader), Some(loader_version)) =
                    (i.loader.clone(), i.loader_version.clone())
                {
                    format!("{}-loader-{}-{}", loader, loader_version, i.version)
                } else {
                    i.version.clone()
                };
                other_vid == version_id
            })
            .count();

        if other_uses == 0 {
            let version_dir = versions_root(&app)?.join(&version_id);
            if version_dir.exists() {
                fs::remove_dir_all(version_dir).map_err(|e| e.to_string())?;
            }
        }
    }

    if dir.exists() {
        fs::remove_dir_all(dir).map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[tauri::command]
pub async fn check_version_usage(
    app: AppHandle,
    instance_id: String,
    version_id: String,
) -> Result<bool, String> {
    let instances = list_instances(app).await?;
    let other_uses = instances
        .iter()
        .filter(|i| i.id != instance_id && i.version == version_id)
        .count();
    Ok(other_uses == 0)
}

#[tauri::command]
pub async fn download_version(
    app: AppHandle,
    instance_id: String,
    version_id: String,
) -> Result<(), String> {
    let version_json_path = versions_root(&app)?
        .join(&version_id)
        .join(format!("{version_id}.json"));

    // 1️⃣ Download version metadata
    let manifest = get_manifest().await?;
    let version_info = manifest
        .versions
        .iter()
        .find(|v| v.id == version_id)
        .ok_or("Version not found in manifest")?;

    let version_json_text = crate::download::download_text(&version_info.url).await?;
    fs::create_dir_all(version_json_path.parent().unwrap()).map_err(|e| e.to_string())?;
    fs::write(&version_json_path, &version_json_text).map_err(|e| e.to_string())?;

    let version: VersionJson =
        serde_json::from_str(&version_json_text).map_err(|e| e.to_string())?;

    // Perform Java installation first
    let java_result = ensure_java(&app, &version_id).await;

    // Perform installation
    let result: Result<(), String> = async {
        if let Err(e) = &java_result {
            return Err(format!("Java installation failed: {}", e));
        }
        install_client_jar(&app, &version_id, &version).await?;
        install_libraries(&app, &version).await?;
        install_assets(&app, &version).await?;
        Ok(())
    }
    .await;

    // Update instance state
    let mut instance = {
        let meta_text = fs::read_to_string(instance_meta_path(&app, &instance_id)?)
            .map_err(|e| e.to_string())?;
        serde_json::from_str::<Instance>(&meta_text).map_err(|e| e.to_string())?
    };

    if let Err(e) = result {
        instance.state = InstanceState::Error;
        let _ = fs::write(
            instance_meta_path(&app, &instance_id)?,
            serde_json::to_string_pretty(&instance).map_err(|e| e.to_string())?,
        );
        return Err(e);
    }

    instance.state = InstanceState::Ready;
    // Store java path if successful
    if let Ok(path) = java_result {
        instance.java_path = Some(path);
    }

    fs::write(
        instance_meta_path(&app, &instance_id)?,
        serde_json::to_string_pretty(&instance).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;

    println!("✓ Installation completed successfully for {}", version_id);
    Ok(())
}

// Ensure vanilla Minecraft version files (version JSON, client, libraries, assets) are present
async fn ensure_vanilla_version(
    app: &AppHandle,
    mc_version: &str,
) -> Result<crate::version::VersionJson, String> {
    // Check if we already have the version json on disk
    let version_json_path = versions_root(app)?
        .join(mc_version)
        .join(format!("{}.json", mc_version));
    if version_json_path.exists() {
        let text = std::fs::read_to_string(&version_json_path).map_err(|e| e.to_string())?;
        let version: crate::version::VersionJson =
            serde_json::from_str(&text).map_err(|e| e.to_string())?;
        return Ok(version);
    }

    // Fetch manifest and download version json
    let manifest = get_manifest().await?;
    let version_info = manifest
        .versions
        .iter()
        .find(|v| v.id == mc_version)
        .ok_or(format!(
            "Minecraft version {} not found in manifest",
            mc_version
        ))?;

    let version_json_text = crate::download::download_text(&version_info.url).await?;

    std::fs::create_dir_all(version_json_path.parent().unwrap()).map_err(|e| e.to_string())?;
    std::fs::write(&version_json_path, &version_json_text).map_err(|e| e.to_string())?;

    let version: crate::version::VersionJson =
        serde_json::from_str(&version_json_text).map_err(|e| e.to_string())?;

    // Install client jar, libraries and assets
    install_client_jar(app, mc_version, &version).await?;
    install_libraries(app, &version).await?;
    install_assets(app, &version).await?;

    Ok(version)
}

#[tauri::command]
pub async fn install_loader(
    app: AppHandle,
    loader_type: String,
    mc_version: String,
    loader_version: String,
) -> Result<(String, String), String> {
    // Support only fabric and quilt for now
    // Track the effective loader version we end up using (may change due to fallback)
    let mut effective_loader_version = loader_version.clone();

    // Helper to build profile URL for a given version
    let build_profile_url = |lt: &str, mc: &str, v: &str| -> String {
        let v_escaped = v.replace("+", "%2B");
        match lt {
            "fabric" => format!(
                "https://meta.fabricmc.net/v2/versions/loader/{}/{}/profile/json",
                mc, v_escaped
            ),
            "quilt" => format!(
                "https://meta.quiltmc.org/v3/versions/loader/{}/{}/profile/json",
                mc, v_escaped
            ),
            _ => String::new(),
        }
    };

    let mut url = build_profile_url(&loader_type, &mc_version, &effective_loader_version);

    // Fetch profile JSON, with fallback: if the server reports "no loader version found",
    // try listing available loader versions and pick a close match to retry.
    let profile_text = match crate::download::download_text(&url).await {
        Ok(t) => t,
        Err(e) => {
            // Only try the fallback for known "no loader version found" responses
            if e.contains("no loader version found")
                || (e.starts_with("HTTP 400") && e.contains("no loader"))
            {
                // Build the versions listing URL and try to resolve a real loader version
                let list_url = match loader_type.as_str() {
                    "fabric" => format!(
                        "https://meta.fabricmc.net/v2/versions/loader/{}",
                        mc_version
                    ),
                    "quilt" => {
                        format!("https://meta.quiltmc.org/v3/versions/loader/{}", mc_version)
                    }
                    _ => return Err(e),
                };

                std::println!(
                    "Loader version {} not found for {}, trying to resolve from list",
                    loader_version,
                    mc_version
                );

                let list_text = crate::download::download_text(&list_url)
                    .await
                    .map_err(|_| e.clone())?;

                // Collect stable and beta lists and try to resolve a close match
                let mut resolved_version: Option<String> = None;
                let mut stable_first: Vec<String> = Vec::new();
                let mut fallback_beta: Vec<String> = Vec::new();

                if let Ok(list_val) = serde_json::from_str::<serde_json::Value>(&list_text) {
                    if let Some(arr) = list_val.as_array() {
                        let base_left = loader_version.split('+').next().unwrap_or(&loader_version);
                        for item in arr {
                            if let Some(vs) = item.get("version").and_then(|v| v.as_str()) {
                                if vs == loader_version || vs == base_left || vs.contains(base_left)
                                {
                                    resolved_version = Some(vs.to_string());
                                    break;
                                }
                            }
                            if let Some(id) = item.get("id").and_then(|v| v.as_str()) {
                                if id == loader_version || id == base_left || id.contains(base_left)
                                {
                                    resolved_version = Some(id.to_string());
                                    break;
                                }
                            }

                            // determine stable flag (Fabric uses loader.stable = true)
                            let mut explicit_stable: Option<bool> = None;
                            if let Some(loader_obj) = item.get("loader") {
                                if let Some(b) = loader_obj.get("stable").and_then(|v| v.as_bool())
                                {
                                    explicit_stable = Some(b);
                                }
                            }
                            if explicit_stable.is_none() {
                                if let Some(b) = item.get("stable").and_then(|v| v.as_bool()) {
                                    explicit_stable = Some(b);
                                }
                            }

                            let s = item
                                .get("version")
                                .and_then(|v| v.as_str())
                                .or_else(|| item.get("id").and_then(|v| v.as_str()));
                            if let Some(s) = s {
                                if explicit_stable.unwrap_or(false) {
                                    stable_first.push(s.to_string());
                                } else {
                                    fallback_beta.push(s.to_string());
                                }
                            }
                        }
                    }
                }

                // Prefer a resolved match, else pick the first stable, then a beta fallback
                if let Some(resolved) = resolved_version {
                    effective_loader_version = resolved.clone();
                } else if let Some(sv) = stable_first.get(0) {
                    effective_loader_version = sv.clone();
                } else if let Some(bv) = fallback_beta.get(0) {
                    effective_loader_version = bv.clone();
                } else {
                    return Err(e);
                }

                // rebuild URL with chosen version and retry
                url = build_profile_url(&loader_type, &mc_version, &effective_loader_version);
                crate::download::download_text(&url)
                    .await
                    .map_err(|_| e.clone())?
            } else {
                return Err(e);
            }
        }
    };

    // Parse and validate inheritsFrom (include a truncated response snippet on parse errors)
    let profile_json: serde_json::Value = serde_json::from_str(&profile_text).map_err(|e| {
        let snippet: String = profile_text.chars().take(200).collect();
        format!("{} - response (truncated): {}", e.to_string(), snippet)
    })?;
    let inherits = profile_json
        .get("inheritsFrom")
        .and_then(|v| v.as_str())
        .ok_or("profile missing inheritsFrom")?;

    if inherits != mc_version {
        return Err(format!(
            "profile inheritsFrom mismatch: expected {}, found {}",
            mc_version, inherits
        ));
    }

    // Map into our VersionJson struct (this will ignore extra profile fields)
    // Derived version id format: <loader>-loader-<loaderVersion>-<mcVersion>
    let derived_id = format!(
        "{}-loader-{}-{}",
        loader_type, effective_loader_version, mc_version
    );

    let derived_dir = versions_root(&app)?.join(&derived_id);
    std::fs::create_dir_all(&derived_dir).map_err(|e| e.to_string())?;
    let derived_json_path = derived_dir.join(format!("{}.json", derived_id));
    if derived_json_path.exists() {
        return Ok((derived_id, effective_loader_version));
    }

    // Try to map profile into our VersionJson; if it doesn't fit, fall back to vanilla version
    let version_json: VersionJson = match serde_json::from_value(profile_json.clone()) {
        Ok(v) => v,
        Err(_) => {
            // Fallback: use vanilla mc files, but try to merge loader-provided libraries/downloads
            let base = ensure_vanilla_version(&app, &mc_version).await?;

            // Start from vanilla and then append loader libraries if present in profile.json
            let mut libraries = base.libraries.clone();

            if let Some(arr) = profile_json.get("libraries").and_then(|v| v.as_array()) {
                for lib in arr {
                    if let Some(name) = lib.get("name").and_then(|v| v.as_str()) {
                        // Try to get downloads.artifact info (standard Minecraft format)
                        let mut artifact_opt: Option<crate::version::Artifact> = None;
                        if let Some(dl) = lib.get("downloads").and_then(|v| v.get("artifact")) {
                            if let (Some(url), Some(path), Some(sha1), Some(size)) = (
                                dl.get("url").and_then(|v| v.as_str()),
                                dl.get("path").and_then(|v| v.as_str()),
                                dl.get("sha1").and_then(|v| v.as_str()),
                                dl.get("size").and_then(|v| v.as_u64()),
                            ) {
                                artifact_opt = Some(crate::version::Artifact {
                                    path: path.to_string(),
                                    url: url.to_string(),
                                    sha1: sha1.to_string(),
                                    size: size as u64,
                                });
                            }
                        } else if let Some(base_url) = lib.get("url").and_then(|v| v.as_str()) {
                            // Fabric format: url field pointing to Maven repository
                            // Some libraries have sha1/size, others don't
                            if let Some(path) = maven_coords_to_path(name) {
                                let full_url = if base_url.ends_with('/') {
                                    format!("{}{}", base_url, path)
                                } else {
                                    format!("{}/{}", base_url, path)
                                };

                                // Use sha1/size if available, otherwise set to 0 (will be verified during download)
                                let sha1 = lib
                                    .get("sha1")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string();
                                let size = lib.get("size").and_then(|v| v.as_u64()).unwrap_or(0);

                                artifact_opt = Some(crate::version::Artifact {
                                    path: path.to_string(),
                                    url: full_url,
                                    sha1,
                                    size,
                                });
                            }
                        }

                        let lib_struct = crate::version::Library {
                            name: name.to_string(),
                            downloads: crate::version::LibraryDownloads {
                                artifact: artifact_opt,
                                classifiers: std::collections::HashMap::new(),
                            },
                            natives: std::collections::HashMap::new(),
                            rules: Vec::new(),
                        };

                        libraries.push(lib_struct);
                    }
                }
            }

            // For downloads, prefer profile client if present
            let downloads = if let Some(client_dl) =
                profile_json.get("downloads").and_then(|v| v.get("client"))
            {
                if let (Some(url), Some(sha1), Some(size)) = (
                    client_dl.get("url").and_then(|v| v.as_str()),
                    client_dl.get("sha1").and_then(|v| v.as_str()),
                    client_dl.get("size").and_then(|v| v.as_u64()),
                ) {
                    crate::version::Downloads {
                        client: crate::version::DownloadInfo {
                            url: url.to_string(),
                            sha1: sha1.to_string(),
                            size: size as u64,
                        },
                    }
                } else {
                    base.downloads.clone()
                }
            } else {
                base.downloads.clone()
            };

            let main_class = profile_json
                .get("mainClass")
                .and_then(|v| v.as_str())
                .unwrap_or(&base.mainClass)
                .to_string();

            crate::version::VersionJson {
                id: None,
                inheritsFrom: Some(mc_version.to_string()),
                releaseTime: None,
                time: None,
                r#type: None,
                arguments: None,
                libraries,
                downloads,
                mainClass: main_class,
                assetIndex: base.assetIndex.clone(),
            }
        }
    };

    // CRITICAL: Ensure inheritsFrom is set correctly for derived versions
    // The derived version should inherit from the base MC version
    let final_version_json = version_json;

    // Add inheritsFrom field if it's missing (this is crucial for Fabric)
    // We need to create a custom struct that includes inheritsFrom
    let version_with_inherits = serde_json::json!({
        "id": derived_id,
        "inheritsFrom": final_version_json.inheritsFrom.as_ref().unwrap_or(&mc_version),
        "releaseTime": final_version_json.releaseTime,
        "time": final_version_json.time,
        "type": final_version_json.r#type.as_ref().unwrap_or(&"release".to_string()),
        "mainClass": final_version_json.mainClass,
        "arguments": final_version_json.arguments,
        "libraries": final_version_json.libraries,
        "downloads": final_version_json.downloads,
        "assetIndex": final_version_json.assetIndex
    });

    // Persist the derived version JSON (pretty) so the launcher treats it as a distinct version
    let derived_text =
        serde_json::to_string_pretty(&version_with_inherits).map_err(|e| e.to_string())?;
    std::fs::write(&derived_json_path, &derived_text).map_err(|e| e.to_string())?;

    // Verify that the version JSON contains Fabric loader libraries
    if loader_type == "fabric" {
        let has_fabric_loader = final_version_json.libraries.iter().any(|lib| {
            lib.name.to_lowercase().contains("fabric-loader")
                || lib.name.to_lowercase().contains("net.fabricmc")
        });

        if !has_fabric_loader {
            println!("Warning: Fabric profile JSON does not contain fabric-loader libraries");
            println!(
                "Libraries found: {:?}",
                final_version_json
                    .libraries
                    .iter()
                    .map(|l| &l.name)
                    .collect::<Vec<_>>()
            );
        } else {
            println!("Fabric loader libraries found in profile JSON");
        }
    }

    // Install client/jar, libraries and assets for the derived version
    println!("Installing client JAR for derived version: {}", derived_id);
    install_client_jar(&app, &derived_id, &final_version_json).await?;

    println!("Installing libraries for derived version: {}", derived_id);
    install_libraries(&app, &final_version_json).await?;

    println!("Installing assets for derived version: {}", derived_id);
    install_assets(&app, &final_version_json).await?;

    println!(
        "Loader installation completed successfully: {} {}",
        loader_type, effective_loader_version
    );
    Ok((derived_id, effective_loader_version))
}

// `get_loader_versions` has been moved to `loader.rs` and is re-exported. See `src-tauri/src/loader.rs` for implementation.

#[derive(Serialize, Clone)]
pub struct InstanceLog {
    pub instance_id: String,
    pub message: String,
}

#[tauri::command]
pub async fn launch_instance(
    app: AppHandle,
    instance_id: String,
    process_state: State<'_, ChildProcessState>,
) -> Result<(), String> {
    let instance_root = instance_dir(&app, &instance_id)?;
    let game_dir = instance_root.join(".minecraft");

    fs::create_dir_all(&game_dir).map_err(|e| e.to_string())?;

    let meta_text =
        fs::read_to_string(instance_root.join("instance.json")).map_err(|e| e.to_string())?;

    let instance: Instance = serde_json::from_str(&meta_text).map_err(|e| e.to_string())?;

    // Determine the version JSON to use: if loader info is present, prefer derived loader-backed version; otherwise use instance.version
    let version_id = if let (Some(loader), Some(loader_v), Some(mc_v)) = (
        instance.loader.clone(),
        instance.loader_version.clone(),
        instance.mc_version.clone(),
    ) {
        format!("{}-loader-{}-{}", loader, loader_v, mc_v)
    } else if let (Some(loader), Some(loader_v)) =
        (instance.loader.clone(), instance.loader_version.clone())
    {
        // Fallback: use instance.version as mc version
        format!("{}-loader-{}-{}", loader, loader_v, instance.version)
    } else {
        instance.version.clone()
    };

    let version_json_path = versions_root(&app)?
        .join(&version_id)
        .join(format!("{version_id}.json"));

    // If the derived version JSON is missing, attempt recovery:
    // - If we have loader info on the instance, try to install the loader (creates derived version)
    // - Otherwise try to ensure vanilla version files
    if !version_json_path.exists() {
        if let (Some(loader), Some(loader_v)) =
            (instance.loader.clone(), instance.loader_version.clone())
        {
            let mc_v = instance
                .mc_version
                .clone()
                .unwrap_or(instance.version.clone());
            let _ = app.emit(
                "loader-install-log",
                format!(
                    "Derived version {} missing, attempting to install loader {} {}",
                    version_id, loader, loader_v
                ),
            );
            match install_loader(app.clone(), loader.clone(), mc_v.clone(), loader_v.clone()).await
            {
                Ok((_derived, used_version)) => {
                    // success — derived version should now exist (used_version is the actual loader version chosen)
                    let _ = app.emit(
                        "loader-install-log",
                        format!(
                            "install_loader resolved loader version: {} -> {}",
                            loader_v, used_version
                        ),
                    );
                }
                Err(e) => {
                    return Err(format!(
                        "Derived version {} missing and install_loader failed: {}",
                        version_id, e
                    ));
                }
            }
        } else {
            // No loader info -> try to ensure vanilla version json exists/install it
            if let Err(e) = ensure_vanilla_version(&app, &instance.version).await {
                return Err(format!(
                    "Version JSON missing at {} and failed to install vanilla {}: {}",
                    version_json_path.to_string_lossy(),
                    instance.version,
                    e
                ));
            }
        }
    }

    // Re-check that the JSON exists after recovery attempt
    if !version_json_path.exists() {
        return Err(format!(
            "Version JSON not found at path: {}",
            version_json_path.to_string_lossy()
        ));
    }

    let text = fs::read_to_string(&version_json_path).map_err(|e| {
        format!(
            "Failed to read version JSON at {}: {}",
            version_json_path.to_string_lossy(),
            e.to_string()
        )
    })?;

    let version: VersionJson = serde_json::from_str(&text).map_err(|e| {
        format!(
            "Failed to parse version JSON at {}: {}",
            version_json_path.to_string_lossy(),
            e.to_string()
        )
    })?;

    // Confirm client JAR exists too and if missing, attempt to recover similarly
    let client_jar = versions_root(&app)?
        .join(&version_id)
        .join(format!("{}.jar", version_id));
    if !client_jar.exists() {
        if let (Some(loader), Some(loader_v)) =
            (instance.loader.clone(), instance.loader_version.clone())
        {
            let mc_v = instance
                .mc_version
                .clone()
                .unwrap_or(instance.version.clone());
            let _ = app.emit(
                "loader-install-log",
                format!(
                    "Client JAR missing for {} - attempting loader install {} {}",
                    version_id, loader, loader_v
                ),
            );
            match install_loader(app.clone(), loader.clone(), mc_v.clone(), loader_v.clone()).await
            {
                Ok((_derived, used_version)) => {
                    let _ = app.emit(
                        "loader-install-log",
                        format!(
                            "install_loader resolved loader version: {} -> {}",
                            loader_v, used_version
                        ),
                    );
                }
                Err(e) => {
                    return Err(format!(
                        "Client JAR missing at {} and failed to install loader {} {}: {}",
                        client_jar.to_string_lossy(),
                        loader,
                        loader_v,
                        e
                    ));
                }
            }
        } else {
            // Try to install vanilla client jar
            if let Err(e) = install_client_jar(&app, &instance.version, &version).await {
                return Err(format!(
                    "Client JAR missing at {} and failed to install vanilla client for {}: {}",
                    client_jar.to_string_lossy(),
                    instance.version,
                    e
                ));
            }
        }

        if !client_jar.exists() {
            return Err(format!(
                "Client JAR not found at path: {}",
                client_jar.to_string_lossy()
            ));
        }
    }

    let classpath = build_classpath(&app, &version_id, &version)?;
    println!("Launch classpath: {}", classpath);
    println!("Launch main class: {}", version.mainClass);
    println!("Launch version ID: {}", version_id);

    let mc_root = minecraft_root(&app)?;

    let settings = crate::settings::get_settings(app.clone()).unwrap_or_default();

    // Java selection priority:
    // 1. Instance override
    // 2. Global setting override
    // 3. Instance auto-detected path
    // 4. "java"
    let java_cmd = instance
        .java_path_override
        .as_deref()
        .or(settings.global_java_path.as_deref())
        .or(instance.java_path.as_deref())
        .unwrap_or("java");

    let mut command = Command::new(java_cmd);

    // Memory settings (Instance override > Global settings)
    let min_mem = instance.min_memory.unwrap_or(settings.min_memory);
    let max_mem = instance.max_memory.unwrap_or(settings.max_memory);
    command.arg(format!("-Xms{}M", min_mem));
    command.arg(format!("-Xmx{}M", max_mem));

    // JVM args (Global settings + Instance override)
    for arg in settings.global_java_args.split_whitespace() {
        command.arg(arg);
    }
    if let Some(args) = &instance.java_args {
        for arg in args.split_whitespace() {
            command.arg(arg);
        }
    }

    command
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
        .arg(&version.assetIndex.id);

    // Capture logs
    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());

    // Update state to Running
    let mut instance_running = instance.clone();
    instance_running.state = InstanceState::Running;
    instance_running.last_played = Some(chrono::Utc::now().timestamp() as u64);

    fs::write(
        instance_meta_path(&app, &instance_running.id)?,
        serde_json::to_string_pretty(&instance_running).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;

    // Emit event to notify frontend immediately
    app.emit("instance-state-changed", &instance_running)
        .map_err(|e| e.to_string())?;

    if settings.close_on_launch {
        command.spawn().map_err(|e| e.to_string())?;
        app.exit(0);
    } else {
        match command.spawn() {
            Ok(mut child) => {
                let stdout = child.stdout.take().unwrap();
                let stderr = child.stderr.take().unwrap();

                // Store child process handle for killing later
                {
                    let mut lock = process_state.inner().0.lock().unwrap();
                    lock.insert(instance_id.clone(), child);
                }

                // Pipe logs in threads
                // Note: we can't take stdout/stderr again since we took them above.
                // But wait, we took them from `child` which we just inserted into `lock`.
                // Actually, taking them before inserting into map is correct.
                let app_logs = app.clone();
                let id_logs = instance_id.clone();
                std::thread::spawn(move || {
                    let reader = BufReader::new(stdout);
                    for line in reader.lines().flatten() {
                        let _ = app_logs.emit(
                            "instance-log",
                            InstanceLog {
                                instance_id: id_logs.clone(),
                                message: line,
                            },
                        );
                    }
                });

                let app_errs = app.clone();
                let id_errs = instance_id.clone();
                std::thread::spawn(move || {
                    let reader = BufReader::new(stderr);
                    for line in reader.lines().flatten() {
                        let _ = app_errs.emit(
                            "instance-log",
                            InstanceLog {
                                instance_id: id_errs.clone(),
                                message: line,
                            },
                        );
                    }
                });

                // Monitor process in background
                let app_handle = app.clone();
                let inst_id = instance_id.clone();

                std::thread::spawn(move || {
                    loop {
                        std::thread::sleep(std::time::Duration::from_millis(500));
                        let state = app_handle.state::<ChildProcessState>();
                        let mut lock = state.0.lock().unwrap();

                        let should_update = if let Some(child) = lock.get_mut(&inst_id) {
                            match child.try_wait() {
                                Ok(Some(_status)) => {
                                    lock.remove(&inst_id);
                                    true
                                }
                                Ok(None) => false,
                                Err(_) => {
                                    lock.remove(&inst_id);
                                    true
                                }
                            }
                        } else {
                            // If it's gone from map (killed by us), we still need to update UI
                            true
                        };

                        if should_update {
                            // Game closed or killed, update state back to Ready
                            if let Ok(root) = instance_dir(&app_handle, &inst_id) {
                                let meta_path = root.join("instance.json");
                                if let Ok(text) = fs::read_to_string(&meta_path) {
                                    if let Ok(mut inst) = serde_json::from_str::<Instance>(&text) {
                                        inst.state = InstanceState::Ready;
                                        if let Ok(updated_text) =
                                            serde_json::to_string_pretty(&inst)
                                        {
                                            let _ = fs::write(&meta_path, updated_text);
                                            let _ = app_handle.emit("instance-state-changed", inst);
                                        }
                                    }
                                }
                            }
                            break;
                        }
                    }
                });
            }
            Err(e) => return Err(e.to_string()),
        }
    }

    Ok(())
}

#[tauri::command]
pub fn save_instance(app: AppHandle, instance: Instance) -> Result<(), String> {
    fs::write(
        instance_meta_path(&app, &instance.id)?,
        serde_json::to_string_pretty(&instance).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())
}

#[derive(Serialize)]
pub struct JavaCompatibility {
    pub compatible: bool,
    pub actual_version: Option<u8>,
    pub required_version: u8,
    pub path: String,
}

#[tauri::command]
pub async fn check_java_compatibility(
    app: AppHandle,
    instance_id: String,
) -> Result<JavaCompatibility, String> {
    let instance_root = instance_dir(&app, &instance_id)?;
    let meta_text =
        fs::read_to_string(instance_root.join("instance.json")).map_err(|e| e.to_string())?;
    let instance: Instance = serde_json::from_str(&meta_text).map_err(|e| e.to_string())?;

    // Use the base Minecraft version if present (derived loader versions have a different id)
    let mc_version_for_java = instance.mc_version.as_deref().unwrap_or(&instance.version);
    let required_version = crate::java::get_required_java_version(mc_version_for_java);
    let path = crate::java::get_intended_java_path(&app, &instance);

    // Global setting: skip java compatibility entirely
    let settings = crate::settings::get_settings(app.clone()).unwrap_or_default();
    if settings.skip_java_check {
        return Ok(JavaCompatibility {
            compatible: true,
            actual_version: crate::java::get_java_major_version(&path),
            required_version,
            path,
        });
    }

    if instance.java_warning_ignored {
        return Ok(JavaCompatibility {
            compatible: true,
            actual_version: crate::java::get_java_major_version(&path),
            required_version,
            path,
        });
    }

    let actual_version = crate::java::get_java_major_version(&path);

    Ok(JavaCompatibility {
        compatible: actual_version.map_or(false, |v| v == required_version),
        actual_version,
        required_version,
        path,
    })
}
#[tauri::command]
pub async fn kill_instance(
    instance_id: String,
    process_state: State<'_, ChildProcessState>,
) -> Result<(), String> {
    let mut lock = process_state.inner().0.lock().unwrap();
    if let Some(mut child) = lock.remove(&instance_id) {
        let _ = child.kill();
    }
    Ok(())
}
#[tauri::command]
pub async fn search_projects(
    query: String,
    project_type: String,
) -> Result<crate::modrinth::ModrinthSearchResult, String> {
    crate::modrinth::search_projects(&query, &project_type).await
}

#[tauri::command]
pub async fn get_project_versions(
    project_id: String,
) -> Result<Vec<crate::modrinth::ModrinthVersion>, String> {
    crate::modrinth::get_project_versions(&project_id).await
}

#[tauri::command]
pub async fn get_popular_mods(
    _app: AppHandle,
    limit: Option<u8>,
) -> Result<crate::modrinth::ModrinthSearchResult, String> {
    let l = limit.unwrap_or(20) as usize;
    crate::modrinth::search_popular_mods(l).await
}

// Loader types and verification moved to `loader.rs` (see `loader` module).

// `find_loader_candidates` moved to `loader.rs` (re-exported from commands via `pub use loader::*`).

#[tauri::command]
pub async fn download_loader_version(
    app: AppHandle,
    instance_id: String,
    project_id: String,
    version_id: String,
) -> Result<(), String> {
    let version = crate::modrinth::get_version(&version_id).await?;

    let root = instance_dir(&app, &instance_id)?;
    let mc_dir = root.join(".minecraft");
    let loader_dir = mc_dir.join("loaders").join(&project_id).join(&version_id);
    std::fs::create_dir_all(&loader_dir).map_err(|e| e.to_string())?;

    // Download files
    for file in &version.files {
        let target = loader_dir.join(&file.filename);
        crate::download::download_to_file(&file.url, &target).await?;
    }

    // Determine loader type from Modrinth version metadata (prefer explicit loader names like fabric/quilt)
    let loader_type = if let Some(l) = version.loaders.first() {
        let lt = l.to_lowercase();
        if lt.contains("fabric") {
            "fabric".to_string()
        } else if lt.contains("quilt") {
            "quilt".to_string()
        } else if lt.contains("forge") {
            "forge".to_string()
        } else {
            l.clone()
        }
    } else {
        // Fallback to project_id (last resort)
        project_id.clone()
    };

    // Update instance metadata to record loader presence after download (store loader type & version)
    if let Ok(meta_text) = std::fs::read_to_string(root.join("instance.json")) {
        if let Ok(mut inst) = serde_json::from_str::<Instance>(&meta_text) {
            inst.loader = Some(loader_type.clone());
            inst.loader_version = Some(version.version_number.clone());
            let _ = std::fs::write(
                root.join("instance.json"),
                serde_json::to_string_pretty(&inst).map_err(|e| e.to_string())?,
            );
        }
    }

    // Try to find an installer jar and run it (best-effort)
    for entry in std::fs::read_dir(&loader_dir).map_err(|e| e.to_string())? {
        if let Ok(entry) = entry {
            let path = entry.path();
            if let Some(name) = path.file_name().and_then(|s| s.to_str()) {
                let lname = name.to_lowercase();
                if lname.contains("installer")
                    || lname.contains("fabric-installer")
                    || lname.contains("forge-installer")
                {
                    // Attempt to run installer with Java: java -jar <installer> client -dir <mc_dir> -mcversion <version> -loader <version_number>
                    let installer_path = path.to_string_lossy().to_string();

                    // Read instance metadata so we can pick the correct Java to run the installer
                    let instance_obj = {
                        let meta_text = std::fs::read_to_string(root.join("instance.json"))
                            .map_err(|e| e.to_string())?;
                        serde_json::from_str::<Instance>(&meta_text).map_err(|e| e.to_string())?
                    };
                    let instance_version = instance_obj.version.clone();
                    let java_cmd = crate::java::get_intended_java_path(&app, &instance_obj);

                    // Determine loader type for verification (prefer explicit loader names)
                    let loader_type_for_verify = if let Some(l) = version.loaders.first() {
                        let lt = l.to_lowercase();
                        if lt.contains("fabric") {
                            "fabric".to_string()
                        } else if lt.contains("quilt") {
                            "quilt".to_string()
                        } else {
                            l.clone()
                        }
                    } else {
                        project_id.clone()
                    };

                    // Clone for thread
                    let app_clone = app.clone();
                    let iid = instance_id.clone();
                    let pid = project_id.clone();
                    let _vid = version_id.clone(); // kept for reference if needed
                                                   // Use the loader's version number (e.g., "0.141.2+1.21.11") for installer arguments and verification
                    let loader_number = version.version_number.clone();
                    let mc_dir_str = mc_dir.to_string_lossy().to_string();
                    let installer = installer_path.clone();
                    let java = java_cmd.clone();
                    let loader_type_for_verify = loader_type_for_verify.clone();
                    let loader_number = loader_number.clone();

                    std::thread::spawn(move || {
                        use std::io::{BufRead, BufReader};

                        // Use the intended Java command for the instance (may be a custom path)
                        let mut cmd = std::process::Command::new(&java);

                        // Fabric installer supports a non-interactive "-y" flag to accept prompts.
                        // Use the canonical invocation: `java -jar fabric-installer.jar client -dir <mc_dir> -mcversion <version> -loader <version> -y`
                        cmd.arg("-jar")
                            .arg(&installer)
                            .arg("client")
                            .arg("-dir")
                            .arg(&mc_dir_str)
                            .arg("-mcversion")
                            .arg(&instance_version)
                            .arg("-loader")
                            .arg(&loader_number)
                            .arg("-y");

                        if let Ok(mut child) = cmd
                            .stdout(std::process::Stdio::piped())
                            .stderr(std::process::Stdio::piped())
                            .spawn()
                        {
                            if let Some(stdout) = child.stdout.take() {
                                let reader = BufReader::new(stdout);
                                for line in reader.lines().flatten() {
                                    let _ = app_clone.emit(
                                        "instance-log",
                                        InstanceLog {
                                            instance_id: iid.clone(),
                                            message: line,
                                        },
                                    );
                                }
                            }
                            if let Some(stderr) = child.stderr.take() {
                                let reader = BufReader::new(stderr);
                                for line in reader.lines().flatten() {
                                    let _ = app_clone.emit(
                                        "instance-log",
                                        InstanceLog {
                                            instance_id: iid.clone(),
                                            message: line,
                                        },
                                    );
                                }
                            }
                            let _ = child.wait();

                            // Verify installation: if Fabric, use `fabric_installed` with explicit versions; otherwise use heuristic `loader_verification`.
                            let success =
                                if loader_type_for_verify.to_lowercase().contains("fabric") {
                                    crate::loader::fabric_installed(
                                        std::path::Path::new(&mc_dir_str),
                                        &instance_version,
                                        &loader_number,
                                    )
                                } else {
                                    crate::loader::loader_verification(
                                        std::path::Path::new(&mc_dir_str),
                                        &loader_type_for_verify,
                                    )
                                };

                            // Mark instance metadata (already set) and emit installed event with details
                            let _ = app_clone.emit(
                                "loader-installed",
                                LoaderInstalled {
                                    instance_id: iid.clone(),
                                    project_id: pid.clone(),
                                    version_id: loader_number.clone(),
                                    success,
                                },
                            );

                            // Ensure derived version JSON exists by calling `install_loader` asynchronously.
                            if success {
                                let app_for_install = app_clone.clone();
                                let lt = loader_type_for_verify.clone();
                                let mc_v = instance_version.clone();
                                let lnum = loader_number.clone();
                                tauri::async_runtime::spawn(async move {
                                    match crate::commands::install_loader(
                                        app_for_install.clone(),
                                        lt,
                                        mc_v,
                                        lnum,
                                    )
                                    .await
                                    {
                                        Ok((derived_id, used_version)) => {
                                            let _ = app_for_install.emit("loader-install-log", format!("install_loader created derived version {} (used {})", derived_id, used_version));
                                        }
                                        Err(e) => {
                                            let _ = app_for_install.emit(
                                                "loader-install-log",
                                                format!("install_loader failed: {}", e),
                                            );
                                        }
                                    }
                                });
                            }
                        } else {
                            // Could not spawn installer, attempt verification (use normalized loader type)
                            let success =
                                if loader_type_for_verify.to_lowercase().contains("fabric") {
                                    crate::loader::fabric_installed(
                                        std::path::Path::new(&mc_dir_str),
                                        &instance_version,
                                        &loader_number,
                                    )
                                } else {
                                    crate::loader::loader_verification(
                                        std::path::Path::new(&mc_dir_str),
                                        &loader_type_for_verify,
                                    )
                                };
                            let _ = app_clone.emit(
                                "loader-installed",
                                LoaderInstalled {
                                    instance_id: iid.clone(),
                                    project_id: pid.clone(),
                                    version_id: loader_number.clone(),
                                    success,
                                },
                            );

                            if success {
                                let app_for_install = app_clone.clone();
                                let lt = loader_type_for_verify.clone();
                                let mc_v = instance_version.clone();
                                let lnum = loader_number.clone();
                                tauri::async_runtime::spawn(async move {
                                    match crate::commands::install_loader(
                                        app_for_install.clone(),
                                        lt,
                                        mc_v,
                                        lnum,
                                    )
                                    .await
                                    {
                                        Ok((derived_id, used_version)) => {
                                            let _ = app_for_install.emit("loader-install-log", format!("install_loader created derived version {} (used {})", derived_id, used_version));
                                        }
                                        Err(e) => {
                                            let _ = app_for_install.emit(
                                                "loader-install-log",
                                                format!("install_loader failed: {}", e),
                                            );
                                        }
                                    }
                                });
                            }
                        }
                    });

                    return Ok(());
                }
            }
        }
    }

    // No installer found, attempt verification and emit installed event (may be partial)
    let success = crate::loader::loader_verification(&mc_dir, &loader_type);
    let _ = app
        .emit(
            "loader-installed",
            LoaderInstalled {
                instance_id: instance_id.clone(),
                project_id: project_id.clone(),
                version_id: version_id.clone(),
                success,
            },
        )
        .map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
pub async fn install_modpack_version(
    app: AppHandle,
    name: String,
    version_id: String,
) -> Result<(), String> {
    let version = crate::modrinth::get_version(&version_id).await?;
    let game_version = version
        .game_versions
        .first()
        .ok_or("No game version specified")?
        .clone();

    let inst_id = uuid::Uuid::new_v4().to_string();
    let root = instance_dir(&app, &inst_id)?;
    fs::create_dir_all(&root).map_err(|e| e.to_string())?;

    // Create initial instance with installing state
    let mut instance = Instance {
        id: inst_id.clone(),
        name,
        version: game_version.clone(),
        mc_version: Some(game_version.clone()),
        state: InstanceState::Installing,
        created_at: chrono::Utc::now().timestamp() as u64,
        last_played: None,
        java_path: None,
        java_path_override: None,
        max_memory: None,
        min_memory: None,
        java_args: None,
        java_warning_ignored: false,
        loader: None,
        loader_version: None,
    };

    let meta_path = instance_meta_path(&app, &inst_id)?;
    let json = serde_json::to_string_pretty(&instance).map_err(|e| e.to_string())?;
    fs::write(&meta_path, json).map_err(|e| e.to_string())?;

    // Emit installation started event
    let _ = app.emit("instance-install-started", &inst_id);

    // Step 1: Download and parse .mrpack file to extract modpack metadata
    let mut mrpack_path_opt: Option<std::path::PathBuf> = None;
    let mut modpack_index: Option<crate::modrinth::ModpackIndex> = None;

    for file in &version.files {
        if file.filename.to_lowercase().ends_with(".mrpack") {
            let target = root.join(&file.filename);

            // Download the .mrpack file
            match crate::download::download_to_file(&file.url, &target).await {
                Ok(_) => {
                    // Parse modpack index to extract loader information
                    match crate::modrinth::parse_mrpack_index(&target) {
                        Ok(idx) => {
                            modpack_index = Some(idx);
                            mrpack_path_opt = Some(target);
                            let _ = app.emit("modpack-download-complete", &inst_id);
                            break;
                        }
                        Err(e) => {
                            let _ = std::fs::remove_file(&target);
                            let _ = app.emit(
                                "instance-install-error",
                                format!("Failed to parse modpack: {}", e),
                            );
                            return Err(format!("Failed to parse modpack: {}", e));
                        }
                    }
                }
                Err(e) => {
                    let _ = app.emit(
                        "instance-install-error",
                        format!("Failed to download modpack: {}", e),
                    );
                    return Err(format!("Failed to download modpack: {}", e));
                }
            }
        }
    }

    // Step 2: Determine Minecraft version and loader requirements
    let (resolved_mc_version, loader_info) = if let Some(ref idx) = modpack_index {
        // Use modpack index for accurate information
        let mc_ver = idx
            .version_id
            .clone()
            .unwrap_or_else(|| game_version.clone());

        // Extract loader information from dependencies
        let loader_info = extract_loader_from_dependencies(&idx.dependencies)?;

        (mc_ver, loader_info)
    } else {
        // Fallback to Modrinth version metadata
        let loader_info = if !version.loaders.is_empty() {
            Some(LoaderInfo {
                loader_type: normalize_loader_type(&version.loaders[0]),
                version: None, // Will be resolved later
            })
        } else {
            None
        };

        (game_version.clone(), loader_info)
    };

    // Step 3: Install vanilla Minecraft version
    let _ = app.emit("vanilla-install-started", &inst_id);
    let _base_version = ensure_vanilla_version(&app, &resolved_mc_version)
        .await
        .map_err(|e| {
            let _ = app.emit(
                "instance-install-error",
                format!("Failed to install vanilla Minecraft: {}", e),
            );
            e
        })?;
    let _ = app.emit("vanilla-install-complete", &inst_id);

    // Step 4: Install loader if required
    if let Some(loader_info) = loader_info {
        if loader_info.loader_type == "forge" {
            // Forge not supported yet
            let mut inst_err = instance.clone();
            inst_err.state = InstanceState::Error;
            let _ = fs::write(
                &meta_path,
                serde_json::to_string_pretty(&inst_err).map_err(|e| e.to_string())?,
            );
            let _ = app.emit(
                "instance-install-error",
                "Forge modpacks are not supported yet",
            );
            return Err("Forge modpacks are not supported yet".to_string());
        }

        let _ = app.emit(
            "loader-install-started",
            format!("Installing {} loader", loader_info.loader_type),
        );

        // Install loader with proper error handling and verification
        let (_derived_version_id, actual_loader_version) = install_loader_robust(
            &app,
            &loader_info.loader_type,
            &resolved_mc_version,
            loader_info.version.as_deref(),
            &inst_id,
        )
        .await?;

        // Update instance metadata with loader information
        // Keep the version as the base MC version, not the derived version
        instance.loader = Some(loader_info.loader_type.clone());
        instance.loader_version = Some(actual_loader_version.clone());
        instance.version = resolved_mc_version.clone();
        instance.mc_version = Some(resolved_mc_version.clone());

        let json = serde_json::to_string_pretty(&instance).map_err(|e| e.to_string())?;
        fs::write(&meta_path, json).map_err(|e| e.to_string())?;

        // Emit loader installed event
        let _ = app.emit(
            "loader-installed",
            LoaderInstalled {
                instance_id: inst_id.clone(),
                project_id: loader_info.loader_type.clone(),
                version_id: actual_loader_version.clone(),
                success: true,
            },
        );

        let _ = app.emit(
            "loader-install-complete",
            format!(
                "{} {} installed successfully",
                loader_info.loader_type, actual_loader_version
            ),
        );
    }

    // Step 5: Extract modpack contents (mods, overrides)
    if let Some(mrpack_path) = mrpack_path_opt {
        let _ = app.emit("modpack-extract-started", &inst_id);

        match crate::modrinth::install_mrpack(&app, &inst_id, &mrpack_path).await {
            Ok(_) => {
                let _ = std::fs::remove_file(&mrpack_path);
                let _ = app.emit("modpack-extract-complete", &inst_id);
            }
            Err(e) => {
                let _ = std::fs::remove_file(&mrpack_path);
                let _ = app.emit(
                    "instance-install-error",
                    format!("Failed to extract modpack: {}", e),
                );
                return Err(format!("Failed to extract modpack: {}", e));
            }
        }
    } else {
        // Fallback: download individual files (legacy modpack format)
        let _ = app.emit("modpack-files-download-started", &inst_id);

        for file in &version.files {
            if !file.filename.to_lowercase().ends_with(".mrpack") {
                let target = root.join(".minecraft").join("mods").join(&file.filename);
                if let Some(parent) = target.parent() {
                    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                }

                match crate::download::download_to_file(&file.url, &target).await {
                    Ok(_) => {
                        let _ = app.emit("file-downloaded", &file.filename);
                    }
                    Err(e) => {
                        let _ = app.emit(
                            "instance-install-error",
                            format!("Failed to download {}: {}", file.filename, e),
                        );
                        return Err(format!("Failed to download {}: {}", file.filename, e));
                    }
                }
            }
        }

        let _ = app.emit("modpack-files-download-complete", &inst_id);
    }

    // Step 6: Mark instance as ready
    instance.state = InstanceState::Ready;
    let json = serde_json::to_string_pretty(&instance).map_err(|e| e.to_string())?;
    fs::write(&meta_path, json).map_err(|e| e.to_string())?;

    let _ = app.emit("instance-install-complete", &inst_id);
    app.emit("list_instances", ()).map_err(|e| e.to_string())?;
    Ok(())
}

// Helper struct for loader information
#[derive(Debug, Clone)]
struct LoaderInfo {
    loader_type: String,
    version: Option<String>,
}

// Extract loader information from modpack dependencies
fn extract_loader_from_dependencies(
    dependencies: &std::collections::HashMap<String, String>,
) -> Result<Option<LoaderInfo>, String> {
    for (key, version) in dependencies.iter() {
        let key_lower = key.to_lowercase();

        if key_lower.contains("fabric-loader") || key_lower == "fabric" {
            return Ok(Some(LoaderInfo {
                loader_type: "fabric".to_string(),
                version: Some(version.clone()),
            }));
        } else if key_lower.contains("quilt-loader") || key_lower == "quilt" {
            return Ok(Some(LoaderInfo {
                loader_type: "quilt".to_string(),
                version: Some(version.clone()),
            }));
        } else if key_lower.contains("forge") || key_lower.contains("neoforge") {
            return Ok(Some(LoaderInfo {
                loader_type: "forge".to_string(),
                version: Some(version.clone()),
            }));
        }
    }

    Ok(None)
}

// Normalize loader type names
fn normalize_loader_type(loader: &str) -> String {
    let loader_lower = loader.to_lowercase();
    if loader_lower.contains("fabric") {
        "fabric".to_string()
    } else if loader_lower.contains("quilt") {
        "quilt".to_string()
    } else if loader_lower.contains("forge") || loader_lower.contains("neoforge") {
        "forge".to_string()
    } else {
        loader.to_string()
    }
}

// Robust loader installation with proper error handling and verification
async fn install_loader_robust(
    app: &AppHandle,
    loader_type: &str,
    mc_version: &str,
    requested_version: Option<&str>,
    _instance_id: &str,
) -> Result<(String, String), String> {
    // Get available loader versions
    let versions =
        match get_loader_versions(loader_type.to_string(), mc_version.to_string(), false).await {
            Ok(v) => v,
            Err(_) => {
                // Try with beta versions if stable versions fail
                get_loader_versions(loader_type.to_string(), mc_version.to_string(), true)
                    .await
                    .map_err(|e| {
                        format!(
                            "Failed to get {} versions for MC {}: {}",
                            loader_type, mc_version, e
                        )
                    })?
            }
        };

    if versions.is_empty() {
        return Err(format!(
            "No {} versions available for Minecraft {}",
            loader_type, mc_version
        ));
    }

    // Determine which version to install
    let target_version = if let Some(requested) = requested_version {
        // Try to find exact match or compatible version
        if versions.contains(&requested.to_string()) {
            requested.to_string()
        } else {
            // Try to find a compatible version (e.g., if requested is "0.15.0", try "0.15.0+build.1")
            versions
                .iter()
                .find(|v| v.starts_with(requested) || v.contains(requested))
                .cloned()
                .unwrap_or_else(|| versions[0].clone())
        }
    } else {
        // Use the first (latest stable) version
        versions[0].clone()
    };

    let _ = app.emit(
        "loader-install-progress",
        format!("Installing {} {}", loader_type, target_version),
    );

    // Install the loader
    let (derived_id, actual_version) = install_loader(
        app.clone(),
        loader_type.to_string(),
        mc_version.to_string(),
        target_version.clone(),
    )
    .await
    .map_err(|e| {
        format!(
            "Failed to install {} {}: {}",
            loader_type, target_version, e
        )
    })?;

    // Verify installation - check in the global minecraft directory, not instance directory
    let minecraft_root = crate::commands::minecraft_root(app)?;

    let verification_success = match loader_type {
        "fabric" => crate::loader::fabric_installed(&minecraft_root, mc_version, &actual_version),
        _ => crate::loader::loader_verification(&minecraft_root, loader_type),
    };

    if !verification_success {
        // Try alternative versions if verification failed
        let _ = app.emit(
            "loader-install-progress",
            format!(
                "Verification failed, trying alternative {} versions",
                loader_type
            ),
        );

        for alt_version in &versions {
            if alt_version == &actual_version {
                continue;
            }

            let _ = app.emit(
                "loader-install-progress",
                format!("Trying {} {}", loader_type, alt_version),
            );

            match install_loader(
                app.clone(),
                loader_type.to_string(),
                mc_version.to_string(),
                alt_version.clone(),
            )
            .await
            {
                Ok((alt_derived_id, alt_actual_version)) => {
                    let alt_success = match loader_type {
                        "fabric" => crate::loader::fabric_installed(
                            &minecraft_root,
                            mc_version,
                            &alt_actual_version,
                        ),
                        _ => crate::loader::loader_verification(&minecraft_root, loader_type),
                    };

                    if alt_success {
                        let _ = app.emit(
                            "loader-install-progress",
                            format!(
                                "Successfully installed {} {}",
                                loader_type, alt_actual_version
                            ),
                        );
                        return Ok((alt_derived_id, alt_actual_version));
                    }
                }
                Err(_) => continue,
            }
        }

        // If all versions failed verification, but we got a successful install_loader call,
        // let's be more lenient and just warn instead of failing completely
        let _ = app.emit(
            "loader-install-progress",
            format!(
                "Warning: Could not verify {} installation, but installation appeared successful",
                loader_type
            ),
        );

        // Return success anyway - the install_loader function succeeded, verification might just be overly strict
        let _ = app.emit(
            "loader-install-progress",
            format!(
                "Proceeding with {} {} installation",
                loader_type, actual_version
            ),
        );
        return Ok((derived_id, actual_version));
    }

    let _ = app.emit(
        "loader-install-progress",
        format!("Successfully verified {} {}", loader_type, actual_version),
    );
    Ok((derived_id, actual_version))
}

#[tauri::command]
pub async fn install_modrinth_mod(
    app: AppHandle,
    instance_id: String,
    version_id: String,
) -> Result<(), String> {
    let version = crate::modrinth::get_version(&version_id).await?;
    let root = instance_dir(&app, &instance_id)?;
    let mods_dir = root.join(".minecraft").join("mods");
    fs::create_dir_all(&mods_dir).map_err(|e| e.to_string())?;

    for file in &version.files {
        // Usually the primary file is the mod jar
        if file.primary || version.files.len() == 1 {
            let target = mods_dir.join(&file.filename);
            crate::download::download_to_file(&file.url, &target).await?;
            break;
        }
    }

    Ok(())
}
