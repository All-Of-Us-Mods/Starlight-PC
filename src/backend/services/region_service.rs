//! Reads and writes Among Us' `regionInfo.json` so community servers from the
//! Starlight servers API can be added as selectable in-game regions.
//!
//! Region entries are kept as raw JSON values so region kinds we don't model
//! (e.g. DNS regions) survive a read/write round-trip untouched. `$type` sorts
//! first in serde_json's (BTreeMap-backed) object output, which is what Among
//! Us' polymorphic deserializer expects.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::backend::api::Server;
use crate::backend::error::{AppError, AppResult};

const STATIC_HTTP_TYPE: &str = "StaticHttpRegionInfo, Assembly-CSharp";

/// Among Us' region list (`regionInfo.json`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionInfo {
    #[serde(rename = "CurrentRegionIdx")]
    pub current_region_idx: i32,
    #[serde(rename = "Regions")]
    pub regions: Vec<Value>,
}

/// Display name of a region entry (falls back to `"Unknown"`).
pub fn region_name(region: &Value) -> &str {
    region
        .get("Name")
        .and_then(Value::as_str)
        .unwrap_or("Unknown")
}

/// Whether `region` already targets `address`:`port`. Matched on host + port
/// (ignoring the URL scheme) so we can tell an API server is installed
/// regardless of its display name.
pub fn region_has_server(region: &Value, address: &str, port: u16) -> bool {
    let ping_matches = region
        .get("PingServer")
        .and_then(Value::as_str)
        .is_some_and(|p| host_of(p).eq_ignore_ascii_case(address));
    region
        .get("Servers")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .any(|s| {
            let port_matches =
                s.get("Port").and_then(Value::as_u64).unwrap_or(0) == u64::from(port);
            let host_matches = s
                .get("Ip")
                .and_then(Value::as_str)
                .is_some_and(|ip| host_of(ip).eq_ignore_ascii_case(address));
            port_matches && (host_matches || ping_matches)
        })
}

/// The host portion of a server URL — strips an `http(s)://` scheme and any path.
fn host_of(url: &str) -> &str {
    let without_scheme = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .unwrap_or(url);
    without_scheme.split('/').next().unwrap_or(without_scheme)
}

#[cfg(windows)]
fn region_info_path() -> AppResult<PathBuf> {
    // %APPDATA% is the Roaming folder; regionInfo.json lives one level up under
    // LocalLow (…/AppData/LocalLow/Innersloth/Among Us/regionInfo.json).
    let roaming = std::env::var_os("APPDATA")
        .ok_or_else(|| AppError::state("APPDATA environment variable is not set"))?;
    let app_data = PathBuf::from(roaming)
        .parent()
        .ok_or_else(|| AppError::state("Could not resolve the LocalLow directory"))?
        .to_path_buf();
    Ok(app_data
        .join("LocalLow")
        .join("Innersloth")
        .join("Among Us")
        .join("regionInfo.json"))
}

#[cfg(not(windows))]
fn region_info_path() -> AppResult<PathBuf> {
    Err(AppError::platform(
        "Configuring Among Us regions is only supported on Windows",
    ))
}

/// Read the current region file. A missing file yields an empty region list to
/// append to; a malformed file is reported as an error rather than being
/// silently overwritten.
pub fn read_region_info() -> AppResult<RegionInfo> {
    let path = region_info_path()?;
    match std::fs::read_to_string(&path) {
        Ok(raw) => Ok(serde_json::from_str(&raw)?),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(empty_region_info()),
        Err(e) => Err(e.into()),
    }
}

fn write_region_info(info: &RegionInfo) -> AppResult<()> {
    let path = region_info_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, serde_json::to_vec(info)?)?;
    Ok(())
}

/// Add a server from the API as a region. No-op (returns `false`) if a region
/// already targets the same host:port.
pub fn add_server_region(server: &Server) -> AppResult<bool> {
    let mut info = read_region_info()?;
    if info
        .regions
        .iter()
        .any(|r| region_has_server(r, &server.address, server.port))
    {
        return Ok(false);
    }
    info.regions.push(server_region(server));
    write_region_info(&info)?;
    Ok(true)
}

/// Remove the region with `name`.
pub fn remove_region(name: &str) -> AppResult<()> {
    let mut info = read_region_info()?;
    info.regions.retain(|r| region_name(r) != name);
    // Keep Among Us' in-game selection index within bounds after the removal.
    let max = info.regions.len().saturating_sub(1) as i32;
    info.current_region_idx = info.current_region_idx.clamp(0, max);
    write_region_info(&info)?;
    Ok(())
}

fn server_region(server: &Server) -> Value {
    let scheme = if server.port == 443 { "https" } else { "http" };
    let ip = format!("{scheme}://{}", server.address);
    json!({
        "$type": STATIC_HTTP_TYPE,
        "Name": server.name,
        "PingServer": server.address,
        "Servers": [{
            "Name": "Http-1",
            "Ip": ip,
            "Port": server.port,
            "UseDtls": server.dtls,
            "Players": 0,
            "ConnectionFailures": 0,
        }],
        "TargetServer": null,
        "TranslateName": server.translate_name,
    })
}

fn empty_region_info() -> RegionInfo {
    RegionInfo {
        current_region_idx: 0,
        regions: Vec::new(),
    }
}
