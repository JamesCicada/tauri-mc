use crate::instance::Instance;
use serde::Serialize;
use std::path::{Path, PathBuf};
use tauri::AppHandle;

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

pub fn fabric_installed(minecraft_root: &Path, mc_version: &str, loader_version: &str) -> bool {
    let version_id = format!("fabric-{}-{}", mc_version, loader_version);

    let version_json: PathBuf = minecraft_root
        .join("versions")
        .join(&version_id)
        .join(format!("{}.json", version_id));

    version_json.exists()
}

pub fn loader_verification(mc_dir: &std::path::Path, project_id: &str) -> bool {
    println!(
        "loader_verification: checking {} in {}",
        project_id,
        mc_dir.display()
    );

    // Heuristic checks: look for version folders or libraries that indicate Fabric/Quilt
    let versions_dir = mc_dir.join("versions");
    if versions_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&versions_dir) {
            for e in entries.flatten() {
                if let Ok(name) = e.file_name().into_string() {
                    let lname = name.to_lowercase();
                    let pid = project_id.to_lowercase();
                    if pid.contains("fabric") && lname.contains("fabric") {
                        println!("loader_verification: found fabric version folder: {}", name);
                        return true;
                    }
                    if pid.contains("quilt") && lname.contains("quilt") {
                        println!("loader_verification: found quilt version folder: {}", name);
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
                    println!("loader_verification: found fabric lib: {}", name);
                    return true;
                }
                if pid.contains("quilt") && lname.contains("quilt") {
                    println!("loader_verification: found quilt lib: {}", name);
                    return true;
                }
            }
        }
    }

    println!(
        "loader_verification: no indicators found for {}",
        project_id
    );
    false
}

#[tauri::command]
pub async fn get_loader_versions(
    loader_type: String,
    mc_version: String,
    include_beta: bool,
) -> Result<Vec<String>, String> {
    println!(
        "get_loader_versions: loader={} mc={} include_beta={}",
        loader_type, mc_version, include_beta
    );
    // List available loader versions from Fabric/Quilt meta endpoints
    let list_url = match loader_type.as_str() {
        "fabric" => format!(
            "https://meta.fabricmc.net/v2/versions/loader/{}",
            mc_version
        ),
        "quilt" => format!("https://meta.quiltmc.org/v3/versions/loader/{}", mc_version),
        other => return Err(format!("Unsupported loader type: {}", other)),
    };

    let text = crate::download::download_text(&list_url).await?;
    let list_val: serde_json::Value = serde_json::from_str(&text).map_err(|e| {
        let snippet: String = text.chars().take(200).collect();
        format!("{} - response (truncated): {}", e.to_string(), snippet)
    })?;

    let arr = list_val
        .as_array()
        .ok_or("unexpected loader list response")?;
    let mut stable: Vec<String> = Vec::new();
    let mut beta: Vec<String> = Vec::new();

    for item in arr {
        // Derive the version string from multiple possible fields
        let s_opt = item
            .get("version")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .or_else(|| {
                item.get("id")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
            })
            .or_else(|| {
                item.get("loader")
                    .and_then(|l| l.get("version"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
            });

        if let Some(s) = s_opt {
            // Prefer explicit stable flag when present (e.g., item.loader.stable)
            let mut explicit_stable: Option<bool> = None;
            if let Some(loader_obj) = item.get("loader") {
                if let Some(b) = loader_obj.get("stable").and_then(|v| v.as_bool()) {
                    explicit_stable = Some(b);
                }
            }
            if explicit_stable.is_none() {
                if let Some(b) = item.get("stable").and_then(|v| v.as_bool()) {
                    explicit_stable = Some(b);
                }
            }

            let is_beta = if let Some(is_stable) = explicit_stable {
                !is_stable
            } else {
                // fallback heuristic
                let sl = s.to_lowercase();
                sl.contains("beta")
                    || sl.contains("alpha")
                    || sl.contains("rc")
                    || sl.contains("preview")
                    || s.contains('-')
            };

            if is_beta {
                beta.push(s);
            } else {
                stable.push(s);
            }
        }
    }

    // If no stable releases found, fall back to returning beta/pre-release versions so UI has options
    if stable.is_empty() {
        if include_beta {
            stable.extend(beta.into_iter());
            return Ok(stable);
        }
        return Ok(beta);
    }

    if include_beta {
        stable.extend(beta.into_iter());
    }

    Ok(stable)
}

#[tauri::command]
pub async fn find_loader_candidates(
    app: AppHandle,
    instance_id: String,
    loader: String,
) -> Result<Vec<LoaderCandidate>, String> {
    println!(
        "find_loader_candidates: loader={} instance={}",
        loader, instance_id
    );
    // Read instance to get game version
    let root = crate::commands::instance_dir(&app, &instance_id)?;
    let meta_text =
        std::fs::read_to_string(root.join("instance.json")).map_err(|e| e.to_string())?;
    let inst: Instance = serde_json::from_str(&meta_text).map_err(|e| e.to_string())?;
    let mc_version = inst.mc_version.clone().unwrap_or(inst.version.clone());

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
