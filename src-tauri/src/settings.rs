use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Settings {
    pub max_memory: u32, // in MB
    pub min_memory: u32, // in MB
    pub close_on_launch: bool,
    pub keep_logs_open: bool,
    pub global_java_args: String,
    pub global_java_path: Option<String>,
    #[serde(default)]
    pub skip_java_check: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            max_memory: 2048,
            min_memory: 512,
            close_on_launch: false,
            keep_logs_open: true,
            global_java_args: "-XX:+UseG1GC -Dsun.stdout.encoding=UTF-8".to_string(),
            global_java_path: None,
            skip_java_check: false,
        }
    }
}

pub fn settings_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_data_dir().map_err(|e| e.to_string())?;
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir.join("settings.json"))
}

#[tauri::command]
pub fn get_settings(app: AppHandle) -> Result<Settings, String> {
    let path = settings_path(&app)?;
    if !path.exists() {
        return Ok(Settings::default());
    }
    let text = fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_json::from_str(&text).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_settings(app: AppHandle, settings: Settings) -> Result<(), String> {
    let path = settings_path(&app)?;
    let text = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;
    fs::write(path, text).map_err(|e| e.to_string())
}
