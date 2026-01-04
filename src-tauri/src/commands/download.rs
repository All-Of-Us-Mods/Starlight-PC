use futures_util::StreamExt;
use log::{debug, error, info, warn};
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Runtime};

#[derive(Clone, serde::Serialize)]
pub struct DownloadProgress {
    pub stage: String,
    pub progress: f64,
    pub message: String,
}

const CONNECT_TIMEOUT: Duration = Duration::from_secs(30);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(300);

fn emit_progress<R: Runtime>(app: &AppHandle<R>, stage: &str, progress: f64, message: &str) {
    let _ = app.emit(
        "download-progress",
        DownloadProgress {
            stage: stage.to_string(),
            progress,
            message: message.to_string(),
        },
    );
}

async fn download_file<R: Runtime>(
    app: &AppHandle<R>,
    url: &str,
    dest_path: &Path,
) -> Result<(), String> {
    info!("Starting download from: {}", url);
    emit_progress(app, "downloading", 0.0, "Starting download...");
    let client = reqwest::Client::builder()
        .connect_timeout(CONNECT_TIMEOUT)
        .timeout(REQUEST_TIMEOUT)
        .build()
        .map_err(|e| {
            error!("Failed to create HTTP client: {}", e);
            e.to_string()
        })?;

    let response = client.get(url).send().await.map_err(|e| {
        error!("Download request failed: {}", e);
        e.to_string()
    })?;
    if !response.status().is_success() {
        error!("Download failed with status: {}", response.status());
        return Err(format!("Download failed: {}", response.status()));
    }

    let total_size = response.content_length();
    debug!("Download size: {:?} bytes", total_size);
    let mut downloaded: u64 = 0;

    if let Some(parent) = dest_path.parent() {
        fs::create_dir_all(parent).map_err(|e| {
            error!("Failed to create directory {:?}: {}", parent, e);
            e.to_string()
        })?;
    }

    let mut temp_file = File::create(dest_path).map_err(|e| {
        error!("Failed to create file {:?}: {}", dest_path, e);
        e.to_string()
    })?;
    let mut stream = response.bytes_stream();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| {
            error!("Error reading download chunk: {}", e);
            e.to_string()
        })?;
        temp_file.write_all(&chunk).map_err(|e| {
            error!("Error writing to file: {}", e);
            e.to_string()
        })?;
        downloaded += chunk.len() as u64;
        if let Some(total) = total_size {
            let progress = downloaded as f64 / total as f64 * 100.0;
            emit_progress(
                app,
                "downloading",
                progress,
                &format!("Downloading... {:.1}%", progress),
            );
        }
    }
    drop(temp_file);
    info!("Download completed: {:?}", dest_path);
    Ok(())
}

fn extract_zip<R: Runtime>(
    app: &AppHandle<R>,
    zip_path: &Path,
    dest_path: &Path,
) -> Result<(), String> {
    info!("Extracting zip: {:?} to {:?}", zip_path, dest_path);
    emit_progress(app, "extracting", 0.0, "Extracting...");
    let file = File::open(zip_path).map_err(|e| {
        error!("Failed to open zip file {:?}: {}", zip_path, e);
        e.to_string()
    })?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| {
        error!("Failed to read zip archive: {}", e);
        e.to_string()
    })?;
    let total_files = archive.len();
    debug!("Zip archive contains {} files", total_files);

    fs::create_dir_all(dest_path).map_err(|e| {
        error!("Failed to create destination directory {:?}: {}", dest_path, e);
        e.to_string()
    })?;

    for i in 0..total_files {
        let mut entry = archive.by_index(i).map_err(|e| {
            error!("Failed to read zip entry {}: {}", i, e);
            e.to_string()
        })?;
        let Some(name) = entry.enclosed_name() else {
            continue;
        };
        let outpath = dest_path.join(name);

        if entry.is_dir() {
            fs::create_dir_all(&outpath).ok();
        } else {
            if let Some(p) = outpath.parent() {
                fs::create_dir_all(p).ok();
            }
            let mut outfile = File::create(&outpath).map_err(|e| {
                error!("Failed to create file {:?}: {}", outpath, e);
                e.to_string()
            })?;
            std::io::copy(&mut entry, &mut outfile).map_err(|e| {
                error!("Failed to extract file {:?}: {}", outpath, e);
                e.to_string()
            })?;

            #[cfg(unix)]
            if let Some(mode) = entry.unix_mode() {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(&outpath, fs::Permissions::from_mode(mode)).ok();
            }
        }
        let progress = (i + 1) as f64 / total_files as f64 * 100.0;
        emit_progress(
            app,
            "extracting",
            progress,
            &format!("Extracting... {}/{}", i + 1, total_files),
        );
    }
    info!("Extraction completed: {} files", total_files);
    Ok(())
}

