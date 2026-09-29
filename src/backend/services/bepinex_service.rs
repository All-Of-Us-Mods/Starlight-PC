use crate::backend::binary::BinaryArch;
use crate::backend::error::AppResult;
use crate::backend::services::bepinex_runtime::BepInExRuntime;
use crate::backend::services::http_download::{download_file, extract_zip};
use log::{debug, info, warn};
use std::fs;
use std::path::Path;

#[derive(Clone, Copy, Debug)]
pub enum BepInExTargetType {
    Profile,
    Cache,
}

#[derive(Clone, Debug)]
pub struct BepInExProgress {
    pub stage: String,
    pub progress: f64,
    pub message: String,
    pub target_type: BepInExTargetType,
    pub target_id: String,
}

fn emit(
    stage: &str,
    progress: f64,
    message: &str,
    target_type: BepInExTargetType,
    target_id: &str,
) {
    crate::backend::events::publish(crate::backend::events::BackendEvent::BepInExProgress(
        BepInExProgress {
            stage: stage.to_string(),
            progress,
            message: message.to_string(),
            target_type,
            target_id: target_id.to_string(),
        },
    ));
}

/// Return the current whole percent only when it differs from the last one
/// emitted. Download callbacks run once per 64 KiB chunk and ZIP extraction
/// callbacks run once per entry, so publishing every callback can keep the UI
/// event loop busy long enough that the progress bar does not repaint.
fn changed_percent(current: u64, total: u64, last_emitted: &mut Option<u8>) -> Option<u8> {
    if total == 0 {
        return None;
    }

    let percent = ((current.saturating_mul(100) / total).min(100)) as u8;
    if *last_emitted == Some(percent) {
        return None;
    }
    *last_emitted = Some(percent);
    Some(percent)
}

/// `download_file` progress callback: emit at most once per whole percentage
/// point. No-op until the total size is known.
fn emit_download_progress(
    downloaded: u64,
    total: Option<u64>,
    target_type: BepInExTargetType,
    target_id: &str,
    last_emitted: &mut Option<u8>,
) {
    if let Some(pct) = total.and_then(|total| changed_percent(downloaded, total, last_emitted)) {
        emit(
            "downloading",
            f64::from(pct),
            &format!("Downloading... {pct}%"),
            target_type,
            target_id,
        );
    }
}

/// `extract_zip` progress callback: emit an "extracting" event for entry
/// `current` of `total`.
fn emit_extract_progress(
    current: usize,
    total: usize,
    target_type: BepInExTargetType,
    target_id: &str,
    last_emitted: &mut Option<u8>,
) {
    let Some(pct) = changed_percent(current as u64, total as u64, last_emitted) else {
        return;
    };
    emit(
        "extracting",
        f64::from(pct),
        &format!("Extracting {current}/{total}"),
        target_type,
        target_id,
    );
}

/// Keep runtime replacement and verification together. Profile metadata never
/// participates in deciding which build is installed.
pub fn ensure_installed(
    runtime: &BepInExRuntime<'_>,
    architecture: BinaryArch,
    url: &str,
    cache_path: Option<&str>,
    profile_id: &str,
) -> AppResult<()> {
    if !runtime.needs_install(architecture) {
        return Ok(());
    }

    // Remove arch-specific files before extraction; preserve plugins and config.
    for dir in [runtime.dotnet_dir(), runtime.core_dir()] {
        if dir.exists() {
            fs::remove_dir_all(&dir)?;
        }
    }

    if let Err(error) = install_bepinex(url, runtime.root(), cache_path, profile_id)
        .and_then(|()| runtime.validate(architecture))
    {
        emit(
            "failed",
            0.0,
            &error.to_string(),
            BepInExTargetType::Profile,
            profile_id,
        );
        return Err(error);
    }
    emit(
        "complete",
        100.0,
        "Complete!",
        BepInExTargetType::Profile,
        profile_id,
    );
    Ok(())
}

