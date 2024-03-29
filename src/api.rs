use futures::StreamExt;
use serde::Deserialize;
use reqwest::{self, Client};
use std::path::Path;
use std::io::Write;
use std::fs::File;
use core::fmt;
use std::error::Error;
use log::debug;
use std::string::ToString;

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

impl PostTags {
    pub fn contains(&self, tag: &String) -> bool {
        self.general.contains(tag) ||
        self.species.contains(tag) ||
        self.character.contains(tag) ||
        self.copyright.contains(tag) ||
        self.artist.contains(tag) ||
        self.invalid.contains(tag) ||
        self.lore.contains(tag) ||
        self.meta.contains(tag)
    }
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

impl ToString for PostRating {
    fn to_string(&self) -> String {
        match self {
            PostRating::Safe => "safe",
            PostRating::Questionable => "questionable",
            PostRating::Explicit => "explicit"
        }.to_string()
    }
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
        .ok_or(ApiError::new("post has no downloadable file (a tag might be blacklisted)"))?;

    let mut file = File::create(to)?;

    let res = reqwest::get(url).await?;

    let mut download_stream = res.bytes_stream();
    while let Some(bytes) = download_stream.next().await {
        if let Err(e) = file.write_all(&bytes?) {
            return Err(Box::new(e));
        }
    }

    Ok(())
}

pub async fn search(tags: &str, limit: u32, page: &str, sfw: bool) -> Result<Vec<Post>, Box<dyn std::error::Error>> {
    let client = Client::new();

    debug!("Sending search request (tags = {}, limit = {}, page = {}, sfw = {})", tags, limit, page, sfw);

    let domain = if sfw { "e926.net" } else { "e621.net" };

    debug!("Using domain {}", domain);

    let res = client.get(&format!("https://{}/posts.json", domain))
        .header(reqwest::header::USER_AGENT, "e6dl: rust edition (@tjhorner on Telegram)")
        .query(&[
            ("tags", tags),
            ("page", page),
            ("limit", &limit.to_string()),
        ])
        .send()
        .await?;

    let pr = res.json::<PostsResponse>().await?;
    Ok(pr.posts)
}