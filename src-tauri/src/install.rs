use crate::{
    assets::{AssetIndexJson, AssetObject},
    download::{download_text, download_to_file},
    rules::rules_allow,
    version::VersionJson,
};

use futures_util::stream::{FuturesUnordered, StreamExt};
use once_cell::sync::Lazy;
use reqwest::Client;

use std::{
    fs,
    path::Path,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use tauri::{AppHandle, Emitter, Manager};
use tokio::{fs::File, io::AsyncWriteExt};

const ASSET_BASE_URL: &str = "https://resources.download.minecraft.net";

// 🔒 KEEP THIS LOW ON WINDOWS
const ASSET_CONCURRENCY: usize = 4;
const ASSET_RETRIES: usize = 3;

static HTTP_CLIENT: Lazy<Client> = Lazy::new(|| {
    Client::builder()
        .http1_only() // 🔥 critical fix
        .pool_max_idle_per_host(4)
        .pool_idle_timeout(Duration::from_secs(30))
        .tcp_keepalive(Duration::from_secs(30))
        .build()
        .expect("failed to build HTTP client")
});

/* ----------------------------- Libraries ----------------------------- */

pub async fn install_libraries(app: &AppHandle, version: &VersionJson) -> Result<(), String> {
    let base = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("minecraft")
        .join("libraries");

    for lib in &version.libraries {
        if !rules_allow(&lib.rules) {
            continue;
        }

        if let Some(artifact) = &lib.downloads.artifact {
            let target = base.join(&artifact.path);
            if !target.exists() {
                download_to_file(&artifact.url, &target).await?;
            }
        }

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
                    download_to_file(&artifact.url, &target).await?;
                }
            }
        }
    }

    Ok(())
}

/* ----------------------------- Client JAR ----------------------------- */

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
        .join(format!("{id}.jar"));

    if jar_path.exists() {
        return Ok(());
    }

    download_to_file(&version.downloads.client.url, &jar_path).await
}

/* ------------------------------ Assets -------------------------------- */

struct AssetProgress {
    downloaded_bytes: u64,
    downloaded_files: u64,
    start: Instant,
}

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

    // Download asset index
    let index_text = download_text(&version.assetIndex.url).await?;
    fs::write(
        indexes.join(format!("{}.json", version.assetIndex.id)),
        &index_text,
    )
    .map_err(|e| e.to_string())?;

    let index: AssetIndexJson = serde_json::from_str(&index_text).map_err(|e| e.to_string())?;

    let assets: Vec<AssetObject> = index.objects.values().cloned().collect();

    let progress = Arc::new(Mutex::new(AssetProgress {
        downloaded_bytes: 0,
        downloaded_files: 0,
        start: Instant::now(),
    }));

    let mut in_flight = FuturesUnordered::new();
    let mut iter = assets.into_iter();

    // Initial fill
    for _ in 0..ASSET_CONCURRENCY {
        if let Some(obj) = iter.next() {
            in_flight.push(spawn_asset(
                objects.clone(),
                app.clone(),
                progress.clone(),
                obj,
            ));
        }
    }

    while let Some(res) = in_flight.next().await {
        res?;

        if let Some(obj) = iter.next() {
            in_flight.push(spawn_asset(
                objects.clone(),
                app.clone(),
                progress.clone(),
                obj,
            ));
        }
    }

    app.emit("asset_done", ()).ok();
    Ok(())
}

fn spawn_asset(
    objects_dir: std::path::PathBuf,
    app: AppHandle,
    progress: Arc<Mutex<AssetProgress>>,
    obj: AssetObject,
) -> impl std::future::Future<Output = Result<(), String>> {
    async move {
        for attempt in 1..=ASSET_RETRIES {
            match download_asset_once(&objects_dir, &app, progress.clone(), &obj).await {
                Ok(()) => return Ok(()),
                Err(e) if attempt < ASSET_RETRIES => {
                    eprintln!(
                        "Retrying asset {} (attempt {}/{})",
                        obj.hash, attempt, ASSET_RETRIES
                    );
                    tokio::time::sleep(Duration::from_millis(500)).await;
                }
                Err(e) => return Err(e),
            }
        }
        Ok(())
    }
}

/* ------------------------ Asset Downloader ---------------------------- */

async fn download_asset_once(
    objects_dir: &Path,
    app: &AppHandle,
    progress: Arc<Mutex<AssetProgress>>,
    obj: &AssetObject,
) -> Result<(), String> {
    let hash = &obj.hash;
    let subdir = &hash[..2];
    let target = objects_dir.join(subdir).join(hash);

    if target.exists() {
        if fs::metadata(&target)
            .map(|m| m.len() == obj.size)
            .unwrap_or(false)
        {
            let mut p = progress.lock().unwrap();
            p.downloaded_files += 1;
            return Ok(());
        }
        let _ = fs::remove_file(&target);
    }

    fs::create_dir_all(target.parent().unwrap()).map_err(|e| e.to_string())?;

    let url = format!("{ASSET_BASE_URL}/{subdir}/{hash}");

    let response = HTTP_CLIENT
        .get(&url)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !response.status().is_success() {
        return Err(format!("HTTP {} for {}", response.status(), url));
    }

    let mut stream = response.bytes_stream();
    let mut file = File::create(&target).await.map_err(|e| e.to_string())?;

    while let Some(chunk) = stream.next().await {
        let chunk = match chunk {
            Ok(c) => c,
            Err(e) => {
                let _ = fs::remove_file(&target);
                return Err(e.to_string());
            }
        };

        file.write_all(&chunk).await.map_err(|e| {
            let _ = fs::remove_file(&target);
            e.to_string()
        })?;

        let (downloaded, speed, eta) = {
            let mut p = progress.lock().unwrap();
            p.downloaded_bytes += chunk.len() as u64;

            let elapsed = p.start.elapsed().as_secs_f64().max(0.001);
            let speed = p.downloaded_bytes as f64 / elapsed;
            let remaining = obj.size.saturating_sub(p.downloaded_bytes) as f64;
            let eta = remaining / speed;

            (p.downloaded_bytes, speed, eta)
        };

        app.emit(
            "asset_progress",
            serde_json::json!({
                "downloadedBytes": downloaded,
                "totalBytes": obj.size,
                "speed": speed,
                "eta": eta
            }),
        )
        .ok();
        std::println!(
            "Downloading asset {}: {}/{} bytes ({:.2} bytes/s, ETA {:.1}s)",
            hash,
            downloaded,
            obj.size,
            speed,
            eta
        );
    }

    file.flush().await.map_err(|e| e.to_string())?;

    let size = fs::metadata(&target).map_err(|e| e.to_string())?.len();
    if size != obj.size {
        let _ = fs::remove_file(&target);
        return Err(format!(
            "size mismatch for {} (expected {}, got {})",
            hash, obj.size, size
        ));
    }

    let mut p = progress.lock().unwrap();
    p.downloaded_files += 1;

    Ok(())
}
