//! Epic Games account login: a manual "open this link, paste the code back"
//! flow instead of an embedded webview (gpui doesn't have one). The user opens
//! [`EpicAuthService::auth_url`] in their system browser, logs in, and pastes
//! the `authorizationCode` shown on the redirect page back into the app.

use crate::backend::error::{AppError, AppResult};
use crate::backend::services::storage_service::KeyringStorage;
use base64::Engine as _;
use log::{debug, error, info};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

const OAUTH_HOST: &str = "account-public-service-prod03.ol.epicgames.com";
const LAUNCHER_CLIENT_ID: &str = "34a02cf8f4414e29b15921876da36f9a";
const LAUNCHER_CLIENT_SECRET: &str = "daafbccc737745039dffe53d94fc76cf";
const USER_AGENT: &str =
    "UELauncher/11.0.1-14907503+++Portal+Release-Live Windows/10.0.19041.1.256.64bit";
const CHUNK_SIZE: usize = 1200;
const B64: base64::engine::GeneralPurpose = base64::engine::general_purpose::STANDARD;

static STORAGE: OnceLock<KeyringStorage<EpicSession>> = OnceLock::new();

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

pub struct EpicAuthService {
    client: Client,
}

impl EpicAuthService {
    pub fn new() -> AppResult<Self> {
        let client = Client::builder()
            .user_agent(USER_AGENT)
            .build()
            .map_err(AppError::from)?;
        Ok(Self { client })
    }

    fn basic_auth() -> String {
        B64.encode(format!("{LAUNCHER_CLIENT_ID}:{LAUNCHER_CLIENT_SECRET}"))
    }

    /// Page to open in the system browser. After login it redirects to a page
    /// showing a JSON blob with an `authorizationCode` field to paste back.
    pub fn auth_url() -> String {
        let redirect = format!(
            "https://www.epicgames.com/id/api/redirect?clientId={LAUNCHER_CLIENT_ID}&responseType=code"
        );
        format!(
            "https://www.epicgames.com/id/login?redirectUrl={}",
            urlencoding::encode(&redirect)
        )
    }

    pub fn login_with_auth_code(&self, code: &str) -> AppResult<EpicSession> {
        info!("Logging in with Epic authorization code");
        self.oauth_request(&[
            ("grant_type", "authorization_code"),
            ("code", code),
            ("token_type", "eg1"),
        ])
    }

    pub fn refresh_session(&self, refresh_token: &str) -> AppResult<EpicSession> {
        debug!("Refreshing Epic session");
        self.oauth_request(&[
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
            ("token_type", "eg1"),
        ])
    }

    fn oauth_request(&self, params: &[(&str, &str)]) -> AppResult<EpicSession> {
        let response = self
            .client
            .post(format!("https://{OAUTH_HOST}/account/api/oauth/token"))
            .header("Authorization", format!("Basic {}", Self::basic_auth()))
            .form(params)
            .send()?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            error!("Epic OAuth request failed ({status}): {body}");
            return Err(AppError::auth(format!(
                "OAuth request failed ({status}): {body}"
            )));
        }

        response.json().map_err(AppError::from)
    }

    pub fn get_game_token(&self, session: &EpicSession) -> AppResult<String> {
        let response = self
            .client
            .get(format!("https://{OAUTH_HOST}/account/api/oauth/exchange"))
            .header("Authorization", format!("Bearer {}", session.access_token))
            .send()?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().unwrap_or_default();
            error!("Failed to get game token ({status}): {body}");
            return Err(AppError::auth(format!(
                "Failed to get game token ({status}): {body}"
            )));
        }

        let token: GameTokenResponse = response.json()?;
        Ok(token.code)
    }
}

fn storage() -> &'static KeyringStorage<EpicSession> {
    STORAGE.get_or_init(|| KeyringStorage::new("starlight", "epic_session"))
}

pub fn save_session(session: &EpicSession) -> AppResult<()> {
    storage().save(session, CHUNK_SIZE)
}

pub fn load_session() -> Option<EpicSession> {
    storage().load()
}

pub fn clear_session() -> AppResult<()> {
    storage().clear()
}
