# e6dl-rs

This is [e6dl](https://github.com/tjhorner/e6dl) ported to Rust, which was actually originally [ported from JS](https://github.com/tjhorner/e621-id-downloader). I am not Good At The Rust™️ yet, so I am using this project to get Good At The Rust™️. (i.e., the code is probably bad and not idiomatic).

## Usage

```
USAGE:
    e6dl [FLAGS] [OPTIONS] <tags>

FLAGS:
    -h, --help       Prints help information
    -s, --sfw        Download posts from e926 instead of e621
    -V, --version    Prints version information

OPTIONS:
    -c, --concurrency <concurrency>    Maximum number of concurrent downloads [default: 5]
    -l, --limit <limit>                The maximum number of posts that should be retrieved in the results. There is a
                                       hard limit of 320 [default: 10]
    -o, --out <out>                    The directory to write the downloaded posts to [default: ./out]
    -p, --page <page>                  The page that will be retrieved. Can also be used with "a" or "b" + `post_id` to
                                       get the posts after or before the specified post ID. For example, "a13" gets
                                       every post after post_id 13 up to the limit. This overrides any ordering meta-
                                       tag; `order:id_desc` is always used instead [default: 1]

ARGS:
    <tags>    The tags to search for, space-separated. See: https://e621.net/help/cheatsheet
```

## License

MIT