fn install_bepinex(
    url: &str,
    dest: &Path,
    cache_path: Option<&str>,
    target_id: &str,
) -> AppResult<()> {
    info!("install_bepinex: {} -> {}", url, dest.display());
    let target_type = BepInExTargetType::Profile;
    let cache_file = cache_path.map(Path::new);
    let temp = dest.with_extension("zip.tmp");

    let archive = if let Some(cache) = cache_file.filter(|path| path.is_file()) {
        info!("Using cached BepInEx");
        cache
    } else {
        emit("downloading", 0.0, "Downloading...", target_type, target_id);
        let mut last_download_percent = Some(0);
        download_file(url, &temp, None, None, |dl, total| {
            emit_download_progress(
                dl,
                total,
                target_type,
                target_id,
                &mut last_download_percent,
            )
        })?;

        if let Some(cache) = cache_file {
            if let Some(parent) = cache.parent() {
                fs::create_dir_all(parent).ok();
            }
            if let Err(e) = fs::copy(&temp, cache) {
                warn!("Failed to cache: {}", e);
            } else {
                debug!("Cached to {:?}", cache);
            }
        }
        temp.as_path()
    };

    emit("extracting", 0.0, "Extracting...", target_type, target_id);
    let mut last_extract_percent = Some(0);
    let result = extract_zip(archive, dest, |cur, total| {
        emit_extract_progress(
            cur,
            total,
            target_type,
            target_id,
            &mut last_extract_percent,
        )
    });

    if archive == temp {
        fs::remove_file(&temp).ok();
    }
    result
}

pub fn download_bepinex_to_cache(
    url: String,
    cache_path: String,
    architecture: String,
) -> AppResult<()> {
    let cache_file = Path::new(&cache_path);

    emit(
        "downloading",
        0.0,
        "Downloading...",
        BepInExTargetType::Cache,
        &architecture,
    );
    let mut last_download_percent = Some(0);
    download_file(&url, cache_file, None, None, |dl, total| {
        emit_download_progress(
            dl,
            total,
            BepInExTargetType::Cache,
            &architecture,
            &mut last_download_percent,
        )
    })?;

    emit(
        "complete",
        100.0,
        "Complete!",
        BepInExTargetType::Cache,
        &architecture,
    );
    Ok(())
}

pub fn clear_cache(cache_path: String, architecture: String) -> AppResult<()> {
    let cache_file = Path::new(&cache_path);
    if cache_file.exists() {
        fs::remove_file(cache_file)?;
    }
    emit(
        "cleared",
        0.0,
        "Cache cleared",
        BepInExTargetType::Cache,
        &architecture,
    );
    Ok(())
}

