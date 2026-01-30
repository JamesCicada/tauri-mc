mod assets;
mod commands;
mod download;
mod install;
mod instance;
mod java;
mod launch;
mod minecraft;
mod modrinth;
mod rules;
mod settings;
mod version;

use commands::{
    check_java_compatibility, check_version_usage, create_instance, delete_instance,
    download_loader_version, download_version, find_loader_candidates, get_popular_mods,
    get_project_versions, get_version_manifest, install_modpack_version, install_modrinth_mod,
    kill_instance, launch_instance, list_instances, save_instance, search_projects,
    ChildProcessState,
};
use settings::{get_settings, save_settings};
use tauri::Manager;

fn main() {
    tauri::Builder::default()
        .manage(ChildProcessState::default())
        .setup(|app| {
            // Reset "Running" or "Installing" states on startup
            let app_handle = app.handle();
            let data_dir = app_handle
                .path()
                .app_data_dir()
                .unwrap()
                .join("minecraft")
                .join("instances");
            if data_dir.exists() {
                if let Ok(entries) = std::fs::read_dir(data_dir) {
                    for entry in entries.flatten() {
                        let meta_path = entry.path().join("instance.json");
                        if meta_path.exists() {
                            if let Ok(text) = std::fs::read_to_string(&meta_path) {
                                if let Ok(mut inst) =
                                    serde_json::from_str::<instance::Instance>(&text)
                                {
                                    if inst.state == instance::InstanceState::Running
                                        || inst.state == instance::InstanceState::Installing
                                    {
                                        inst.state = instance::InstanceState::Ready;
                                        if let Ok(updated) = serde_json::to_string_pretty(&inst) {
                                            let _ = std::fs::write(&meta_path, updated);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_version_manifest,
            download_version,
            launch_instance,
            list_instances,
            create_instance,
            delete_instance,
            check_version_usage,
            check_java_compatibility,
            get_settings,
            save_settings,
            save_instance,
            kill_instance,
            search_projects,
            get_project_versions,
            get_popular_mods,
            install_modpack_version,
            install_modrinth_mod,
            find_loader_candidates,
            download_loader_version,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
