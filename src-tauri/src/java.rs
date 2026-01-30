use std::fs;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Manager};

/* ============================================================
 * Java Version Requirements
 * ============================================================ */

/// Determines the required Java version for a given Minecraft version
pub fn get_required_java_version(mc_version: &str) -> u8 {
    // Parse version to determine Java requirement
    // MC 1.18+ requires Java 17
    // MC 1.17 requires Java 16
    // MC 1.12-1.16.5 requires Java 8
    // MC <1.12 requires Java 8

    if let Some(version_num) = parse_version(mc_version) {
        if version_num >= (1, 18, 0) {
            return 17;
        } else if version_num >= (1, 17, 0) {
            return 16;
        }
    }

    // Default to Java 8 for older versions
    8
}

/// Parse Minecraft version string into (major, minor, patch) tuple
fn parse_version(version: &str) -> Option<(u32, u32, u32)> {
    let parts: Vec<&str> = version.split('.').collect();

    if parts.len() < 2 {
        return None;
    }

    let major = parts[0].parse::<u32>().ok()?;
    let minor = parts[1].parse::<u32>().ok()?;
    let patch = if parts.len() > 2 {
        parts[2].parse::<u32>().unwrap_or(0)
    } else {
        0
    };

    Some((major, minor, patch))
}

/* ============================================================
 * Java Detection
 * ============================================================ */

/// Get the major version of a Java executable
pub fn get_java_major_version(path: &str) -> Option<u8> {
    let output = std::process::Command::new(path)
        .arg("-version")
        .output()
        .ok()?;

    let text = String::from_utf8_lossy(&output.stderr);
    let first_line = text.lines().next()?;

    // Examples:
    // "openjdk version \"17.0.1\" 2021-10-19"
    // "java version \"1.8.0_311\""
    let version_part = first_line.split_whitespace().nth(2)?.trim_matches('"');
    let parts: Vec<&str> = version_part.split('.').collect();

    if !parts.is_empty() {
        if parts[0] == "1" && parts.len() > 1 {
            return parts[1].parse().ok();
        } else {
            return parts[0].parse().ok();
        }
    }
    None
}

/// Find Java installation on the system
pub fn find_system_java(required_version: u8) -> Result<Option<String>, String> {
    // Try common Java locations on Windows
    let mut search_paths = vec![
        format!(
            "C:\\Program Files\\Java\\jdk-{}\\bin\\java.exe",
            required_version
        ),
        format!(
            "C:\\Program Files\\Java\\jre-{}\\bin\\java.exe",
            required_version
        ),
        format!(
            "C:\\Program Files (x86)\\Java\\jdk-{}\\bin\\java.exe",
            required_version
        ),
        format!(
            "C:\\Program Files (x86)\\Java\\jre-{}\\bin\\java.exe",
            required_version
        ),
    ];

    // Add PATH entries
    if let Ok(output) = std::process::Command::new("where").arg("java").output() {
        if output.status.success() {
            if let Ok(path) = String::from_utf8(output.stdout) {
                for line in path.lines() {
                    let trimmed = line.trim();
                    if !trimmed.is_empty() {
                        search_paths.push(trimmed.to_string());
                    }
                }
            }
        }
    }

    for path in search_paths {
        if PathBuf::from(&path).exists() {
            if let Some(version) = get_java_major_version(&path) {
                if version == required_version {
                    return Ok(Some(path));
                }
            }
        }
    }

    Ok(None)
}

/* ============================================================
 * Java Download (Adoptium/Temurin)
 * ============================================================ */

