use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use tauri::{AppHandle, Emitter, Manager, State};

use crate::install::{install_assets, install_client_jar, install_libraries};
use crate::instance::{Instance, InstanceState};
use crate::java::ensure_java;
use crate::launch::build_classpath;
use crate::minecraft::{get_manifest, McVersion};
use crate::version::VersionJson;
use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Default)]
pub struct ChildProcessState(pub Mutex<HashMap<String, std::process::Child>>);

fn minecraft_root(app: &AppHandle) -> Result<PathBuf, String> {
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
        version,
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
        let version_id = {
            let meta_text =
                fs::read_to_string(dir.join("instance.json")).map_err(|e| e.to_string())?;
            let instance: Instance = serde_json::from_str(&meta_text).map_err(|e| e.to_string())?;
            instance.version
        };

        let other_uses = instances
            .iter()
            .filter(|i| i.id != instance_id && i.version == version_id)
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

    let version_id = &instance.version;

    let version_json_path = versions_root(&app)?
        .join(version_id)
        .join(format!("{version_id}.json"));

    let text = fs::read_to_string(&version_json_path).map_err(|e| e.to_string())?;

    let version: VersionJson = serde_json::from_str(&text).map_err(|e| e.to_string())?;

    let classpath = build_classpath(&app, version_id, &version)?;
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

    let required_version = crate::java::get_required_java_version(&instance.version);
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

#[derive(Serialize, Clone)]
pub struct LoaderCandidate {
    pub project_id: String,
    pub project_title: String,
    pub version_id: String,
    pub version_number: String,
    pub game_versions: Vec<String>,
}

#[derive(Serialize, Clone)]
pub struct LoaderInstalled {
    pub instance_id: String,
    pub project_id: String,
    pub version_id: String,
    pub success: bool,
}

fn loader_verification(mc_dir: &std::path::Path, project_id: &str) -> bool {
    // Heuristic checks: look for version folders or libraries that indicate Fabric/Quilt
    let versions_dir = mc_dir.join("versions");
    if versions_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&versions_dir) {
            for e in entries.flatten() {
                if let Ok(name) = e.file_name().into_string() {
                    let lname = name.to_lowercase();
                    let pid = project_id.to_lowercase();
                    if pid.contains("fabric") && lname.contains("fabric") {
                        return true;
                    }
                    if pid.contains("quilt") && lname.contains("quilt") {
                        return true;
                    }
                }
            }
        }
    }

    let libs = mc_dir.join("libraries");
    if libs.exists() {
        let pid = project_id.to_lowercase();
        let mut stack: Vec<std::path::PathBuf> = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&libs) {
            for e in entries.flatten() {
                stack.push(e.path());
            }
        }
        while let Some(p) = stack.pop() {
            if p.is_dir() {
                if let Ok(iter) = std::fs::read_dir(&p) {
                    for e in iter.flatten() {
                        stack.push(e.path());
                    }
                }
            } else if let Some(name) = p.file_name().and_then(|s| s.to_str()) {
                let lname = name.to_lowercase();
                if pid.contains("fabric") && lname.contains("fabric") {
                    return true;
                }
                if pid.contains("quilt") && lname.contains("quilt") {
                    return true;
                }
            }
        }
    }

    false
}

