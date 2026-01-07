//! Download and extraction utilities with progress reporting

use futures_util::StreamExt;
use log::{debug, error, info, warn};
use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use std::time::Duration;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(30);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(300);

/// Downloads a file from URL to destination, calling on_progress(downloaded, total) during download
pub async fn download_file<F>(url: &str, dest_path: &Path, on_progress: F) -> Result<(), String>
where
    F: Fn(u64, Option<u64>),
{
    info!("Downloading: {}", url);
    let client = reqwest::Client::builder()
        .connect_timeout(CONNECT_TIMEOUT)
        .timeout(REQUEST_TIMEOUT)
        .build()
        .map_err(|e| {
            error!("Failed to create HTTP client: {}", e);
            e.to_string()
        })?;

    let response = client.get(url).send().await.map_err(|e| {
        error!("Download failed: {}", e);
        e.to_string()
    })?;

    if !response.status().is_success() {
        let status = response.status();
        error!("Download failed with status: {}", status);
        return Err(format!("Download failed: {}", status));
    }

    let total_size = response.content_length();
    debug!("Download size: {:?} bytes", total_size);

    if let Some(parent) = dest_path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let mut file = File::create(dest_path).map_err(|e| {
        error!("Failed to create file {:?}: {}", dest_path, e);
        e.to_string()
    })?;

    let mut stream = response.bytes_stream();
    let mut downloaded: u64 = 0;
    on_progress(0, total_size);

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| e.to_string())?;
        file.write_all(&chunk).map_err(|e| e.to_string())?;
        downloaded += chunk.len() as u64;
        on_progress(downloaded, total_size);
    }

    info!("Download completed: {:?}", dest_path);
    Ok(())
}

/// Extracts a ZIP archive to destination, calling on_progress(current, total) during extraction
pub fn extract_zip<F>(zip_path: &Path, dest_path: &Path, on_progress: F) -> Result<(), String>
where
    F: Fn(usize, usize),
{
    info!("Extracting: {:?} -> {:?}", zip_path, dest_path);
    let file = File::open(zip_path).map_err(|e| {
        error!("Failed to open zip: {}", e);
        e.to_string()
    })?;

    let mut archive = zip::ZipArchive::new(file).map_err(|e| {
        error!("Invalid zip archive: {}", e);
        e.to_string()
    })?;

    let total = archive.len();
    fs::create_dir_all(dest_path).map_err(|e| e.to_string())?;
    on_progress(0, total);

    for i in 0..total {
        let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;

        // enclosed_name() returns None for entries with ".." or absolute paths (zip slip protection)
        let Some(name) = entry.enclosed_name() else {
            warn!("Skipping entry {} with unsafe path", i);
            continue;
        };

        let outpath = dest_path.join(name);

        if entry.is_dir() {
            fs::create_dir_all(&outpath).ok();
        } else {
            if let Some(parent) = outpath.parent() {
                fs::create_dir_all(parent).ok();
            }
            let mut outfile = File::create(&outpath).map_err(|e| e.to_string())?;
            std::io::copy(&mut entry, &mut outfile).map_err(|e| e.to_string())?;

            #[cfg(unix)]
            if let Some(mode) = entry.unix_mode() {
                use std::os::unix::fs::PermissionsExt;
                fs::set_permissions(&outpath, fs::Permissions::from_mode(mode)).ok();
            }
        }

        on_progress(i + 1, total);
    }

    info!("Extraction completed: {} files", total);
    Ok(())
}
