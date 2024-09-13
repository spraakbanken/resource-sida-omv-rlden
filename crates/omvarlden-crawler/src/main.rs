use std::{path::PathBuf, sync::Arc, time::Duration};

use clap::Parser;
use tracing_subscriber::fmt::format::FmtSpan;
use webcrawler::{Crawler, CrawlerOptions};

use crate::options::Args;

mod options;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let delay_ms = args.delay_ms;
    let crawling_concurrency = args.crawling_concurrency;
    let processing_concurrency = args.processing_concurrency;
    let state_path = args.state;
    let output = args.output;

    init_tracing()?;

    let crawler = Crawler::new(
        Some(state_path),
        CrawlerOptions {
            delay: Duration::from_millis(delay_ms),
            crawling_concurrency,
            processing_concurrency,
        },
    );

    let spider = Arc::new(omvarlden_spider::omvarlden::OmvarldenSpider::new(
        omvarlden_spider::omvarlden::OmvarldenSpiderOptions {
            user_agent: Some(APP_USER_AGENT.into()),
            output_path: output.unwrap_or_else(|| PathBuf::from("./output")),
        },
    )?);
    crawler.run(spider).await;

    Ok(())
}

/// construct a subscriber that prints formatted traces to stdout
fn init_tracing() -> anyhow::Result<()> {
    use std::io;
    use tracing_subscriber::EnvFilter;

    let subscriber = tracing_subscriber::fmt()
        .json()
        .with_span_events(FmtSpan::NEW | FmtSpan::EXIT)
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .or_else(|_| EnvFilter::try_new("fetch_sfs=info,info"))
                .expect("telemetry: Creating EnvFilter"),
        )
        .with_writer(io::stderr)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;
    Ok(())
}

// == Client ==
// Name your user agent after your app?
pub static APP_USER_AGENT: &str = concat!(
    "Mozilla/5.0",
    " ",
    "(X11; Linux x86_64; rv:130.0)",
    " ",
    "Gecko/20100101",
    " ",
    "Firefox/130.0",
);
