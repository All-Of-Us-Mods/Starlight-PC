//! BepInEx installation and cache management commands

use crate::utils::download::{download_file, extract_zip};
use log::{debug, error, info, warn};
use std::fs;
use std::path::Path;
use tauri::{AppHandle, Emitter, Runtime};

#[derive(Clone, serde::Serialize)]
struct Progress {
    stage: String,
    progress: f64,
    message: String,
}

fn emit<R: Runtime>(app: &AppHandle<R>, stage: &str, progress: f64, message: &str) {
    let _ = app.emit(
        "bepinex-progress",
        Progress {
            stage: stage.to_string(),
            progress,
            message: message.to_string(),
        },
    );
}

/// Installs BepInEx from URL or cache to the destination
#[tauri::command]
pub async fn install_bepinex<R: Runtime>(
    app: AppHandle<R>,
    url: String,
    destination: String,
    cache_path: Option<String>,
) -> Result<(), String> {
    info!("install_bepinex: {} -> {}", url, destination);

    let dest = Path::new(&destination);

    // Use cache if available
    if let Some(ref cache) = cache_path {
        let cache_file = Path::new(cache);
        if cache_file.exists() {
            info!("Using cached BepInEx");
            emit(&app, "extracting", 0.0, "Using cached BepInEx...");
            extract_zip(cache_file, dest, |cur, total| {
                emit(
                    &app,
                    "extracting",
                    cur as f64 / total as f64 * 100.0,
                    &format!("Extracting {}/{}", cur, total),
                );
            })?;
            emit(&app, "complete", 100.0, "Complete!");
            return Ok(());
        }
    }

    // Download
    let temp = dest.with_extension("zip.tmp");
    emit(&app, "downloading", 0.0, "Downloading...");
    download_file(&url, &temp, |dl, total| {
        if let Some(t) = total {
            emit(
                &app,
                "downloading",
                dl as f64 / t as f64 * 100.0,
                &format!("Downloading... {:.0}%", dl as f64 / t as f64 * 100.0),
            );
        }
    })
    .await?;

    // Cache if enabled
    if let Some(ref cache) = cache_path {
        let cache_file = Path::new(cache);
        if let Some(parent) = cache_file.parent() {
            fs::create_dir_all(parent).ok();
        }
        if let Err(e) = fs::copy(&temp, cache_file) {
            warn!("Failed to cache: {}", e);
        } else {
            debug!("Cached to {:?}", cache_file);
        }
    }

    // Extract
    emit(&app, "extracting", 0.0, "Extracting...");
    extract_zip(&temp, dest, |cur, total| {
        emit(
            &app,
            "extracting",
            cur as f64 / total as f64 * 100.0,
            &format!("Extracting {}/{}", cur, total),
        );
    })?;

    fs::remove_file(&temp).ok();
    emit(&app, "complete", 100.0, "Complete!");
    info!("install_bepinex completed");
    Ok(())
}

/// Downloads BepInEx to cache
#[tauri::command]
pub async fn download_bepinex_to_cache<R: Runtime>(
    app: AppHandle<R>,
    url: String,
    cache_path: String,
) -> Result<(), String> {
    info!("download_bepinex_to_cache: {}", cache_path);

    let cache_file = Path::new(&cache_path);

    emit(&app, "downloading", 0.0, "Downloading...");
    download_file(&url, cache_file, |dl, total| {
        if let Some(t) = total {
            emit(
                &app,
                "downloading",
                dl as f64 / t as f64 * 100.0,
                &format!("Downloading... {:.0}%", dl as f64 / t as f64 * 100.0),
            );
        }
    })
    .await?;

    emit(&app, "complete", 100.0, "Complete!");
    Ok(())
}

/// Clears BepInEx cache
#[tauri::command]
pub async fn clear_bepinex_cache(cache_path: String) -> Result<(), String> {
    let cache_file = Path::new(&cache_path);
    if cache_file.exists() {
        fs::remove_file(cache_file).map_err(|e| {
            error!("Failed to clear cache: {}", e);
            e.to_string()
        })?;
        info!("Cache cleared: {:?}", cache_file);
    }
    Ok(())
}

/// Checks if BepInEx cache exists
#[tauri::command]
pub async fn check_bepinex_cache_exists(cache_path: String) -> Result<bool, String> {
    Ok(Path::new(&cache_path).exists())
}
