use log::{info, warn};
use serde::Serialize;
use serde_json::{Map, Value};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use tauri::async_runtime::spawn_blocking;
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

#[derive(Serialize)]
pub struct ProfileImportResult {
    pub metadata_name: Option<String>,
}

#[tauri::command]
pub async fn export_profile_zip(profile_path: String, destination: String) -> Result<(), String> {
    spawn_blocking(move || export_profile_zip_blocking(profile_path, destination))
        .await
        .map_err(|e| format!("Export task failed: {e}"))?
}

fn export_profile_zip_blocking(profile_path: String, destination: String) -> Result<(), String> {
    let profile_dir = Path::new(&profile_path);
    if !profile_dir.exists() || !profile_dir.is_dir() {
        return Err(format!(
            "Profile directory does not exist: {}",
            profile_path
        ));
    }

    let destination_path = Path::new(&destination);
    if let Some(parent) = destination_path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let sanitized_metadata = build_sanitized_metadata(profile_dir)?;

    let output = File::create(destination_path).map_err(|e| e.to_string())?;
    let mut zip = ZipWriter::new(output);
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    let mut metadata_written = false;

    add_directory_to_zip(
        &mut zip,
        profile_dir,
        profile_dir,
        destination_path,
        options,
        &sanitized_metadata,
        &mut metadata_written,
    )?;

    if !metadata_written {
        zip.start_file("metadata.json", options)
            .map_err(|e| e.to_string())?;
        zip.write_all(sanitized_metadata.as_bytes())
            .map_err(|e| e.to_string())?;
    }

    zip.finish().map_err(|e| e.to_string())?;
    info!("Exported profile zip: {} -> {}", profile_path, destination);
    Ok(())
}

#[tauri::command]
pub async fn import_profile_zip(
    zip_path: String,
    destination: String,
) -> Result<ProfileImportResult, String> {
    spawn_blocking(move || import_profile_zip_blocking(zip_path, destination))
        .await
        .map_err(|e| format!("Import task failed: {e}"))?
}

fn import_profile_zip_blocking(
    zip_path: String,
    destination: String,
) -> Result<ProfileImportResult, String> {
    let zip_file = File::open(&zip_path).map_err(|e| e.to_string())?;
    let mut archive = ZipArchive::new(zip_file).map_err(|e| e.to_string())?;

    let destination_path = Path::new(&destination);
    fs::create_dir_all(destination_path).map_err(|e| e.to_string())?;

    let root_prefix = detect_common_root_prefix(&mut archive)?;
    let mut metadata_name: Option<String> = None;

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
        let Some(raw_entry_path) = entry.enclosed_name().map(|p| p.to_path_buf()) else {
            warn!("Skipping entry {} with unsafe path", i);
            continue;
        };

        let relative_path = strip_root_prefix(&raw_entry_path, root_prefix.as_deref());
        if relative_path.as_os_str().is_empty() {
            continue;
        }

        let out_path = destination_path.join(&relative_path);
        if entry.is_dir() {
            fs::create_dir_all(&out_path).map_err(|e| e.to_string())?;
            continue;
        }

        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }

        if is_metadata_file(&relative_path) {
            let mut bytes = Vec::new();
            entry.read_to_end(&mut bytes).map_err(|e| e.to_string())?;
            fs::write(&out_path, &bytes).map_err(|e| e.to_string())?;
            if metadata_name.is_none() {
                metadata_name = extract_name_from_metadata(&bytes);
            }
        } else {
            let mut output = File::create(&out_path).map_err(|e| e.to_string())?;
            std::io::copy(&mut entry, &mut output).map_err(|e| e.to_string())?;
        }

        #[cfg(unix)]
        if let Some(mode) = entry.unix_mode() {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&out_path, fs::Permissions::from_mode(mode)).ok();
        }
    }

    info!("Imported profile zip: {} -> {}", zip_path, destination);
    Ok(ProfileImportResult {
        metadata_name: metadata_name
            .map(|n| n.trim().to_string())
            .filter(|n| !n.is_empty()),
    })
}

fn add_directory_to_zip(
    zip: &mut ZipWriter<File>,
    root_dir: &Path,
    current_dir: &Path,
    destination_path: &Path,
    options: SimpleFileOptions,
    sanitized_metadata: &str,
    metadata_written: &mut bool,
) -> Result<(), String> {
    let entries = fs::read_dir(current_dir).map_err(|e| e.to_string())?;
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path == destination_path {
            continue;
        }

        let relative = path.strip_prefix(root_dir).map_err(|e| e.to_string())?;
        if should_skip_export_file(relative) {
            continue;
        }

        let zip_path = to_zip_path(relative)?;
        if zip_path.is_empty() {
            continue;
        }

        if path.is_dir() {
            zip.add_directory(format!("{zip_path}/"), options)
                .map_err(|e| e.to_string())?;
            add_directory_to_zip(
                zip,
                root_dir,
                &path,
                destination_path,
                options,
                sanitized_metadata,
                metadata_written,
            )?;
            continue;
        }

        zip.start_file(zip_path, options)
            .map_err(|e| e.to_string())?;
        if is_metadata_file(relative) {
            *metadata_written = true;
            zip.write_all(sanitized_metadata.as_bytes())
                .map_err(|e| e.to_string())?;
        } else {
            let mut file = File::open(&path).map_err(|e| e.to_string())?;
            std::io::copy(&mut file, zip).map_err(|e| e.to_string())?;
        }
    }

    Ok(())
}

