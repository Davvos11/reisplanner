use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::io::Write;

use protobuf::Message;
use reqwest::blocking::get;

use crate::gtfs_realtime::gtfs_realtime::FeedMessage;

fn download_gtfs_realtime(url: &String, file_path: &String) -> Result<(), Box<dyn Error>> {
    let response = get(url)?;
    let mut file = File::create(file_path)?;
    file.write_all(&response.bytes()?)?;
    Ok(())
}

fn parse_gtfs_realtime(file_path: &String) -> Result<(), Box<dyn Error>> {
    let mut file = File::open(file_path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;

    let feed = FeedMessage::parse_from_bytes(&buffer)?;
    for entity in &feed.entity[..2] {
        dbg!(entity);
    }
    Ok(())
}

pub fn run_gtfs_realtime() -> Result<(), Box<dyn Error>> {
    for stream_title in ["alerts", "trainUpdates", "tripUpdates", "vehiclePositions"] {
        let url = format!("https://gtfs.ovapi.nl/nl/{stream_title}.pb");
        let file_path = format!("{stream_title}.pb");

        download_gtfs_realtime(&url, &file_path)?;
        parse_gtfs_realtime(&file_path)?;
    }
    Ok(())
}