use std::error::Error;
use std::fs;
use std::fs::File;
use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};

use csv::ReaderBuilder;
use rbatis::RBatis;
use reqwest::Client;
use serde::Deserialize;
use zip::ZipArchive;

use crate::gtfs::types::{Agency, CalendarDate, FeedInfo, Route, Shape, Stop, StopTime, Transfer, Trip};

pub mod types;

/// Download and extract a GTFS zip
/// Returns a Vec of PathBuf of each extracted file
async fn download_gtfs(url: &str, folder_path: &str) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    eprintln!("Downloading...");

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

fn parse_gtfs<T>(file_path: &str) -> Result<Vec<T>, Box<dyn Error>>
    where
        T: for<'de> Deserialize<'de>,
{
    let file = File::open(file_path)?;
    let mut reader = ReaderBuilder::new().from_reader(file);

    let records: Result<Vec<T>, _> = reader.deserialize().take(100).collect();

    if let Err(error) = &records {
        dbg!(file_path);
    }

    Ok(records?)
}

const URL: &str = "https://gtfs.ovapi.nl/nl/gtfs-nl.zip";
const FOLDER: &str = "gtfs";


pub async fn run_gtfs(db: RBatis) -> Result<(), Box<dyn Error>> {
    if should_download(URL, FOLDER).await? {
        download_gtfs(URL, FOLDER).await?;
    }

    let agencies: Vec<Agency> = parse_gtfs(format!("{FOLDER}/agency.txt").as_str())?;
    let calendar_dates: Vec<CalendarDate> = parse_gtfs(format!("{FOLDER}/calendar_dates.txt").as_str())?;
    let feed_info: Vec<FeedInfo> = parse_gtfs(format!("{FOLDER}/feed_info.txt").as_str())?;
    let routes: Vec<Route> = parse_gtfs(format!("{FOLDER}/routes.txt").as_str())?;
    let shapes: Vec<Shape> = parse_gtfs(format!("{FOLDER}/shapes.txt").as_str())?;
    let stops: Vec<Stop> = parse_gtfs(format!("{FOLDER}/stops.txt").as_str())?;
    let stop_times: Vec<StopTime> = parse_gtfs(format!("{FOLDER}/stop_times.txt").as_str())?;
    let transfers: Vec<Transfer> = parse_gtfs(format!("{FOLDER}/transfers.txt").as_str())?;
    let trips: Vec<Trip> = parse_gtfs(format!("{FOLDER}/trips.txt").as_str())?;

    let mut transaction = db.acquire_begin().await?;
    Agency::delete_all(&transaction).await?;
    Agency::insert_batch(&transaction, &agencies, 64).await?;
    transaction.commit().await?;

    Ok(())
}