#[tauri::command]
pub async fn find_loader_candidates(
    app: AppHandle,
    instance_id: String,
    loader: String,
) -> Result<Vec<LoaderCandidate>, String> {
    // Read instance to get game version
    let root = instance_dir(&app, &instance_id)?;
    let meta_text =
        std::fs::read_to_string(root.join("instance.json")).map_err(|e| e.to_string())?;
    let inst: Instance = serde_json::from_str(&meta_text).map_err(|e| e.to_string())?;
    let mc_version = inst.version.clone();

    // Search Modrinth for projects matching loader term
    let search = crate::modrinth::search_projects(&loader, "mod").await?;
    let mut results: Vec<LoaderCandidate> = Vec::new();

    // Also include popular/popular loader projects by searching for common loader names if initial search returned none
    if search.hits.is_empty() {
        let common = vec!["fabric", "forge", "quilt"];
        for name in common.iter() {
            if let Ok(pop) = crate::modrinth::search_projects(name, "mod").await {
                for hit in pop.hits {
                    if let Ok(versions) =
                        crate::modrinth::get_project_versions(&hit.project_id).await
                    {
                        for v in versions {
                            if v.game_versions.iter().any(|g| g == &mc_version) {
                                results.push(LoaderCandidate {
                                    project_id: hit.project_id.clone(),
                                    project_title: hit.title.clone(),
                                    version_id: v.id.clone(),
                                    version_number: v.version_number.clone(),
                                    game_versions: v.game_versions.clone(),
                                });
                            }
                        }
                    }
                }
            }
        }
    } else {
        for hit in search.hits.iter() {
            if let Ok(versions) = crate::modrinth::get_project_versions(&hit.project_id).await {
                for v in versions.into_iter() {
                    // Compatible if version.game_versions includes mc_version
                    if v.game_versions.iter().any(|g| g == &mc_version) {
                        results.push(LoaderCandidate {
                            project_id: hit.project_id.clone(),
                            project_title: hit.title.clone(),
                            version_id: v.id.clone(),
                            version_number: v.version_number.clone(),
                            game_versions: v.game_versions.clone(),
                        });
                    }
                }
            }
        }
    }

    for hit in search.hits.iter() {
        if let Ok(versions) = crate::modrinth::get_project_versions(&hit.project_id).await {
            for v in versions.into_iter() {
                // Compatible if version.game_versions includes mc_version
                if v.game_versions.iter().any(|g| g == &mc_version) {
                    results.push(LoaderCandidate {
                        project_id: hit.project_id.clone(),
                        project_title: hit.title.clone(),
                        version_id: v.id.clone(),
                        version_number: v.version_number.clone(),
                        game_versions: v.game_versions.clone(),
                    });
                }
            }
        }
    }

    Ok(results)
}

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

    // Update instance metadata to record loader presence after download
    if let Ok(meta_text) = std::fs::read_to_string(root.join("instance.json")) {
        if let Ok(mut inst) = serde_json::from_str::<Instance>(&meta_text) {
            inst.loader = Some(project_id.clone());
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

                    // Clone for thread
                    let app_clone = app.clone();
                    let iid = instance_id.clone();
                    let pid = project_id.clone();
                    let vid = version_id.clone();
                    let mc_dir_str = mc_dir.to_string_lossy().to_string();
                    let installer = installer_path.clone();
                    let java = java_cmd.clone();

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
                            .arg(&vid)
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

                            // Verify installation heuristically
                            let success =
                                loader_verification(std::path::Path::new(&mc_dir_str), &pid);

                            // Mark instance metadata (already set) and emit installed event with details
                            let _ = app_clone.emit(
                                "loader-installed",
                                LoaderInstalled {
                                    instance_id: iid.clone(),
                                    project_id: pid.clone(),
                                    version_id: vid.clone(),
                                    success,
                                },
                            );
                        } else {
                            // Could not spawn installer, attempt verification
                            let success =
                                loader_verification(std::path::Path::new(&mc_dir_str), &pid);
                            let _ = app_clone.emit(
                                "loader-installed",
                                LoaderInstalled {
                                    instance_id: iid.clone(),
                                    project_id: pid.clone(),
                                    version_id: vid.clone(),
                                    success,
                                },
                            );
                        }
                    });

                    return Ok(());
                }
            }
        }
    }

    // No installer found, attempt verification and emit installed event (may be partial)
    let success = loader_verification(&mc_dir, &project_id);
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

    // Detect loader if present in the Modrinth version metadata
    let (detected_loader, detected_loader_version) = if !version.loaders.is_empty() {
        (
            Some(version.loaders[0].clone()),
            Some(version.version_number.clone()),
        )
    } else {
        (None, None)
    };

    let instance = Instance {
        id: inst_id.clone(),
        name,
        version: game_version.clone(),
        state: InstanceState::Installing,
        created_at: chrono::Utc::now().timestamp() as u64,
        last_played: None,
        java_path: None,
        java_path_override: None,
        max_memory: None,
        min_memory: None,
        java_args: None,
        java_warning_ignored: false,
        loader: detected_loader.clone(),
        loader_version: detected_loader_version.clone(),
    };

    let meta_path = instance_meta_path(&app, &inst_id)?;
    let json = serde_json::to_string_pretty(&instance).map_err(|e| e.to_string())?;
    fs::write(&meta_path, json).map_err(|e| e.to_string())?;

    // Download files and handle modpack (.mrpack) extraction
    let mods_dir = root.join(".minecraft").join("mods");
    fs::create_dir_all(&mods_dir).map_err(|e| e.to_string())?;

    for file in &version.files {
        // If a modpack file is present, download and extract it to the instance
        if file.filename.to_lowercase().ends_with(".mrpack") {
            let target = root.join(&file.filename);
            crate::download::download_to_file(&file.url, &target).await?;
            // Install contents of mrpack into the instance (.minecraft)
            let _index = crate::modrinth::install_mrpack(&app, &inst_id, &target).await?;
            // Remove the mrpack archive after extraction to avoid clutter
            let _ = std::fs::remove_file(&target);
        } else {
            let target = mods_dir.join(&file.filename);
            crate::download::download_to_file(&file.url, &target).await?;
        }
    }

    // If the Modrinth version lists a loader, try to find and auto-install a candidate
    if let Some(loader_name) = version.loaders.first().cloned() {
        if let Ok(mut candidates) =
            find_loader_candidates(app.clone(), inst_id.clone(), loader_name.clone()).await
        {
            if !candidates.is_empty() {
                // Prefer quilt/fabric/forge in that order when choosing a loader automatically
                candidates.sort_by_key(|c| {
                    let title = c.project_title.to_lowercase();
                    if title.contains("quilt") {
                        0
                    } else if title.contains("fabric") {
                        1
                    } else if title.contains("forge") {
                        2
                    } else {
                        3
                    }
                });

                if let Some(pick) = candidates.first() {
                    // Attempt to download and run installer for the chosen loader version
                    let _ = download_loader_version(
                        app.clone(),
                        inst_id.clone(),
                        pick.project_id.clone(),
                        pick.version_id.clone(),
                    )
                    .await;
                }
            }
        }
    }

    // Update state to Ready
    let mut ready_inst = instance;
    ready_inst.state = InstanceState::Ready;
    let json = serde_json::to_string_pretty(&ready_inst).map_err(|e| e.to_string())?;
    fs::write(&meta_path, json).map_err(|e| e.to_string())?;

    // If we detected a loader, emit a notification event so the frontend can show a toast
    if let Some(loader) = detected_loader {
        let _ = app.emit("modpack-loader-detected", loader);
    }

    app.emit("list_instances", ()).map_err(|e| e.to_string())?;
    Ok(())
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
