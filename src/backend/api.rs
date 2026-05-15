use crate::backend::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};

pub const DEFAULT_API_BASE_URL: &str = "https://starlight.allofus.dev";

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Post {
    pub id: u32,
    pub title: String,
    pub author: String,
    pub content: String,
    pub tags: Option<Vec<String>>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ModResponse {
    pub id: String,
    pub name: String,
    pub description: String,
    pub author: String,
    pub mod_type: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub downloads: u64,
}

pub fn fetch_news() -> AppResult<Vec<Post>> {
    let url = format!("{}/api/v3/news/posts", DEFAULT_API_BASE_URL);
    ureq::get(&url)
        .call()?
        .into_json::<Vec<Post>>()
        .map_err(|e| AppError::Http(e.to_string()))
}

pub fn fetch_trending_mods() -> AppResult<Vec<ModResponse>> {
    let url = format!("{}/api/v3/mods/trending", DEFAULT_API_BASE_URL);
    ureq::get(&url)
        .call()?
        .into_json::<Vec<ModResponse>>()
        .map_err(|e| AppError::Http(e.to_string()))
}

pub fn fetch_mods(limit: u32, offset: u32) -> AppResult<Vec<ModResponse>> {
    let url = format!(
        "{}/api/v3/mods?limit={}&offset={}",
        DEFAULT_API_BASE_URL, limit, offset
    );
    ureq::get(&url)
        .call()?
        .into_json::<Vec<ModResponse>>()
        .map_err(|e| AppError::Http(e.to_string()))
}

pub fn fetch_mod(id: &str) -> AppResult<ModResponse> {
    let url = format!("{}/api/v3/mods/{}", DEFAULT_API_BASE_URL, id);
    ureq::get(&url)
        .call()?
        .into_json::<ModResponse>()
        .map_err(|e| AppError::Http(e.to_string()))
}

pub fn mod_thumbnail_url(id: &str) -> String {
    format!("{}/api/v3/mods/{}/thumbnail", DEFAULT_API_BASE_URL, id)
}

pub fn search_mods(query: &str, limit: u32, offset: u32) -> AppResult<Vec<ModResponse>> {
    let url = format!(
        "{}/api/v3/mods/search?q={}&limit={}&offset={}",
        DEFAULT_API_BASE_URL,
        urlencoding::encode(query),
        limit,
        offset,
    );
    ureq::get(&url)
        .call()?
        .into_json::<Vec<ModResponse>>()
        .map_err(|e| AppError::Http(e.to_string()))
}
