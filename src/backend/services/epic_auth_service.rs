//! Epic Games OAuth scaffolding.
//!
//! Currently only exposes the game-token exchange used by the modded launch
//! path. The interactive login flow (OAuth client credentials, session
//! persistence, refresh) was removed pending a redesign around a "open this
//! link, paste the code back" flow — re-add the client-id / -secret consts and
//! a `login_with_auth_code` helper here when implementing it.

use crate::backend::error::{AppError, AppResult};
use log::error;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};

const OAUTH_HOST: &str = "account-public-service-prod03.ol.epicgames.com";
const USER_AGENT: &str =
    "UELauncher/11.0.1-14907503+++Portal+Release-Live Windows/10.0.19041.1.256.64bit";

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

#[allow(dead_code)]
pub struct EpicAuthService {
    client: Client,
}

#[allow(dead_code)]
impl EpicAuthService {
    pub fn new() -> AppResult<Self> {
        let client = Client::builder()
            .user_agent(USER_AGENT)
            .build()
            .map_err(AppError::from)?;
        Ok(Self { client })
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
            error!("Failed to get game token ({}): {}", status, body);
            return Err(AppError::auth(format!(
                "Failed to get game token ({status}): {body}"
            )));
        }

        let token: GameTokenResponse = response
            .json()
            .map_err(|e| AppError::Http(e.to_string()))?;
        Ok(token.code)
    }
}