/// Download and install Java for the launcher
pub async fn download_java(app: &AppHandle, version: u8) -> Result<String, String> {
    let java_dir = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("java")
        .join(format!("jdk-{}", version));

    // Check if already downloaded
    let java_exe = java_dir.join("bin").join("java.exe");
    if java_exe.exists() {
        return Ok(java_exe.to_string_lossy().to_string());
    }

    println!("📥 Downloading Java {} from Adoptium...", version);

    // Adoptium API endpoint for Windows x64 JRE
    let api_url = format!(
        "https://api.adoptium.net/v3/binary/latest/{}/ga/windows/x64/jre/hotspot/normal/eclipse",
        version
    );

    // Download the zip file
    let response = reqwest::get(&api_url)
        .await
        .map_err(|e| format!("Failed to download Java: {}", e))?;

    if !response.status().is_success() {
        return Err(format!(
            "Failed to download Java: HTTP {}",
            response.status()
        ));
    }

    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Failed to read Java download: {}", e))?;

    // Save to temp file
    let temp_dir = std::env::temp_dir();
    let zip_path = temp_dir.join(format!("java-{}.zip", version));
    fs::write(&zip_path, &bytes).map_err(|e| e.to_string())?;

    println!("📦 Extracting Java {}...", version);

    // Extract the zip file
    extract_zip(&zip_path, java_dir.parent().unwrap())?;

    // Clean up zip file
    let _ = fs::remove_file(&zip_path);

    // Find the actual extracted directory (Adoptium uses various naming schemes)
    let extracted_root = java_dir.parent().unwrap();
    for entry in fs::read_dir(extracted_root).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.is_dir() {
            let name = path.file_name().unwrap().to_string_lossy();
            // Check for names like jdk-17.0.1+12 or jdk8u312-b07
            if name.contains(&format!("jdk-{}", version))
                || name.contains(&format!("jdk{}", version))
                || name.contains(&format!("jre-{}", version))
                || name.contains(&format!("jre{}", version))
            {
                // Rename to our expected directory name
                if path != java_dir {
                    if java_dir.exists() {
                        fs::remove_dir_all(&java_dir).map_err(|e| e.to_string())?;
                    }
                    fs::rename(&path, &java_dir).map_err(|e| e.to_string())?;
                }
                break;
            }
        }
    }

    if java_exe.exists() {
        println!("✓ Java {} installed successfully", version);
        Ok(java_exe.to_string_lossy().to_string())
    } else {
        Err(format!("Java extraction failed: java.exe not found"))
    }
}

/// Extract a zip file to a destination directory
fn extract_zip(zip_path: &PathBuf, dest: &Path) -> Result<(), String> {
    let file = fs::File::open(zip_path).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| e.to_string())?;
        let outpath = dest.join(file.name());

        if file.name().ends_with('/') {
            fs::create_dir_all(&outpath).map_err(|e| e.to_string())?;
        } else {
            if let Some(p) = outpath.parent() {
                fs::create_dir_all(p).map_err(|e| e.to_string())?;
            }
            let mut outfile = fs::File::create(&outpath).map_err(|e| e.to_string())?;
            std::io::copy(&mut file, &mut outfile).map_err(|e| e.to_string())?;
        }
    }

    Ok(())
}

/* ============================================================
 * Java Management
 * ============================================================ */

/// Ensure Java is available for the given Minecraft version
pub async fn ensure_java(app: &AppHandle, mc_version: &str) -> Result<String, String> {
    let required_version = get_required_java_version(mc_version);
    let settings = crate::settings::get_settings(app.clone()).unwrap_or_default();

    println!(
        "🔍 Minecraft {} requires Java {}",
        mc_version, required_version
    );

    // 1. Check global custom path first if provided and matches version
    if let Some(global_path) = &settings.global_java_path {
        if PathBuf::from(global_path).exists() {
            if let Some(v) = get_java_major_version(global_path) {
                if v == required_version {
                    println!("✓ Using global custom Java: {}", global_path);
                    return Ok(global_path.clone());
                } else {
                    println!("⚠ Global custom Java version mismatch (found {}, need {}). Falling back to detection.", v, required_version);
                }
            }
        }
    }

    // 2. Try to find system Java
    if let Some(java_path) = find_system_java(required_version)? {
        println!("✓ Found system Java at: {}", java_path);
        return Ok(java_path);
    }

    // 3. If not found, download it
    println!(
        "⚠ Java {} not found on system, downloading...",
        required_version
    );
    download_java(app, required_version).await
}

/// Helper to get the Java path that WILL be used for an instance
pub fn get_intended_java_path(app: &AppHandle, instance: &crate::instance::Instance) -> String {
    let settings = crate::settings::get_settings(app.clone()).unwrap_or_default();

    settings
        .global_java_path
        .as_deref()
        .or(instance.java_path_override.as_deref())
        .or(instance.java_path.as_deref())
        .unwrap_or("java")
        .to_string()
}
