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
    pub status: Option<String>,
    pub id: String,
    pub name: String,
    pub description: String,
    pub long_description: Option<String>,
    pub author: String,
    pub mod_type: Option<String>,
    pub license: Option<String>,
    pub links: Option<Vec<ExternalLink>>,
    pub tags: Option<Vec<String>>,
    pub created_at: i64,
    pub updated_at: i64,
    pub downloads: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ExternalLink {
    #[serde(rename = "type")]
    pub link_type: String,
    pub url: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ModVersion {
    pub status: Option<String>,
    pub name: String,
    pub version: String,
    pub supported_platforms: Option<Vec<String>>,
    pub downloads: u64,
    pub created_at: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ModDependency {
    pub mod_id: String,
    pub name: String,
    pub version_constraint: String,
    #[serde(rename = "type")]
    pub dependency_type: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ModVersionInfo {
    pub status: Option<String>,
    pub name: String,
    pub version: String,
    pub supported_platforms: Option<Vec<String>>,
    pub downloads: u64,
    pub created_at: i64,
    pub changelog: Option<String>,
    pub dependencies: Vec<ModDependency>,
}

fn get_json<T: for<'de> Deserialize<'de>>(url: &str) -> AppResult<T> {
    reqwest::blocking::get(url)?
        .error_for_status()?
        .json::<T>()
        .map_err(|e| AppError::Http(e.to_string()))
}

pub fn fetch_news() -> AppResult<Vec<Post>> {
    get_json(&format!("{}/api/v3/news/posts", DEFAULT_API_BASE_URL))
}

pub fn fetch_trending_mods() -> AppResult<Vec<ModResponse>> {
    get_json(&format!("{}/api/v3/mods/trending", DEFAULT_API_BASE_URL))
}

pub fn fetch_mods_total() -> AppResult<u32> {
    get_json(&format!("{}/api/v3/mods/total", DEFAULT_API_BASE_URL))
}

pub fn fetch_mods(limit: u32, offset: u32) -> AppResult<Vec<ModResponse>> {
    get_json(&format!(
        "{}/api/v3/mods?limit={}&offset={}",
        DEFAULT_API_BASE_URL, limit, offset
    ))
}

pub fn fetch_mod(id: &str) -> AppResult<ModResponse> {
    get_json(&format!("{}/api/v3/mods/{}", DEFAULT_API_BASE_URL, id))
}

pub fn fetch_mod_versions(id: &str) -> AppResult<Vec<ModVersion>> {
    get_json(&format!(
        "{}/api/v3/mods/{}/versions",
        DEFAULT_API_BASE_URL, id
    ))
}

pub fn fetch_mod_version_info(id: &str, version: &str) -> AppResult<ModVersionInfo> {
    get_json(&format!(
        "{}/api/v3/mods/{}/versions/{}",
        DEFAULT_API_BASE_URL,
        id,
        urlencoding::encode(version)
    ))
}

pub fn mod_thumbnail_url(id: &str) -> String {
    format!("{}/api/v3/mods/{}/thumbnail", DEFAULT_API_BASE_URL, id)
}

pub fn search_mods(query: &str, limit: u32, offset: u32) -> AppResult<Vec<ModResponse>> {
    get_json(&format!(
        "{}/api/v3/mods/search?q={}&limit={}&offset={}",
        DEFAULT_API_BASE_URL,
        urlencoding::encode(query),
        limit,
        offset,
    ))
}
