use std::{fmt, fs, io::Write, path::PathBuf};

use async_trait::async_trait;
use flate2::Compression;
use reqwest::{Client, Url};
use scraper::{Html, Selector};

use crate::item::Item;
use crate::Error;

pub struct OmvarldenSpider {
    http_client: Client,
    output_path: PathBuf,
}

impl fmt::Debug for OmvarldenSpider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "OmvarldenSpider {{ /* omitted */ }}")
    }
}

#[derive(Debug, Clone)]
pub struct OmvarldenSpiderOptions {
    pub user_agent: Option<String>,
    pub output_path: PathBuf,
}

impl Default for OmvarldenSpiderOptions {
    fn default() -> Self {
        Self {
            user_agent: None,
            output_path: "./output".into(),
        }
    }
}

impl OmvarldenSpider {
    const BASE_URL: &'static str = "https://www.omvarlden.se";
    const PODD_META_URL: &'static str = "https://utvecklingssamtalet.libsyn.com";
    const PODD_URL: &'static str = "https://traffic.libsyn.com";

    pub fn new(
        OmvarldenSpiderOptions {
            user_agent,
            output_path,
        }: OmvarldenSpiderOptions,
    ) -> Result<Self, Error> {
        tracing::info!("creating {}, if not exists", output_path.display());
        fs::create_dir_all(&output_path).map_err(|error| Error::CantCreateDir {
            path: output_path.clone(),
            error,
        })?;
        let output_path =
            output_path
                .canonicalize()
                .map_err(|error| Error::CantCanonicalizePath {
                    path: output_path,
                    error,
                })?;
        let user_agent = user_agent.as_deref().unwrap_or(crate::APP_USER_AGENT);
        tracing::warn!(user_agent, "configuring SfsSpider {:?}", output_path);
        let http_client = reqwest::Client::builder()
            .user_agent(user_agent)
            .gzip(true)
            .build()
            .map_err(|err| Error::CantCreateHttpClient(err))?;
        Ok(Self {
            http_client,
            output_path,
        })
    }
}
#[async_trait]
impl webcrawler::Spider for OmvarldenSpider {
    type Item = Item;
    type Error = Error;

    fn name(&self) -> String {
        "omvarlden".into()
    }

    fn start_urls(&self) -> Vec<String> {
        vec![
            Self::BASE_URL.to_string(),
            "https://utvecklingssamtalet.libsyn.com/page/1/size/200".to_string(),
        ]
    }

