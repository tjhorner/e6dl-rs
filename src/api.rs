use serde::Deserialize;
use reqwest::{self, Client};
use std::path::Path;
use std::io;
use std::fs::File;
use core::fmt;
use std::error::Error;
use log::debug;

#[derive(Deserialize, Debug)]
pub struct PostFile {
    pub width: u32,
    pub height: u32,
    pub ext: String,
    pub size: u32,
    pub md5: String,
    pub url: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct PostPreview {
    pub width: u32,
    pub height: u32,
    pub url: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct PostSample {
    pub has: bool,
    pub width: u32,
    pub height: u32,
    pub url: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct PostScore {
    pub up: u32,
    pub down: i32,
    pub total: i32,
}

#[derive(Deserialize, Debug)]
pub struct PostTags {
    pub general: Vec<String>,
    pub species: Vec<String>,
    pub character: Vec<String>,
    pub copyright: Vec<String>,
    pub artist: Vec<String>,
    pub invalid: Vec<String>,
    pub lore: Vec<String>,
    pub meta: Vec<String>,
}

#[derive(Deserialize, Debug)]
pub struct PostFlags {
    pub pending: bool,
    pub flagged: bool,
    pub note_locked: bool,
    pub status_locked: bool,
    pub rating_locked: bool,
    pub deleted: bool,
}

#[derive(Deserialize, Debug)]
pub struct PostRelationships {
    pub parent_id: Option<u32>,
    pub has_children: bool,
    pub has_active_children: bool,
    pub children: Vec<u32>,
}

#[derive(Deserialize, Debug)]
pub struct Post {
    pub id: u32,
    pub description: String,
    pub created_at: String,
    pub updated_at: String,
    pub tags: PostTags,
    pub file: PostFile,
    pub preview: PostPreview,
    pub sample: PostSample,
    pub score: PostScore,
    pub locked_tags: Vec<String>,
    pub change_seq: u32,
    pub flags: PostFlags,
    pub rating: PostRating,
    pub fav_count: u32,
    pub sources: Vec<String>,
    pub pools: Vec<u32>,
    pub relationships: PostRelationships,
    pub approver_id: Option<u32>,
    pub uploader_id: u32,
    pub comment_count: u32,
    pub is_favorited: bool,
    pub has_notes: bool,
    pub duration: Option<f32>,
}

#[derive(Deserialize, Debug)]
pub enum PostRating {
    #[serde(rename = "s")]
    Safe,
    #[serde(rename = "q")]
    Questionable,
    #[serde(rename = "e")]
    Explicit
}

#[derive(Deserialize)]
struct PostsResponse {
    posts: Vec<Post>,
}

#[derive(Debug)]
struct ApiError {
    details: String
}

impl ApiError {
    fn new(msg: &str) -> ApiError {
        ApiError { details: msg.to_string() }
    }
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f,"{}",self.details)
    }
}

impl Error for ApiError {
    fn description(&self) -> &str {
        &self.details
    }
}

pub async fn download(post: &Post, to: &Path) -> Result<(), Box<dyn Error>> {
    let url = post.file.url.as_ref()
        .ok_or(ApiError::new("post has no downloadable file"))?;

    let mut file = File::create(to)?;

    let res = reqwest::get(url).await?;

    let bytes = res.bytes().await?;
    io::copy(&mut bytes.as_ref(), &mut file)?;

    Ok(())
}

pub async fn search(tags: &String, limit: &u32, page: &String, sfw: &bool) -> reqwest::Result<Vec<Post>> {
    let client = Client::new();

    debug!("Sending search request (tags = {}, limit = {}, page = {}, sfw = {})", tags, limit, page, sfw);

    let domain = if *sfw { "e926.net" } else { "e621.net" };

    debug!("Using domain {}", domain);

    let res = client.get(&format!("https://{}/posts.json", domain))
        .header(reqwest::header::USER_AGENT, "e6dl: rust edition (@tjhorner on Telegram)")
        .query(&[
            ("tags", tags),
            ("limit", &limit.to_string()),
            ("page", &page.to_string())
        ])
        .send()
        .await?;

    let pr = res.json::<PostsResponse>().await?;
    Ok(pr.posts)
}