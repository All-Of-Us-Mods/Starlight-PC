use crate::backend::error::{AppError, AppResult};
use log::{debug, info};
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;
use std::time::Duration;
use uuid::Uuid;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(30);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(300);

#[derive(Clone, Debug, serde::Serialize)]
pub struct ModDownloadProgress {
    pub mod_id: String,
    pub downloaded: u64,
    pub total: Option<u64>,
    pub progress: f64,
    pub stage: String,
}

fn emit_progress(mod_id: &str, downloaded: u64, total: Option<u64>, stage: &str) {
    let progress = total
        .map(|t| downloaded as f64 / t as f64 * 100.0)
        .unwrap_or(0.0);
    crate::backend::events::publish(crate::backend::events::BackendEvent::ModDownloadProgress(
        ModDownloadProgress {
            mod_id: mod_id.to_string(),
            downloaded,
            total,
            progress,
            stage: stage.to_string(),
        },
    ));
}

pub fn download_mod(
    mod_id: String,
    url: String,
    destination: String,
    expected_checksum: Option<String>,
) -> AppResult<()> {
    let dest_path = Path::new(&destination);
    if let Some(parent) = dest_path.parent() {
        fs::create_dir_all(parent)?;
    }

    let tracking_id = get_tracking_id()?;

    let agent = ureq::AgentBuilder::new()
        .timeout_connect(CONNECT_TIMEOUT)
        .timeout(REQUEST_TIMEOUT)
        .build();

    emit_progress(&mod_id, 0, None, "connecting");

    let response = agent.get(&url).set("X-Starlight-ID", &tracking_id).call()?;

    let total_size: Option<u64> = response
        .header("Content-Length")
        .and_then(|s| s.parse().ok());
    debug!("Download size: {:?}", total_size);

    let mut hasher = Sha256::new();
    let mut downloaded: u64 = 0;
    let mut buffer = Vec::new();
    let mut chunk = vec![0u8; 64 * 1024];

    emit_progress(&mod_id, 0, total_size, "downloading");

    let mut reader = response.into_reader();
    loop {
        let n = reader.read(&mut chunk)?;
        if n == 0 {
            break;
        }
        hasher.update(&chunk[..n]);
        buffer.extend_from_slice(&chunk[..n]);
        downloaded += n as u64;
        emit_progress(&mod_id, downloaded, total_size, "downloading");
    }

    emit_progress(&mod_id, downloaded, total_size, "verifying");
    let computed_checksum = format!("{:x}", hasher.finalize());
    if let Some(expected_checksum) = expected_checksum.filter(|checksum| !checksum.is_empty())
        && computed_checksum != expected_checksum.to_lowercase()
    {
        return Err(AppError::validation(format!(
            "Checksum mismatch: expected {}, got {}",
            expected_checksum, computed_checksum
        )));
    }

    emit_progress(&mod_id, downloaded, total_size, "writing");
    let mut file = File::create(dest_path)?;
    file.write_all(&buffer)?;

    emit_progress(&mod_id, downloaded, total_size, "complete");
    info!("Mod download completed: {} -> {:?}", mod_id, dest_path);
    Ok(())
}

fn get_tracking_id() -> AppResult<String> {
    use std::fs;
    let dir = crate::backend::directories::app_data_dir()?;
    fs::create_dir_all(&dir)?;
    let path = dir.join("tracking_id");
    if let Ok(existing) = fs::read_to_string(&path) {
        let trimmed = existing.trim();
        if !trimmed.is_empty() {
            return Ok(trimmed.to_string());
        }
    }
    let new_id = Uuid::new_v4().to_string();
    fs::write(&path, &new_id)?;
    Ok(new_id)
}
