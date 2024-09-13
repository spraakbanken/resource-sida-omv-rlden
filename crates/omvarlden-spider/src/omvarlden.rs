use std::{fmt, fs, io::Write, path::PathBuf};

use async_trait::async_trait;
use reqwest::{Client, Url};
use scraper::{Html, Selector};

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
    type Item = String;
    type Error = Error;

    fn name(&self) -> String {
        "omvarlden".into()
    }

    fn start_urls(&self) -> Vec<String> {
        vec![Self::BASE_URL.to_string()]
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
        let text = response.text().await.map_err(|error| {
            tracing::error!("Failed getting text: {}", error);
            Error::FailedToGetText {
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
        items.push(text);
        Ok((items, new_urls))
    }

    #[tracing::instrument(skip(item))]
    async fn process(&self, url: String, item: Self::Item) -> Result<String, Error> {
        let mut path = self.output_path.clone();
        tracing::debug!(path = ?path);
        tracing::info!(url, "analyzing url");
        let url = Url::parse(&url).unwrap();
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
            path.set_extension("html");
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
            let mut writer = std::io::BufWriter::new(file);
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
}