fn build_sanitized_metadata(profile_dir: &Path) -> Result<String, String> {
    let metadata_path = profile_dir.join("metadata.json");
    let mut metadata = match fs::read_to_string(&metadata_path) {
        Ok(content) => parse_metadata_object(&content),
        Err(_) => Map::new(),
    };

    metadata.remove("id");
    metadata.remove("path");
    metadata.remove("created_at");
    metadata.remove("total_play_time");
    metadata.insert("bepinex_installed".to_string(), Value::Bool(true));

    if !metadata.contains_key("name") {
        metadata.insert(
            "name".to_string(),
            Value::String(default_profile_name(profile_dir)),
        );
    }
    if !metadata.contains_key("mods") {
        metadata.insert("mods".to_string(), Value::Array(vec![]));
    }

    serde_json::to_string_pretty(&Value::Object(metadata)).map_err(|e| e.to_string())
}

fn parse_metadata_object(content: &str) -> Map<String, Value> {
    serde_json::from_str::<Value>(content)
        .ok()
        .and_then(|value| value.as_object().cloned())
        .unwrap_or_default()
}

fn default_profile_name(profile_dir: &Path) -> String {
    profile_dir
        .file_name()
        .and_then(|name| name.to_str())
        .map(std::string::ToString::to_string)
        .unwrap_or_else(|| "Imported Profile".to_string())
}

fn detect_common_root_prefix(archive: &mut ZipArchive<File>) -> Result<Option<String>, String> {
    let mut prefix: Option<String> = None;

    for i in 0..archive.len() {
        let entry = archive.by_index(i).map_err(|e| e.to_string())?;
        let Some(path) = entry.enclosed_name() else {
            continue;
        };

        let parts: Vec<String> = path
            .components()
            .filter_map(|component| match component {
                Component::Normal(part) => Some(part.to_string_lossy().to_string()),
                _ => None,
            })
            .collect();
        if parts.is_empty() {
            continue;
        }

        // A top-level directory entry like "profile/" should not block root-prefix detection.
        if parts.len() == 1 && entry.is_dir() {
            continue;
        }

        // A real top-level file/entry means there's no single-folder root wrapper.
        if parts.len() == 1 {
            return Ok(None);
        }

        let first_component = parts[0].clone();
        match &prefix {
            None => prefix = Some(first_component),
            Some(existing) if existing == &first_component => {}
            Some(_) => return Ok(None),
        }
    }

    Ok(prefix)
}

fn strip_root_prefix(path: &Path, root_prefix: Option<&str>) -> PathBuf {
    let mut components = path.components();
    if let Some(prefix) = root_prefix
        && let Some(Component::Normal(first)) = components.next()
        && first == prefix
    {
        return components.as_path().to_path_buf();
    }
    path.to_path_buf()
}

fn extract_name_from_metadata(bytes: &[u8]) -> Option<String> {
    serde_json::from_slice::<Value>(bytes)
        .ok()
        .and_then(|value| {
            value
                .get("name")
                .and_then(Value::as_str)
                .map(str::to_string)
        })
}

fn is_metadata_file(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.eq_ignore_ascii_case("metadata.json"))
        .unwrap_or(false)
}

fn should_skip_export_file(path: &Path) -> bool {
    let is_log_file = path
        .file_name()
        .and_then(|name| name.to_str())
        .map(|name| {
            name.eq_ignore_ascii_case("errorlog.log") || name.eq_ignore_ascii_case("logoutput.log")
        })
        .unwrap_or(false);
    if !is_log_file {
        return false;
    }

    path.components().any(|component| {
        matches!(
            component,
            Component::Normal(name) if name.to_string_lossy().eq_ignore_ascii_case("bepinex")
        )
    })
}

fn to_zip_path(path: &Path) -> Result<String, String> {
    let mut parts = Vec::new();
    for component in path.components() {
        match component {
            Component::Normal(segment) => parts.push(segment.to_string_lossy().to_string()),
            Component::CurDir => {}
            _ => {
                return Err(format!("Unsupported path in zip entry: {:?}", path));
            }
        }
    }
    Ok(parts.join("/"))
}
