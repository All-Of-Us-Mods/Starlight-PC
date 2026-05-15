use crate::backend::error::{AppError, AppResult};
use base64::Engine;
use flate2::Compression;
use flate2::read::{GzDecoder, GzEncoder};
use keyring::Entry;
use log::{debug, error, info};
use serde::{Deserialize, Serialize};
use std::io::Read;

const B64: base64::engine::GeneralPurpose = base64::engine::general_purpose::STANDARD;

pub struct KeyringStorage<T> {
    service: &'static str,
    base_key: &'static str,
    _phantom: std::marker::PhantomData<T>,
}

impl<T: Serialize + for<'de> Deserialize<'de>> KeyringStorage<T> {
    pub const fn new(service: &'static str, base_key: &'static str) -> Self {
        Self {
            service,
            base_key,
            _phantom: std::marker::PhantomData,
        }
    }

    fn entry(&self, suffix: &str) -> AppResult<Entry> {
        Entry::new(self.service, &format!("{}_{}", self.base_key, suffix)).map_err(AppError::from)
    }

    pub fn save(&self, data: &T, chunk_size: usize) -> AppResult<()> {
        info!("Saving {} to keyring", self.base_key);
        self.clear()?;

        let json = serde_json::to_vec(data)?;
        let encoded = B64.encode(compress(&json)?);
        let chunks: Vec<_> = encoded.as_bytes().chunks(chunk_size).collect();

        self.entry("n")?
            .set_password(&chunks.len().to_string())
            .map_err(AppError::from)?;

        for (i, chunk) in chunks.iter().enumerate() {
            let chunk_str = std::str::from_utf8(chunk)
                .map_err(|e| AppError::parse(format!("Invalid UTF-8 chunk: {e}")))?;
            self.entry(&i.to_string())?
                .set_password(chunk_str)
                .map_err(AppError::from)?;
        }

        info!("{} saved ({} chunks)", self.base_key, chunks.len());
        Ok(())
    }

    pub fn load(&self) -> Option<T> {
        debug!("Loading {} from keyring", self.base_key);

        let count: usize = self.entry("n").ok()?.get_password().ok()?.parse().ok()?;
        let encoded: String = (0..count)
            .map(|i| self.entry(&i.to_string()).ok()?.get_password().ok())
            .collect::<Option<_>>()?;

        let decoded = B64.decode(&encoded).ok()?;
        let json = decompress(&decoded).ok()?;
        serde_json::from_slice(&json).ok()
    }

    pub fn clear(&self) -> AppResult<()> {
        let count: usize = self
            .entry("n")
            .and_then(|entry| {
                entry
                    .get_password()
                    .map_err(|e| AppError::state(format!("Failed to get count credential: {e}")))
            })
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);

        if count == 0 {
            return Ok(());
        }

        info!("Clearing {} ({} chunks)", self.base_key, count);
        if let Err(e) = self.entry("n").map(|entry| entry.delete_credential()) {
            error!("Failed to delete chunk count credential: {}", e);
        }
        for i in 0..count {
            if let Err(e) = self
                .entry(&i.to_string())
                .map(|entry| entry.delete_credential())
            {
                error!("Failed to delete credential '{}': {}", i, e);
            }
        }

        Ok(())
    }
}

fn compress(data: &[u8]) -> AppResult<Vec<u8>> {
    let mut encoder = GzEncoder::new(data, Compression::default());
    let mut out = Vec::new();
    encoder
        .read_to_end(&mut out)
        .map_err(|e| AppError::parse(format!("Compression failed: {e}")))?;
    Ok(out)
}

fn decompress(data: &[u8]) -> AppResult<Vec<u8>> {
    let mut decoder = GzDecoder::new(data);
    let mut out = Vec::new();
    decoder
        .read_to_end(&mut out)
        .map_err(|e| AppError::parse(format!("Decompression failed: {e}")))?;
    Ok(out)
}
