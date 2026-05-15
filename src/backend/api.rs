use reqwest::Client;
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
    pub authors: Vec<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub icon_url: Option<String>,
    pub latest_version: String,
    pub download_count: u64,
    pub views: u64,
}

pub async fn fetch_news() -> Result<Vec<Post>, reqwest::Error> {
    let url = format!("{}/api/v3/news/posts", DEFAULT_API_BASE_URL);
    let client = Client::new();
    let response = client.get(&url).send().await?;
    response.json::<Vec<Post>>().await
}

pub async fn fetch_trending_mods() -> Result<Vec<ModResponse>, reqwest::Error> {
    let url = format!("{}/api/v3/mods/trending", DEFAULT_API_BASE_URL);
    let client = Client::new();
    let response = client.get(&url).send().await?;
    response.json::<Vec<ModResponse>>().await
}