#[tauri::command]
pub async fn download_and_extract_zip<R: Runtime>(
    app: AppHandle<R>,
    url: String,
    destination: String,
    cache_path: Option<String>,
) -> Result<(), String> {
    info!("download_and_extract_zip: url={}, destination={}", url, destination);
    let dest_path = Path::new(&destination);

    // Check if we should use cached file
    if let Some(ref cache) = cache_path {
        let cache_file = Path::new(cache);
        if cache_file.exists() {
            info!("Using cached file: {:?}", cache_file);
            emit_progress(&app, "extracting", 0.0, "Using cached BepInEx...");
            extract_zip(&app, cache_file, dest_path)?;
            emit_progress(&app, "complete", 100.0, "Installation complete!");
            return Ok(());
        }
    }

    // Download to temp file
    let temp_path = dest_path.with_extension("zip.tmp");
    download_file(&app, &url, &temp_path).await?;

    // If caching is enabled, copy to cache location
    if let Some(ref cache) = cache_path {
        let cache_file = Path::new(cache);
        if let Some(parent) = cache_file.parent() {
            fs::create_dir_all(parent).ok();
        }
        if let Err(e) = fs::copy(&temp_path, cache_file) {
            warn!("Failed to cache BepInEx: {}", e);
        } else {
            debug!("Cached BepInEx to: {:?}", cache_file);
        }
    }

    // Extract ZIP
    extract_zip(&app, &temp_path, dest_path)?;

    let _ = fs::remove_file(&temp_path);
    emit_progress(&app, "complete", 100.0, "Installation complete!");
    info!("download_and_extract_zip completed successfully");
    Ok(())
}

#[tauri::command]
pub async fn download_bepinex_to_cache<R: Runtime>(
    app: AppHandle<R>,
    url: String,
    cache_path: String,
) -> Result<(), String> {
    info!("download_bepinex_to_cache: url={}, cache_path={}", url, cache_path);
    let cache_file = Path::new(&cache_path);
    download_file(&app, &url, cache_file).await?;
    emit_progress(&app, "complete", 100.0, "Download complete!");
    info!("BepInEx download to cache completed");
    Ok(())
}

#[tauri::command]
pub async fn clear_bepinex_cache(cache_path: String) -> Result<(), String> {
    info!("clear_bepinex_cache: {}", cache_path);
    let cache_file = Path::new(&cache_path);
    if cache_file.exists() {
        fs::remove_file(cache_file).map_err(|e| {
            error!("Failed to clear cache file {:?}: {}", cache_file, e);
            e.to_string()
        })?;
        info!("Cache cleared: {:?}", cache_file);
    }
    Ok(())
}

#[tauri::command]
pub async fn check_bepinex_cache_exists(cache_path: String) -> Result<bool, String> {
    let cache_file = Path::new(&cache_path);
    let exists = cache_file.exists();
    debug!("check_bepinex_cache_exists: {} -> {}", cache_path, exists);
    Ok(exists)
}
