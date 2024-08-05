use std::{fs, io};
use std::fmt::Debug;
use std::fs::File;
use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};

use csv::{DeserializeRecordsIter, ReaderBuilder};
use indicatif::ProgressBar;
use itertools::Itertools;
use rbatis::executor::Executor;
use rbatis::RBatis;
use reqwest::Client;
use reqwest::header::USER_AGENT;
use serde::{Deserialize, Serialize};
use tracing::{debug, instrument};
use zip::ZipArchive;

use DownloadError::ParseRemoteModified;

use crate::errors::{DownloadError, GtfsError};
use crate::errors::DownloadError::{FileSystem, ParseLocalModified};
use crate::errors::ParseError::Csv;
use crate::gtfs::types::{Agency, CalendarDate, FeedInfo, Route, Shape, Stop, StopTime, Transfer, Trip};
use crate::rbatis_wrapper::DatabaseModel;

pub mod types;

const MAINTAINER_EMAIL: &str = "vosdavid2@gmail.com";
const APP_NAME: &str = env!("CARGO_PKG_NAME");
const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn get_contact_info() -> String {
    format!("{MAINTAINER_EMAIL}/{APP_NAME}-{APP_VERSION}")
}

/// Download and extract a GTFS zip
/// Returns a Vec of PathBuf of each extracted file
async fn download_gtfs(url: &str, folder_path: &str) -> Result<Vec<PathBuf>, DownloadError> {
    debug!("Downloading static GTFS data, this will take a while...");

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

pub const DOWNLOAD_ERROR: &str = "download";
pub const PARSE_ERROR: &str = "parse";
pub const ERRORS: &[&str] = &[DOWNLOAD_ERROR, PARSE_ERROR];

pub fn write_error_file<T: Debug>(name: &'static str, error: &T) -> io::Result<()> {
    let path = Path::new(FOLDER).join(name);
    // Create all necessary subdirectories
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Create and write to the file
    let mut file = File::create(path)?;
    let content = format!("{:?}", error);
    file.write_all(content.as_bytes())?;
    Ok(())
}

pub fn remove_error_files() -> io::Result<()> {
    for name in ERRORS {
        let path = Path::new(FOLDER).join(name);
        // Check if the file exists
        if path.exists() {
            // Remove the file
            fs::remove_file(path)?;
        }
    }
    Ok(())
}

enum ErrorFiles {
    DownloadError,
    ParseError,
}

fn check_error_files() -> io::Result<Option<ErrorFiles>> {
    for &name in ERRORS {
        let path = Path::new(FOLDER).join(name);
        // Check if the file exists
        if path.exists() {
            match name {
                DOWNLOAD_ERROR => { return Ok(Some(ErrorFiles::DownloadError)); }
                PARSE_ERROR => { return Ok(Some(ErrorFiles::ParseError)); }
                _ => { panic!("Not all error files implemented in check_error_files"); }
            }
        }
    }
    Ok(None)
}

const DESERIALIZE_CHUNK_SIZE: u64 = 500;

async fn gtfs_to_db<T>(db: &dyn Executor, file_path: &str) -> Result<(), GtfsError>
where
    T: for<'de> Deserialize<'de> + Serialize + DatabaseModel<T>,
{
    // TODO change this and bar according to debug level
    eprintln!("Saving {file_path} to database...");
    // Open file to count the length for the progressbar
    let file = File::open(file_path).map_err(FileSystem)?;
    let mut reader = ReaderBuilder::new().from_reader(file);
    let records: DeserializeRecordsIter<File, T> = reader.deserialize();
    let length = records.count() as u64;
    let bar = ProgressBar::new(length);

    T::delete_all(db).await?;

    // Reopen the file to actually read it
    let file = File::open(file_path).map_err(FileSystem)?;
    let mut reader = ReaderBuilder::new().from_reader(file);
    let records: DeserializeRecordsIter<File, T> = reader.deserialize();

    for chunk in &records.chunks(DESERIALIZE_CHUNK_SIZE as usize) {
        let items: Vec<_> = chunk.into_iter().collect::<Result<_, _>>()
            .map_err(|e| Csv(e, file_path.to_string()))?;

        T::insert_batch(db, &items, DESERIALIZE_CHUNK_SIZE).await?;
        bar.inc(DESERIALIZE_CHUNK_SIZE);
    }

    bar.finish();
    Ok(())
}

async fn add_stop_time_ids(db: &dyn Executor) -> Result<(), GtfsError> {
    dbg!("Sorting stop_time...");
    db.exec(
        "WITH OrderedRows AS (
                SELECT
                ROW_NUMBER() OVER (ORDER BY trip_id, stop_sequence) AS row_num,
                trip_id, stop_sequence
                FROM stop_time
            )
            UPDATE stop_time
            SET id = OrderedRows.row_num
            FROM OrderedRows
            WHERE stop_time.trip_id = OrderedRows.trip_id
            AND stop_time.stop_sequence = OrderedRows.stop_sequence;",
        vec![]).await?;

    Ok(())
}