    #[tracing::instrument]
    async fn scrape(&self, url: String) -> Result<(Vec<Self::Item>, Vec<String>), Self::Error> {
        let mut new_urls = Vec::new();
        let mut items = Vec::new();

        tracing::info!("calling {}", url);
        let response = self.http_client.get(&url).send().await.map_err(|error| {
            tracing::error!("Failed fetching: {:?}", error);
            Error::ScrapeError {
                url: url.clone(),
                error,
            }
        })?;

        tracing::trace!("response status: {}", response.status());
        if !response.status().is_success() {
            let status_code = response.status();
            tracing::error!(
                "The request returned '{}': '{}",
                response.status(),
                response.text().await.map_err(|error| Error::ScrapeError {
                    url: url.clone(),
                    error
                })?
            );
            return Err(Error::RequestReturnedError { url, status_code });
        }

        if url.starts_with(Self::BASE_URL) {
            let text = response.text().await.map_err(|error| {
                tracing::error!("Failed getting text: {}", error);
                Error::FailedToGetData {
                    url: url.clone(),
                    error,
                }
            })?;
            let document = Html::parse_document(&text);
            let a_selector = Selector::parse("a").unwrap();
            for link in document.select(&a_selector) {
                let href = link.attr("href");
                if let Some(href) = href {
                    if href.starts_with("/") {
                        new_urls.push(format!("{}{}", Self::BASE_URL, href));
                    }
                }
            }
            tracing::debug!(new_urls = ?new_urls);
            items.push(Item::Html(text));
        } else if url.starts_with(Self::PODD_META_URL) {
            let text = response.text().await.map_err(|error| {
                tracing::error!("Failed getting text: {}", error);
                Error::FailedToGetData {
                    url: url.clone(),
                    error,
                }
            })?;
            let document = Html::parse_document(&text);
            let a_selector = Selector::parse("a").unwrap();
            let item_selector = Selector::parse(r#"div[class="libsyn-item"]"#).unwrap();
            let title_selector = Selector::parse(r#"div[class="libsyn-item-title"]"#).unwrap();
            let release_date_selector =
                Selector::parse(r#"div[class="libsyn-item-release-date"]"#).unwrap();
            let body_selector = Selector::parse(r#"div[class="libsyn-item-body"]"#).unwrap();
            let content_selector = Selector::parse(r#"div[class="libsyn-item-content"]"#).unwrap();
            for item in document.select(&item_selector) {
                let title = item.select(&title_selector).next().unwrap();
                let a = title.select(&a_selector).next().unwrap();
                let title: String = a.text().collect();
                let meta_url = if let Some(href) = a.attr("href") {
                    href.to_string()
                } else {
                    String::new()
                };
                let release_date_div = item.select(&release_date_selector).next().unwrap();
                let release_date: String = release_date_div.text().collect();
                let description = if let Some(body_div) = item.select(&body_selector).next() {
                    Some(body_div.text().collect::<String>())
                } else {
                    None
                };
                let content_div = item.select(&content_selector).next().unwrap();
                let a = content_div.select(&a_selector).next().unwrap();
                let mp3_url = if let Some(href) = a.attr("href") {
                    href.to_string()
                } else {
                    String::new()
                };
                dbg!(&mp3_url);
                new_urls.push(mp3_url.clone());
                items.push(Item::PoddMeta {
                    meta_url,
                    title,
                    release_date,
                    description,
                    mp3_url,
                });
            }
        } else if url.starts_with(Self::PODD_URL) {
            tracing::debug!("downloading mp3 from '{}'", url);
            let mp3_data = response.bytes().await.map_err(|error| {
                tracing::error!("Failed getting bytes: {}", error);
                Error::FailedToGetData {
                    url: url.clone(),
                    error,
                }
            })?;
            items.push(Item::Podd(mp3_data.to_vec()));
        } else {
            tracing::error!("unknown url '{}'", url);
        }
        Ok((items, new_urls))
    }

    #[tracing::instrument(skip(item))]
    async fn process(&self, url: String, item: Self::Item) -> Result<String, Error> {
        let mut path = self.output_path.clone();
        tracing::debug!(path = ?path);
        tracing::info!(url, "analyzing url");
        let url = Url::parse(&url).unwrap();
        match item {
            Item::Html(item) => {
                let file_name = match url.path() {
                    "/" => "index.html",
                    x => {
                        let parts: Vec<&str> = x.split('/').collect();
                        let mut result = parts[0];
                        let parts_len = parts.len();
                        for (i, part) in parts.into_iter().enumerate() {
                            if i == parts_len - 1 {
                                result = part;
                            } else {
                                if !part.is_empty() {
                                    path.push(part);
                                }
                            }
                        }
                        result
                    }
                };
                tracing::debug!(file_name = ?file_name);
                tokio::fs::create_dir_all(&path).await.map_err(|error| {
                    tracing::error!(
                        "failed creating path='{}', url={}, error={}",
                        path.display(),
                        url,
                        error
                    );
                    Error::CantCreateDir {
                        path: path.clone(),
                        error,
                    }
                })?;
                if [
                    "nyheter",
                    "globala-malen",
                    "opinion",
                    "reportage",
                    "intervjuer",
                    "poddar",
                    "teman",
                ]
                .contains(&file_name)
                {
                    Ok(String::new())
                } else {
                    let mut path = path.join(file_name);
                    path.set_extension("html.gz");
                    tracing::debug!( path = ?path, "final path");
                    let file = std::fs::File::create(&path).map_err(|error| {
                        tracing::error!(
                            "failed creating '{}', url={}, error={}",
                            path.display(),
                            url,
                            error
                        );
                        Error::CantCreateDir {
                            path: path.clone(),
                            error,
                        }
                    })?;
                    let compress_writer = flate2::write::GzEncoder::new(file, Compression::best());
                    let mut writer = std::io::BufWriter::new(compress_writer);
                    tracing::info!("writing html");
                    writer.write_all(item.as_bytes()).map_err(|error| {
                        tracing::error!("failed writing '{}', url={}", path.display(), url);
                        Error::FailedWritingFile {
                            path: path.clone(),
                            error,
                        }
                    })?;
                    Ok(path.display().to_string())
                }
            }
            Item::Podd(podd_mp3) => {
                let path = path.join(&url.path()[1..]);
                dbg!(&path);
                if let Some(parent) = path.parent() {
                    tokio::fs::create_dir_all(parent).await.map_err(|error| {
                        tracing::error!(
                            "failed creating path='{}', url={}, error={}",
                            parent.display(),
                            url,
                            error
                        );
                        Error::CantCreateDir {
                            path: parent.to_path_buf(),
                            error,
                        }
                    })?;
                }
                let mut file = std::fs::File::create(&path).map_err(|error| {
                    tracing::error!(
                        "failed creating '{}', url={}, error={}",
                        path.display(),
                        url,
                        error
                    );
                    Error::CantCreateDir {
                        path: path.clone(),
                        error,
                    }
                })?;
                file.write_all(&podd_mp3).map_err(|error| {
                    tracing::error!("failed writing to '{}': {}", path.display(), error);
                    Error::FailedWritingFile {
                        path: path.clone(),
                        error,
                    }
                })?;
                Ok(String::new())
            }
            Item::PoddMeta {
                meta_url,
                title,
                release_date,
                description,
                mp3_url,
            } => {
                let meta_url = Url::parse(&meta_url).unwrap();
                let path = path.join("podd_meta");
                let mut path = path.join(&meta_url.path()[1..]);
                path.set_extension("json");
                dbg!(&path);
                if let Some(parent) = path.parent() {
                    tokio::fs::create_dir_all(parent).await.map_err(|error| {
                        tracing::error!(
                            "failed creating path='{}', url={}, error={}",
                            parent.display(),
                            url,
                            error
                        );
                        Error::CantCreateDir {
                            path: parent.to_path_buf(),
                            error,
                        }
                    })?;
                }
                let json_data = serde_json::json!(
                    {
                        "title": title,
                        "release_date": release_date,
                        "description": description,
                        "mp3_url": mp3_url,
                    }
                );
                let file = std::fs::File::create(&path).map_err(|error| {
                    tracing::error!(
                        "failed creating '{}', url={}, error={}",
                        path.display(),
                        url,
                        error
                    );
                    Error::CantCreateDir {
                        path: path.clone(),
                        error,
                    }
                })?;
                let mut writer = std::io::BufWriter::new(file);
                serde_json::to_writer(&mut writer, &json_data).map_err(|error| {
                    tracing::error!("Failed to write JSON to '{}': {}", path.display(), error);
                    Error::FailedWritingJson {
                        path: path.clone(),
                        error,
                    }
                })?;
                Ok(String::new())
            }
        }
    }
}
