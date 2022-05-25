use structopt::StructOpt;
use std::path::{PathBuf, Path};
use futures::StreamExt;
use std::error::Error;
use log::{info, error, warn, debug};
use std::fs;
use std::process;
use std::str::FromStr;

extern crate pretty_env_logger;

mod api;
mod errors;

#[derive(Debug)]
enum PostGrouping {
    Pool,
    Rating,
    Artist,
    FileType,
    Tag(String),
}

impl PostGrouping {
    fn matches_post(&self, post: &api::Post) -> bool {
        match self {
            PostGrouping::Pool => !post.pools.is_empty(),
            PostGrouping::Rating | PostGrouping::FileType => true,
            PostGrouping::Artist => !post.tags.artist.is_empty(),
            PostGrouping::Tag(tag) => post.tags.contains(tag)
        }
    }

    fn post_group(&self, post: &api::Post) -> String {
        match self {
            PostGrouping::Pool => format!("pool_{}", post.pools.first().unwrap()),
            PostGrouping::Rating => post.rating.to_string(),
            PostGrouping::FileType => post.file.ext.to_string(),
            PostGrouping::Artist => post.tags.artist.first().unwrap().to_string(),
            PostGrouping::Tag(tag) => tag.to_string()
        }
    }
}

impl FromStr for PostGrouping {
    type Err = errors::ParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.starts_with("tag:") {
            let split: Vec<&str> = s.split(':').collect();
            return Ok(PostGrouping::Tag(split[1].to_string()));
        }

        match s {
            "pool" => Ok(PostGrouping::Pool),
            "rating" => Ok(PostGrouping::Rating),
            "artist" => Ok(PostGrouping::Artist),
            "filetype" => Ok(PostGrouping::FileType),
            _ => Err(errors::ParseError::new(&format!("invalid post grouping `{}`", s)))
        }
    }
}

#[derive(StructOpt, Debug)]
struct Cli {
    /// The tags to search for, space-separated. See: https://e621.net/help/cheatsheet
    tags: String,

    /// The maximum number of posts that should be retrieved per page.
    /// There is a hard limit of 320.
    #[structopt(short, long, default_value = "10")]
    limit: u32,

    /// The page that will be retrieved. Can also be used with "a" or "b" + `post_id`
    /// to get the posts after or before the specified post ID. For example, "a13"
    /// gets every post after post_id 13 up to the limit. This overrides any
    /// ordering meta-tag; `order:id_desc` is always used instead.
    ///
    /// If used with the `pages` option, only numeric page values are allowed.
    #[structopt(long, default_value = "1")]
    page: String,

    /// The maximum number of pages to download.
    ///
    /// Can be used in conjunction with the `page` option to start search at
    /// a specific page.
    #[structopt(short, long, default_value = "1")]
    pages: u32,

    /// The directory to write the downloaded posts to.
    #[structopt(short, long, default_value = "./out", parse(from_os_str))]
    out: PathBuf,

    /// Download posts from e926 instead of e621.
    #[structopt(short, long)]
    sfw: bool,

    /// Maximum number of concurrent downloads.
    #[structopt(short, long, default_value = "5")]
    concurrency: usize,

    /// Save downloaded posts grouped by the specified groupings. You can specify
    /// multiple groupings. See: https://github.com/tjhorner/e6dl-rs/wiki/Post-Grouping
    #[structopt(short, long)]
    group: Vec<PostGrouping>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    if std::env::var("E6DL_LOG").is_err() {
        std::env::set_var("E6DL_LOG", "info");
    }

    pretty_env_logger::init_custom_env("E6DL_LOG");

    let args = Cli::from_args();

    // Make sure `page` is numeric if `pages` > 1
    if args.pages > 1 && args.page.parse::<u32>().is_err() {
        error!("When providing the `pages` argument, the `page` argument must be numeric; before/after syntax is not supported.");
        process::exit(1);
    }

    info!("Searching for \"{}\"...", args.tags);

    let results = collect_posts(&args).await;

    match results {
        Ok(posts) => {
            if posts.is_empty() {
                warn!("No posts to download!");
                return Ok(());
            }

            let out_dir = args.out.as_path();
            info!("Found {} posts matching criteria, downloading to \"{}\"...", posts.len(), out_dir.to_str().unwrap());
            download_all(&posts, out_dir, &args.group, args.concurrency).await?;
        },
        Err(e) => error!("Could not search for posts: {}", e)
    }

    Ok(())
}

async fn collect_posts(args: &Cli) -> Result<Vec<api::Post>, Box<dyn std::error::Error>> {
    if args.pages == 1 {
        info!("Collecting posts from page {}...", args.page);
        return api::search(&args.tags, args.limit, &args.page, args.sfw).await;
    }

    let mut all_posts = Vec::new();

    // We did a check earlier (in main) to make sure this worked.
    let starting_page = args.page.parse::<u32>().expect("starting page was not numeric");
    let ending_page = starting_page + args.pages;

    info!("Collecting posts from up to {} pages, starting with page {}...", args.pages, starting_page);

    for page_num in starting_page..ending_page {
        debug!("Collecting posts from page {}...", page_num);
        let results = api::search(&args.tags, args.limit, &page_num.to_string(), args.sfw).await;

        match results {
            Ok(mut posts) => {
                if posts.is_empty() {
                    info!("No more posts on page {}; reached end of search results.", page_num);
                    break;
                }

                all_posts.append(&mut posts);
            },
            Err(e) => error!("Could not collect posts on page {}: {}", page_num, e)
        }
    }

    Ok(all_posts)
}

async fn download(post: &api::Post, to: &Path, grouping: &Vec<PostGrouping>) {
    let mut file_name = to.to_path_buf();

    if !grouping.is_empty() {
        for group in grouping {
            if !group.matches_post(post) { continue }
            file_name.push(group.post_group(post));
            if let Err(e) = fs::create_dir_all(&file_name) {
                error!("Couldn't create directory for post {}: {}", post.id, e);
            }
            break;
        }
    }

    file_name.push(format!("{}.{}", post.id, post.file.ext));

    info!("Downloading post {} -> {}...", post.id, file_name.to_str().unwrap());
    let result = api::download(post, &file_name).await;

    match result {
        Ok(_) => debug!("Done downloading post {}", post.id),
        Err(e) => error!("Error downloading post {}: {}", post.id, e)
    }
}

async fn download_all(posts: &Vec<api::Post>, to: &Path, grouping: &Vec<PostGrouping>, concurrency: usize) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(&to)?;

    futures::stream::iter(posts)
        .for_each_concurrent(concurrency, |post| download(post, to, grouping))
        .await;

    info!("Done!");

    Ok(())
}
