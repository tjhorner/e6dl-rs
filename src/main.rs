use structopt::StructOpt;
use std::path::{PathBuf, Path};
use futures::StreamExt;
use std::error::Error;
use log::{info, error, warn, debug};
use std::fs;
use std::process;

extern crate pretty_env_logger;

mod api;

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
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    if let Err(_) = std::env::var("E6DL_LOG") {
        std::env::set_var("E6DL_LOG", "info");
    }

    pretty_env_logger::init_custom_env("E6DL_LOG");

    let args: Cli = Cli::from_args();

    // Make sure `page` is numeric if `pages` > 1
    if args.pages > 1 && args.page.parse::<u32>().is_err() {
        error!("When providing the `pages` argument, the `page` argument must be numeric; before/after syntax is not supported.");
        process::exit(1);
    }

    info!("Searching for \"{}\"...", args.tags);

    let results = collect_pages(&args).await;

    match results {
        Ok(posts) => {
            if posts.len() == 0 {
                warn!("No posts to download!");
                return Ok(());
            }

            let out_dir = args.out.as_path();
            info!("Found {} posts matching criteria, downloading to \"{}\"...", posts.len(), out_dir.to_str().unwrap());
            download_all(posts, out_dir, args.concurrency).await?;
        },
        Err(e) => error!("Could not search for posts: {}", e)
    }

    Ok(())
}

async fn collect_pages(args: &Cli) -> Result<Vec<api::Post>, Box<dyn std::error::Error>> {
    if args.pages == 1 {
        info!("Collecting posts from page {}...", args.page);
        return api::search(&args.tags, args.limit, &args.page, args.sfw).await;
    }

    let mut all_posts = Vec::<api::Post>::new();

    // We did a check earlier (in main) to make sure this worked.
    let starting_page = args.page.parse::<u32>().expect("starting page was not numerical");
    let ending_page = starting_page + args.pages;

    info!("Collecting posts from up to {} pages, starting with page {}...", args.pages, starting_page);

    for page_num in starting_page..ending_page {
        debug!("Collecting posts from page {}...", page_num);
        let results = api::search(&args.tags, args.limit, &page_num.to_string(), args.sfw).await;

        match results {
            Ok(mut posts) => {
                if posts.len() == 0 {
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

async fn download_all(posts: Vec<api::Post>, to: &Path, concurrency: usize) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(&to)?;

    let downloads = posts.into_iter().map( |post| async move {
        let file_name = to.join(format!("{}.{}", post.id, post.file.ext));

        info!("Downloading post {} -> {}...", post.id, file_name.to_str().unwrap());
        let result = api::download(&post, &file_name).await;

        match result {
            Ok(_) => debug!("Done downloading post {}", post.id),
            Err(e) => error!("Error downloading post {}: {}", post.id, e)
        }
    });

    let fetches = futures::stream::iter(downloads)
        .buffer_unordered(concurrency)
        .collect::<Vec<_>>();

    fetches.await;

    info!("Done!");

    Ok(())
}
