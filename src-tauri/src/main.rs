mod assets;
mod commands;
mod download;
mod install;
mod instance;
mod launch;
mod minecraft;
mod rules;
mod version;
use commands::{
    create_instance, download_version, get_version_manifest, launch_instance, list_instances,
};

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            get_version_manifest,
            download_version,
            launch_instance,
            list_instances,
            create_instance,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
