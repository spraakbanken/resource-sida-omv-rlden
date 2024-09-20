use std::{error::Error as StdError, fmt, io, path::PathBuf};

#[derive(Debug)]
pub enum Error {
    CantCreateDir {
        path: PathBuf,
        error: io::Error,
    },
    CantCanonicalizePath {
        path: PathBuf,
        error: io::Error,
    },
    CantCreateHttpClient(reqwest::Error),
    FailedToGetData {
        url: String,
        error: reqwest::Error,
    },
    FailedWritingFile {
        path: PathBuf,
        error: io::Error,
    },
    FailedWritingJson {
        path: PathBuf,
        error: serde_json::Error,
    },
    ScrapeError {
        url: String,
        error: reqwest::Error,
    },
    RequestReturnedError {
        url: String,
        status_code: reqwest::StatusCode,
    },
    Unknown(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CantCanonicalizePath { path, error } => f.write_fmt(format_args!(
                "Can't canonicalize '{}': {}",
                path.display(),
                error
            )),
            Self::CantCreateDir { path, error } => f.write_fmt(format_args!(
                "Can't create dir '{}': {}",
                path.display(),
                error
            )),
            Self::CantCreateHttpClient(error) => {
                f.write_fmt(format_args!("Can't create http client: {}", error))
            }
            Self::FailedToGetData { url, error } => f.write_fmt(format_args!(
                "Failed getting data from '{}': {}",
                url, error
            )),
            Self::FailedWritingFile { path, error } => f.write_fmt(format_args!(
                "Failed to write file '{}': {}",
                path.display(),
                error
            )),
            Self::FailedWritingJson { path, error } => f.write_fmt(format_args!(
                "Failed to write JSON to '{}': {}",
                path.display(),
                error
            )),
            Self::RequestReturnedError { url, status_code } => f.write_fmt(format_args!(
                "The request to '{}' returned {}",
                url, status_code
            )),
            Self::ScrapeError { url, error } => {
                f.write_fmt(format_args!("Failed fetching '{}': {}", url, error))
            }
            Self::Unknown(msg) => f.write_fmt(format_args!("Unknown error: {}", msg)),
        }
    }
}

impl StdError for Error {}