const URL: &str = "https://gtfs.ovapi.nl/nl/gtfs-nl.zip";
const FOLDER: &str = "reisplanner-gtfs/gtfs";


async fn download_parse_gtfs(db: &RBatis) -> Result<(), GtfsError> {
    debug!("Run planning");
    // Check if a previous run of the program has failed while downloading or parsing.
    let previous_errors = check_error_files()
        .map_err(|e| GtfsError::Misc(e.into()))?;

    // Determine if we should download the GTFS files
    let has_updated = match has_updated(URL, FOLDER).await {
        Err(ParseLocalModified(_)) => true,
        result => result?,
    };
    let mut should_download = has_updated;
    if let Some(ErrorFiles::DownloadError) = previous_errors {
        should_download = true;
    }

    // Download GTFS files if needed
    if should_download {
        download_gtfs(URL, FOLDER).await?;
    } else {
        debug!("No need to download static GTFS");
    }

    // Determine if we should parse the GTFS files and add them to the database
    let mut should_parse = has_updated;
    if let Some(ErrorFiles::ParseError) = previous_errors {
        should_parse = true;
    }
    if !should_parse {
        debug!("No need to parse static GTFS");
        return Ok(());
    }

    // Parse data and sync with db
    let mut transaction = db.acquire_begin().await?;

    gtfs_to_db::<Agency>(&transaction, format!("{FOLDER}/agency.txt").as_str()).await?;
    gtfs_to_db::<CalendarDate>(&transaction, format!("{FOLDER}/calendar_dates.txt").as_str()).await?;
    gtfs_to_db::<FeedInfo>(&transaction, format!("{FOLDER}/feed_info.txt").as_str()).await?;
    gtfs_to_db::<Route>(&transaction, format!("{FOLDER}/routes.txt").as_str()).await?;
    gtfs_to_db::<Stop>(&transaction, format!("{FOLDER}/stops.txt").as_str()).await?;
    gtfs_to_db::<Transfer>(&transaction, format!("{FOLDER}/transfers.txt").as_str()).await?;
    gtfs_to_db::<Trip>(&transaction, format!("{FOLDER}/trips.txt").as_str()).await?;
    gtfs_to_db::<Shape>(&transaction, format!("{FOLDER}/shapes.txt").as_str()).await?;
    gtfs_to_db::<StopTime>(&transaction, format!("{FOLDER}/stop_times.txt").as_str()).await?;
    add_stop_time_ids(db).await?;

    transaction.commit().await?;

    Ok(())
}

#[instrument(skip(db))]
pub async fn run_gtfs(db: &RBatis) -> Result<(), GtfsError> {
    // TODO write error files on ctrl+c
    // Run planning and write error files on error, remove any previous error files on success
    match download_parse_gtfs(db).await {
        Ok(()) => {
            remove_error_files()?
        }
        Err(e) => {
            let error_name = match e {
                GtfsError::Parse(_) | GtfsError::Database(_) => PARSE_ERROR,
                _ => DOWNLOAD_ERROR,
            };
            write_error_file(error_name, &e)?;
            Err(e)?;
        }
    }
    Ok(())
}