use crate::version::VersionJson;
use std::path::PathBuf;
use tauri::AppHandle;
use tauri::Manager;

pub fn build_classpath(app: &AppHandle, id: &str, version: &VersionJson) -> Result<String, String> {
    let base = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("minecraft");

    let mut entries: Vec<PathBuf> = Vec::new();

    // Libraries
    let os_key = if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "osx"
    } else {
        "linux"
    };

    for lib in &version.libraries {
        if !crate::rules::rules_allow(&lib.rules) {
            continue;
        }

        if let Some(artifact) = &lib.downloads.artifact {
            entries.push(base.join("libraries").join(&artifact.path));
        }

        if let Some(classifier) = lib.natives.get(os_key) {
            if let Some(artifact) = lib.downloads.classifiers.get(classifier) {
                entries.push(base.join("libraries").join(&artifact.path));
            }
        }
    }

    // Client jar LAST
    entries.push(base.join("versions").join(id).join(format!("{}.jar", id)));

    let sep = if cfg!(windows) { ";" } else { ":" };

    Ok(entries
        .iter()
        .map(|p| p.to_string_lossy())
        .collect::<Vec<_>>()
        .join(sep))
}
