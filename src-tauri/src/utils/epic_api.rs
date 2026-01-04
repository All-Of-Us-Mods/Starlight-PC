use base64::Engine;
use flate2::{Compression, read::GzDecoder, write::GzEncoder};
use keyring::Entry;
use log::{debug, error, info};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::io::{Read, Write};

const OAUTH_HOST: &str = "account-public-service-prod03.ol.epicgames.com";
const LAUNCHER_CLIENT_ID: &str = "34a02cf8f4414e29b15921876da36f9a";
const LAUNCHER_CLIENT_SECRET: &str = "daafbccc737745039dffe53d94fc76cf";
const USER_AGENT: &str =
    "UELauncher/11.0.1-14907503+++Portal+Release-Live Windows/10.0.19041.1.256.64bit";

const KEYRING_SERVICE: &str = "starlight";
const KEYRING_KEY: &str = "epic_session";
const CHUNK_SIZE: usize = 1200; // UTF-16 doubles size, must stay under 2560/2

const B64: base64::engine::GeneralPurpose = base64::engine::general_purpose::STANDARD;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EpicSession {
    pub access_token: String,
    pub refresh_token: String,
    pub account_id: String,
}

#[derive(Debug, Deserialize)]
struct GameTokenResponse {
    code: String,
}

pub struct EpicApi {
    client: Client,
}

impl EpicApi {
    pub fn new() -> Result<Self, String> {
        Client::builder()
            .user_agent(USER_AGENT)
            .gzip(true)
            .build()
            .map(|client| Self { client })
            .map_err(|e| {
                error!("Failed to create HTTP client: {}", e);
                format!("Failed to create HTTP client: {e}")
            })
    }

    fn get_basic_auth() -> String {
        B64.encode(format!("{LAUNCHER_CLIENT_ID}:{LAUNCHER_CLIENT_SECRET}"))
    }

    pub fn get_auth_url() -> String {
        let redirect = format!(
            "https://www.epicgames.com/id/api/redirect?clientId={LAUNCHER_CLIENT_ID}&responseType=code"
        );
        format!(
            "https://www.epicgames.com/id/login?redirectUrl={}",
            urlencoding::encode(&redirect)
        )
    }

    pub async fn login_with_auth_code(&self, code: &str) -> Result<EpicSession, String> {
        info!("Logging in with Epic authorization code");
        self.oauth_request(&[
            ("grant_type", "authorization_code"),
            ("code", code),
            ("token_type", "eg1"),
        ])
        .await
    }

    pub async fn refresh_session(&self, refresh_token: &str) -> Result<EpicSession, String> {
        debug!("Refreshing Epic session");
        self.oauth_request(&[
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
            ("token_type", "eg1"),
        ])
        .await
    }

    async fn oauth_request(&self, params: &[(&str, &str)]) -> Result<EpicSession, String> {
        let response = self
            .client
            .post(format!("https://{OAUTH_HOST}/account/api/oauth/token"))
            .header("Authorization", format!("Basic {}", Self::get_basic_auth()))
            .form(params)
            .send()
            .await
            .map_err(|e| {
                error!("Epic OAuth request failed: {}", e);
                format!("Request failed: {e}")
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            error!("Epic OAuth request failed ({}): {}", status, body);
            return Err(format!("OAuth request failed ({status}): {body}"));
        }

        response
            .json()
            .await
            .map_err(|e| {
                error!("Failed to parse Epic OAuth response: {}", e);
                format!("Failed to parse response: {e}")
            })
    }

    pub async fn get_game_token(&self, session: &EpicSession) -> Result<String, String> {
        debug!("Requesting Epic game token");
        let response = self
            .client
            .get(format!("https://{OAUTH_HOST}/account/api/oauth/exchange"))
            .header("Authorization", format!("Bearer {}", session.access_token))
            .send()
            .await
            .map_err(|e| {
                error!("Epic game token request failed: {}", e);
                format!("Request failed: {e}")
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            error!("Failed to get Epic game token ({}): {}", status, body);
            return Err(format!("Failed to get game token ({status}): {body}"));
        }

        response
            .json::<GameTokenResponse>()
            .await
            .map(|t| {
                debug!("Epic game token obtained");
                t.code
            })
            .map_err(|e| {
                error!("Failed to parse Epic game token: {}", e);
                format!("Failed to parse token: {e}")
            })
    }
}

// --- Keyring storage with chunking for Windows credential size limits ---

fn keyring_entry(suffix: &str) -> Result<Entry, String> {
    Entry::new(KEYRING_SERVICE, &format!("{KEYRING_KEY}_{suffix}"))
        .map_err(|e| {
            error!("Keyring access failed: {}", e);
            format!("Keyring access failed: {e}")
        })
}

fn compress(data: &[u8]) -> Result<Vec<u8>, String> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::best());
    encoder
        .write_all(data)
        .and_then(|_| encoder.finish())
        .map_err(|e| {
            error!("Compression failed: {}", e);
            format!("Compression failed: {e}")
        })
}

fn decompress(data: &[u8]) -> Result<Vec<u8>, String> {
    let mut decoder = GzDecoder::new(data);
    let mut out = Vec::new();
    decoder
        .read_to_end(&mut out)
        .map_err(|e| {
            error!("Decompression failed: {}", e);
            format!("Decompression failed: {e}")
        })?;
    Ok(out)
}

pub fn save_session(session: &EpicSession) -> Result<(), String> {
    info!("Saving Epic session to keyring");
    clear_session()?;

    // Serialize -> compress -> base64 encode
    let json = serde_json::to_vec(session).map_err(|e| {
        error!("Failed to serialize session: {}", e);
        format!("Serialize failed: {e}")
    })?;
    let encoded = B64.encode(compress(&json)?);

    // Store chunks (base64 is ASCII, safe to split at any byte boundary)
    let chunks: Vec<_> = encoded.as_bytes().chunks(CHUNK_SIZE).collect();

    keyring_entry("n")?
        .set_password(&chunks.len().to_string())
        .map_err(|e| {
            error!("Failed to save chunk count: {}", e);
            format!("Failed to save chunk count: {e}")
        })?;

    for (i, chunk) in chunks.iter().enumerate() {
        let chunk_str = std::str::from_utf8(chunk).unwrap(); // Safe: base64 is ASCII
        keyring_entry(&i.to_string())?
            .set_password(chunk_str)
            .map_err(|e| {
                error!("Failed to save chunk {}: {}", i, e);
                format!("Failed to save chunk {i}: {e}")
            })?;
    }

    info!("Epic session saved ({} chunks)", chunks.len());
    Ok(())
}

pub fn load_session() -> Option<EpicSession> {
    debug!("Loading Epic session from keyring");

    let count: usize = keyring_entry("n").ok()?.get_password().ok()?.parse().ok()?;

    let encoded: String = (0..count)
        .map(|i| keyring_entry(&i.to_string()).ok()?.get_password().ok())
        .collect::<Option<_>>()?;

    let json = decompress(&B64.decode(&encoded).ok()?).ok()?;
    let session = serde_json::from_slice(&json).ok()?;

    debug!("Epic session loaded ({count} chunks)");
    Some(session)
}

pub fn clear_session() -> Result<(), String> {
    let count: usize = keyring_entry("n")
        .and_then(|e| e.get_password().map_err(|e| e.to_string()))
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);

    if count == 0 {
        return Ok(());
    }

    info!("Clearing Epic session ({count} chunks)");
    let _ = keyring_entry("n").map(|e| e.delete_credential());
    for i in 0..count {
        let _ = keyring_entry(&i.to_string()).map(|e| e.delete_credential());
    }

    Ok(())
}
