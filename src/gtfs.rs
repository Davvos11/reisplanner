use std::error::Error;
use std::fs;
use std::fs::File;
use std::io::Cursor;
use std::path::{Path, PathBuf};

use csv::{DeserializeRecordsIter, ReaderBuilder};
use indicatif::ProgressBar;
use itertools::Itertools;
use rbatis::executor::Executor;
use rbatis::RBatis;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::time::sleep;
use zip::ZipArchive;

use crate::gtfs::types::{Agency, CalendarDate, FeedInfo, Route, Shape, Stop, StopTime, Transfer, Trip};
use crate::rbatis_wrapper::DatabaseModel;

pub mod types;

/// Download and extract a GTFS zip
/// Returns a Vec of PathBuf of each extracted file
async fn download_gtfs(url: &str, folder_path: &str) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    println!("Downloading, this will take a while...");

    let client = Client::builder()
        .build()?;

    let response = client.get(url).send().await?.error_for_status()?;
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

async fn should_download(url: &str, file_path: &str) -> Result<bool, Box<dyn Error>> {
    if !Path::new(file_path).exists() {
        return Ok(true);
    }

    let response = Client::new()
        .head(url)
        .send().await?;

    if response.status().is_success() {
        if let Some(last_modified) = response.headers().get(reqwest::header::LAST_MODIFIED) {
            let last_modified = last_modified.to_str()?;
            let remote_modified_time = httpdate::parse_http_date(last_modified)?;

            let mut metadata = fs::metadata(file_path)?;
            if metadata.is_dir() {
                if let Some(first_file) = fs::read_dir(file_path)?
                    .filter_map(Result::ok)
                    .find(|f| f.metadata().is_ok_and(|f| f.is_file()))
                {
                    metadata = first_file.metadata()?;
                } else {
                    return Ok(false);
                }
            }
            let local_modified_time = metadata.modified()?;

            return Ok(remote_modified_time > local_modified_time);
        }
    }

    Ok(false)
}

const DESERIALIZE_CHUNK_SIZE: u64 = 500;

async fn gtfs_to_db<T>(db: &dyn Executor, file_path: &str) -> Result<(), Box<dyn Error>>
    where
        T: for<'de> Deserialize<'de> + Serialize + DatabaseModel<T>,
{
    println!("Saving {file_path} to database...");
    // Open file to count the length for the progressbar
    let file = File::open(file_path)?;
    let mut reader = ReaderBuilder::new().from_reader(file);
    let records: DeserializeRecordsIter<File, T> = reader.deserialize();
    let length = records.count() as u64;
    let bar = ProgressBar::new(length);

    T::delete_all(db).await?;

    // Reopen the file to actually read it
    let file = File::open(file_path)?;
    let mut reader = ReaderBuilder::new().from_reader(file);
    let records: DeserializeRecordsIter<File, T> = reader.deserialize();

    for chunk in &records.chunks(DESERIALIZE_CHUNK_SIZE as usize) {
        let items: Vec<_> = chunk.into_iter().collect::<Result<_, _>>()?;

        T::insert_batch(db, &items, DESERIALIZE_CHUNK_SIZE).await?;
        bar.inc(DESERIALIZE_CHUNK_SIZE);
    }
    
    bar.finish();
    Ok(())
}

const URL: &str = "https://gtfs.ovapi.nl/nl/gtfs-nl.zip";
const FOLDER: &str = "gtfs";


pub async fn run_gtfs(db: &RBatis, force: bool) -> Result<(), Box<dyn Error>> {
    let has_updated = should_download(URL, FOLDER).await?;

    if has_updated {
        download_gtfs(URL, FOLDER).await?;
    }

    if !(has_updated | force) {
        return Ok(())
    }

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

    transaction.commit().await?;

    Ok(())
}