pub fn cache_size(cache_path: &str) -> Option<u64> {
    fs::metadata(cache_path).ok().map(|m| m.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::test_support::{TempDir, pe_bytes, write_test_pe};
    use std::io::Write;

    fn cached_runtime(dir: &Path, machine: Option<u16>) -> String {
        let cache = dir.join("bepinex.zip");
        let mut zip = zip::ZipWriter::new(fs::File::create(&cache).unwrap());
        let options = zip::write::SimpleFileOptions::default();
        zip.start_file("BepInEx/core/BepInEx.Unity.IL2CPP.dll", options)
            .unwrap();
        zip.write_all(b"managed").unwrap();
        if let Some(machine) = machine {
            let coreclr = BepInExRuntime::new(dir).coreclr_path();
            zip.start_file(
                format!("dotnet/{}", coreclr.file_name().unwrap().to_string_lossy()),
                options,
            )
            .unwrap();
            zip.write_all(&pe_bytes(machine)).unwrap();
        }
        zip.finish().unwrap();
        cache.to_string_lossy().into_owned()
    }

    #[test]
    fn cached_install_replaces_runtime_and_preserves_plugins_and_config() {
        let dir = TempDir::new("runtime-replacement");
        let root = dir.0.join("profile");
        let runtime = BepInExRuntime::new(&root);
        fs::create_dir_all(runtime.dotnet_dir()).unwrap();
        fs::create_dir_all(runtime.core_dir()).unwrap();
        fs::write(runtime.assembly_path(), b"managed").unwrap();
        write_test_pe(&runtime.coreclr_path(), 0x014c);
        assert_eq!(runtime.installed_arch(), Some(BinaryArch::X86));
        fs::write(runtime.dotnet_dir().join("old-arch.dll"), b"old").unwrap();
        fs::write(runtime.core_dir().join("old-arch.dll"), b"old").unwrap();
        fs::create_dir_all(root.join("BepInEx/plugins")).unwrap();
        fs::create_dir_all(root.join("BepInEx/config")).unwrap();
        fs::write(root.join("BepInEx/plugins/Mod.dll"), b"plugin").unwrap();
        fs::write(root.join("BepInEx/config/Mod.cfg"), b"config").unwrap();
        let cache = cached_runtime(&dir.0, Some(0x8664));
        ensure_installed(&runtime, BinaryArch::X64, "unused", Some(&cache), "test").unwrap();

        assert_eq!(runtime.installed_arch(), Some(BinaryArch::X64));
        assert!(!runtime.dotnet_dir().join("old-arch.dll").exists());
        assert!(!runtime.core_dir().join("old-arch.dll").exists());
        assert_eq!(
            fs::read(root.join("BepInEx/plugins/Mod.dll")).unwrap(),
            b"plugin"
        );
        assert_eq!(
            fs::read(root.join("BepInEx/config/Mod.cfg")).unwrap(),
            b"config"
        );
        assert!(!root.join("metadata.json").exists());

        // A compatible runtime is a no-op, even without a usable URL or cache.
        fs::write(runtime.core_dir().join("keep.dll"), b"keep").unwrap();
        ensure_installed(&runtime, BinaryArch::X64, "unused", None, "test").unwrap();
        assert!(runtime.core_dir().join("keep.dll").is_file());
    }

    #[test]
    fn cached_install_rejects_wrong_architecture_and_incomplete_packages() {
        for (machine, tag) in [(Some(0x014c), "wrong-arch"), (None, "incomplete")] {
            let dir = TempDir::new(tag);
            let root = dir.0.join("profile");
            let runtime = BepInExRuntime::new(&root);
            let cache = cached_runtime(&dir.0, machine);
            let mut events = crate::backend::events::subscribe();
            let result = ensure_installed(&runtime, BinaryArch::X64, "unused", Some(&cache), tag);
            assert!(result.is_err());
            assert!(runtime.needs_install(BinaryArch::X64));
            let mut stages = Vec::new();
            while let Ok(event) = events.try_recv() {
                if let crate::backend::events::BackendEvent::BepInExProgress(progress) = event
                    && progress.target_id == tag
                {
                    stages.push(progress.stage);
                }
            }
            assert_eq!(stages.last().map(String::as_str), Some("failed"));
            assert!(!stages.iter().any(|stage| stage == "complete"));
        }
    }

    #[test]
    fn progress_is_emitted_once_per_whole_percent() {
        let mut last = Some(0);

        assert_eq!(changed_percent(11, 100, &mut last), Some(11));
        assert_eq!(changed_percent(119, 1000, &mut last), None);
        assert_eq!(changed_percent(120, 1000, &mut last), Some(12));
        assert_eq!(changed_percent(1_500, 1_000, &mut last), Some(100));
        assert_eq!(changed_percent(2_000, 1_000, &mut last), None);
    }

    #[test]
    fn progress_ignores_an_unknown_zero_total() {
        let mut last = None;

        assert_eq!(changed_percent(64, 0, &mut last), None);
        assert_eq!(last, None);
    }
}
