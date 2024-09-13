use std::path::PathBuf;

#[derive(Debug, clap::Parser)]
#[clap(name = "fetch-sfs",author,version,about,long_about=None)]
pub struct Args {
    /// Display verbose messages and progress information
    #[clap(long, short = 'v')]
    pub verbose: bool,

    /// Display structured `tracing` output in a tree-like structure.
    #[clap(long)]
    pub trace: bool,

    /// Turn off verbose message display for commands where these are shown by default.
    #[clap(long, conflicts_with("verbose"))]
    pub no_verbose: bool,

    /// The delay (in milliseconds) to wait betweeen downloads.
    #[clap(long, default_value = "500")]
    pub delay_ms: u64,

    /// The number of crawlers to use
    #[clap(long, short = 'c', default_value = "2")]
    pub crawling_concurrency: usize,

    /// The number of processors to use
    #[clap(long, short = 'p', default_value = "50")]
    pub processing_concurrency: usize,

    /// load and save state to this path
    #[clap(long, short = 's', default_value = "visited.json")]
    pub state: PathBuf,

    /// Path to save downloaded artefacts
    pub output: Option<PathBuf>,
}
