use std::path::PathBuf;

use add_metadata::add_metadata;
use clap::Parser;
use env_logger::Builder;

use tokio;
use wipe_metadata::wipe_metadata;

mod add_metadata;
mod utils;
mod wipe_metadata;

/// A tool to find and assign metadata to Geometry Dash music files
#[derive(Parser, Debug)]
#[clap(version, about, long_about = None)]
struct Args {
    /// The path to the GD music folder, including trailing slash
    #[clap(long, parse(from_os_str), default_value = "./")]
    path: PathBuf,

    /// The number of concurrent requests to make
    #[clap(long, default_value = "4")]
    parallel_requests: usize,

    /// Wipes metadata of provided songs. Takes priority over all other flags.
    #[clap(short)]
    wipe: bool,

    #[clap(flatten)]
    verbose: clap_verbosity_flag::Verbosity,
}

#[tokio::main]
async fn main() {
    // Arg parser - makes it a proper command line app
    let Args {
        path: base_path,
        parallel_requests,
        wipe,
        verbose,
    } = Args::parse();

    Builder::new()
        .filter_level(verbose.log_level_filter())
        .init();

    if wipe {
        wipe_metadata(base_path).unwrap();
        return;
    }

    add_metadata(base_path, parallel_requests).await.unwrap();
}
