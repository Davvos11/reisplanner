use std::collections::HashSet;
use std::error::Error;
use std::fs;
use std::fs::{File, read_dir};
use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};

use csv::ReaderBuilder;
use reqwest::blocking::Client;
use serde::Deserialize;
use zip::ZipArchive;

use crate::gtfs::types::{Agency, CalendarDate, FeedInfo, Route, Shape, Stop, StopTime, Transfer, Trip};

pub mod types;

/// Download and extract a GTFS zip
/// Returns a Vec of PathBuf of each extracted file
fn download_gtfs(url: &str, folder_path: &str) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    eprintln!("Downloading...");

    let client = Client::builder()
        .timeout(None) // Disable timeout
        .build()?;

    let response = client.get(url).send()?.error_for_status()?;
    let mut archive = ZipArchive::new(
        Cursor::new(response.bytes()?)
    )?;
    let path = Path::new(folder_path);
    archive.extract(path)?;
    let extracted_paths = archive.file_names()
        .map(|filename| path.join(filename))
        .collect();
    Ok(extracted_paths)
}

fn should_download(url: &str, file_path: &str) -> Result<bool, Box<dyn Error>> {
    if !Path::new(file_path).exists() {
        return Ok(true);
    }

    let response = Client::new()
        .head(url)
        .send()?;

    if response.status().is_success() {
        if let Some(last_modified) = response.headers().get(reqwest::header::LAST_MODIFIED) {
            let last_modified = last_modified.to_str()?;
            let remote_modified_time = httpdate::parse_http_date(last_modified)?;

            let metadata = fs::metadata(file_path)?;
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


pub fn run_gtfs() -> Result<(), Box<dyn Error>> {
    if should_download(URL, FOLDER)? {
        download_gtfs(URL, FOLDER)?;
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

    for unit in transfers {
        dbg!(unit);
    }
    
    Ok(())
}