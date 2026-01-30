use std::fs;
use std::path::Path;

pub async fn download_to_file(url: &str, path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let bytes = reqwest::get(url)
        .await
        .map_err(|e| e.to_string())?
        .bytes()
        .await
        .map_err(|e| e.to_string())?;

    fs::write(path, bytes).map_err(|e| e.to_string())?;
    Ok(())
}
pub async fn download_text(url: &str) -> Result<String, String> {
    let res = reqwest::get(url).await.map_err(|e| e.to_string())?;
    let status = res.status();
    let text = res.text().await.map_err(|e| e.to_string())?;
    if !status.is_success() {
        let snippet: String = text.chars().take(200).collect();
        return Err(format!("HTTP {} response: {}", status.as_u16(), snippet));
    }
    Ok(text)
}
