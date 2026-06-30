//! Process-wide cache of Starlight catalog mod lookups (`api::fetch_mod`),
//! shared by every view that needs to resolve a mod id to catalog info — a
//! mod looked up once (e.g. opening a profile in the Library) is reused by
//! any other view that needs it (e.g. browsing lobbies) instead of
//! re-fetching.

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

use log::warn;

use crate::backend::api::{self, ModLookup, ModResponse};

/// `None` means the catalog confirmed (HTTP 404) it has no matching mod —
/// cached too, so callers don't retry forever. A transient failure (network
/// error, timeout, 5xx) is never cached here, so it's retried on the next
/// lookup instead of permanently mislabeling the mod as missing.
static CACHE: LazyLock<Mutex<HashMap<String, Option<ModResponse>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// The cached result for `mod_id`. Outer `None` means it hasn't been looked
/// up this session; `Some(None)` means it was looked up and not found.
pub fn get(mod_id: &str) -> Option<Option<ModResponse>> {
    CACHE.lock().ok()?.get(mod_id).cloned()
}

/// Display names for every catalog mod resolved so far this session.
pub fn cached_names() -> HashMap<String, String> {
    CACHE
        .lock()
        .map(|cache| {
            cache
                .iter()
                .filter_map(|(id, info)| info.as_ref().map(|m| (id.clone(), m.name.clone())))
                .collect()
        })
        .unwrap_or_default()
}

/// Resolve `mod_id` against the Starlight catalog, using (and populating)
/// the shared cache. Blocking — does a network request on a cache miss, so
/// call from the background executor, never from `render`.
pub fn fetch(mod_id: &str) -> Option<ModResponse> {
    if let Some(cached) = get(mod_id) {
        return cached;
    }
    match api::lookup_mod(mod_id) {
        Ok(ModLookup::Found(info)) => {
            let info = *info;
            if let Ok(mut cache) = CACHE.lock() {
                cache.insert(mod_id.to_string(), Some(info.clone()));
            }
            Some(info)
        }
        Ok(ModLookup::NotFound) => {
            if let Ok(mut cache) = CACHE.lock() {
                cache.insert(mod_id.to_string(), None);
            }
            None
        }
        Err(e) => {
            // Transient failure — don't cache it, so the next lookup retries
            // instead of permanently treating this mod as missing.
            warn!("mod catalog lookup failed for {mod_id}: {e}");
            None
        }
    }
}
