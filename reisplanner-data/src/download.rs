use std::{fs, io};
use std::io::Cursor;
use std::path::{Path, PathBuf};
use reqwest::Client;
use reqwest::header::USER_AGENT;
use tracing::debug;
use zip::result::ZipError;
use zip::ZipArchive;
use crate::download::DownloadError::{FileSystem, ParseLocalModified, ParseRemoteModified};

const MAINTAINER_EMAIL: &str = "vosdavid2@gmail.com";
const APP_NAME: &str = env!("CARGO_PKG_NAME");
const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(thiserror::Error, Debug)]
pub enum DownloadError {
    #[error("Failed to get GTFS response")]
    Connection(#[from] reqwest::Error),
    #[error("Failed to parse last modified date from OVapi")]
    ParseRemoteModified(#[source] anyhow::Error),
    #[error("Failed to parse last modified date of local files")]
    ParseLocalModified(#[source] anyhow::Error),
    #[error("Failed to extract the GTFS archive")]
    Unzip(#[from] ZipError),
    #[error("Failed to access the downloaded GTFS files")]
    FileSystem(#[from] io::Error),
}

pub fn get_contact_info() -> String {
    format!("{MAINTAINER_EMAIL}/{APP_NAME}-{APP_VERSION}")
}

/// Download and extract a zip
/// Returns a Vec of PathBuf of each extracted file
pub async fn download_zip(url: &str, folder_path: &str) -> Result<Vec<PathBuf>, DownloadError> {
    let has_updated = match has_updated(url, folder_path).await {
        Err(ParseLocalModified(_)) => true,
        result => result?,
    };
    if has_updated {
        debug!("No need to download data");
        // Just return the contents of the folder
        return get_folder_contents(folder_path);
    }
    debug!("Downloading static data, this will take a while...");

    let client = Client::builder()
        .build()?;

    let response = client
        .get(url)
        .header(USER_AGENT, get_contact_info())
        .send().await?
        .error_for_status()?;
    let mut archive = ZipArchive::new(
        Cursor::new(response.bytes().await?)
    )?;
    let path = Path::new(folder_path);
    archive.extract(path)?;
    let extracted_paths = archive.file_names()
        .map(|filename| path.join(filename))
        .collect();
    Ok(extracted_paths)
}

async fn has_updated(url: &str, file_path: &str) -> Result<bool, DownloadError> {
    if !Path::new(file_path).exists() {
        return Ok(true);
    }

    let response = Client::new()
        .head(url)
        .header(USER_AGENT, get_contact_info())
        .send().await?;

    if response.status().is_success() {
        if let Some(last_modified) = response.headers().get(reqwest::header::LAST_MODIFIED) {
            let last_modified = last_modified.to_str()
                .map_err(|e| ParseRemoteModified(e.into()))?;
            let remote_modified_time = httpdate::parse_http_date(last_modified)
                .map_err(|e| ParseRemoteModified(e.into()))?;

            let mut metadata = fs::metadata(file_path)
                .map_err(|e| ParseLocalModified(e.into()))?;
            if metadata.is_dir() {
                if let Some(first_file) = fs::read_dir(file_path)
                    .map_err(|e| ParseLocalModified(e.into()))?
                    .filter_map(Result::ok)
                    .find(|f| f.metadata().is_ok_and(|f| f.is_file()))
                {
                    metadata = first_file.metadata()
                        .map_err(|e| ParseLocalModified(e.into()))?;
                } else {
                    return Ok(false);
                }
            }
            let local_modified_time = metadata.modified()
                .map_err(|e| ParseLocalModified(e.into()))?;

            return Ok(remote_modified_time > local_modified_time);
        }
    }

    Ok(false)
}

fn get_folder_contents(path: &str) -> Result<Vec<PathBuf>, DownloadError> {
    let entries = fs::read_dir(path)
        .map_err(|e| FileSystem(e))?
        .filter_map(|entry| entry.ok()) // Ignore errors on individual entries
        .filter(|entry| {
            if let Some(name) = entry.file_name().to_str() {
                !name.starts_with('.') // Filter out hidden files/folders
            } else { false } // Skip if file name is invalid Unicode 
        })
        .map(|entry| entry.path())
        .collect();
    Ok(entries)
}
