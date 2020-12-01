use structopt::StructOpt;
use std::path::{PathBuf, Path};
use futures::StreamExt;
use std::error::Error;
use log::{info, error};
use std::fs;

extern crate pretty_env_logger;

mod api;

#[derive(StructOpt, Debug)]
struct Cli {
    /// The tags to search for, space-separated. See: https://e621.net/help/cheatsheet
    tags: String,

    /// The maximum number of posts that should be retrieved in the results.
    /// There is a hard limit of 320.
    #[structopt(short, long, default_value = "10")]
    limit: u32,

    /// The page that will be retrieved. Can also be used with "a" or "b" + `post_id`
    /// to get the posts after or before the specified post ID. For example, "a13"
    /// gets every post after post_id 13 up to the limit. This overrides any
    /// ordering meta-tag; `order:id_desc` is always used instead.
    #[structopt(short, long, default_value = "1")]
    page: String,

    /// The directory to write the downloaded posts to.
    #[structopt(short, long, default_value = "./out", parse(from_os_str))]
    out: PathBuf,

    /// Download posts from e926 instead of e621.
    #[structopt(short, long)]
    sfw: bool,

    /// Maximum number of concurrent downloads.
    #[structopt(short, long, default_value = "5")]
    concurrency: usize,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    if let Err(_) = std::env::var("E6DL_LOG") {
        std::env::set_var("E6DL_LOG", "info");
    }

    pretty_env_logger::init_custom_env("E6DL_LOG");

    let args: Cli = Cli::from_args();

    info!("Searching for \"{}\" (page {}, limit {})...", args.tags, args.page, args.limit);

    let results = api::search(&args.tags, &args.limit, &args.page).await;

    match results {
        Ok(posts) => {
            let out_dir = args.out.as_path();
            info!("Found {} posts matching criteria, downloading to \"{}\"...", posts.len(), out_dir.to_str().unwrap());
            download_all(posts, out_dir, args.concurrency).await?;
        },
        Err(e) => error!("Could not search for posts: {}", e)
    }

    Ok(())
}

async fn download_all(posts: Vec<api::Post>, to: &Path, concurrency: usize) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(&to)?;

    let downloads = posts.into_iter().map( |post| async move {
        let file_name = to.join(format!("{}.{}", post.id, post.file.ext));

        info!("Downloading post {} -> {}...", post.id, file_name.to_str().unwrap());
        let result = api::download(&post, &file_name).await;

        match result {
            Ok(_) => info!("Downloaded post {}!", post.id),
            Err(e) => error!("Error downloading post {}: {}", post.id, e)
        }
    });

    let fetches = futures::stream::iter(downloads)
        .buffer_unordered(concurrency)
        .collect::<Vec<_>>();

    fetches.await;

    Ok(())
}
