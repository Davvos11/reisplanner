use std::fs::File;
use std::io::Read;
use std::io::Write;

use protobuf::{EnumOrUnknown, Message};
use rbatis::executor::Executor;
use rbatis::RBatis;
use reqwest::Client;
use reqwest::header::USER_AGENT;

use crate::errors::{DownloadError, GtfsError, ParseError};
use crate::gtfs::get_contact_info;
use crate::gtfs::types::{StopTime, Trip};
use crate::gtfs_realtime::gtfs_realtime::{FeedEntity, FeedMessage};
use crate::gtfs_realtime::gtfs_realtime::feed_header::Incrementality::FULL_DATASET;
use crate::utils::{parse_int, parse_optional_int, parse_optional_int_option};

async fn download_gtfs_realtime(url: &String, file_path: &String) -> Result<(), DownloadError> {
    let response = Client::new()
        .get(url)
        .header(USER_AGENT, get_contact_info())
        .send().await?
        .error_for_status()?;
    let mut file = File::create(file_path)?;
    file.write_all(&response.bytes().await?)?;
    Ok(())
}

async fn parse_gtfs_realtime(file_path: &String, db: &dyn Executor) -> Result<(), GtfsError> {
    let mut file = File::open(file_path).map_err(DownloadError::FileSystem)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).map_err(DownloadError::FileSystem)?;

    let feed = FeedMessage::parse_from_bytes(&buffer)
        .map_err(|e| ParseError::Protobuf(e, file_path.to_string()))?;

    assert_eq!(feed.header.incrementality, Some(EnumOrUnknown::new(FULL_DATASET)),
               "Full dataset expected");

    for entity in &feed.entity[..200] {
        parse_gtfs_realtime_entry(entity, db).await?;
    }
    Ok(())
}

async fn parse_gtfs_realtime_entry(entry: &FeedEntity, db: &dyn Executor) -> Result<(), GtfsError> {
    if let Some(trip_update) = entry.trip_update.as_ref() {
        let trip_id = parse_optional_int(trip_update.trip.trip_id.as_ref(), "trip_id")
            .map_err(|e| ParseError::Realtime(e, Box::new(trip_update.clone())))?;

        for (i, update) in trip_update.stop_time_update.iter().enumerate() {
            let stop_id: Option<u32> = parse_optional_int_option(update.stop_id.as_ref(), "stop_id")
                .map_err(|e| ParseError::Realtime(e, Box::new(update.clone())))?;
            // Find result by either stop_sequence or by stop_id
            let mut result = match stop_id {
                Some(stop_id) => {
                    StopTime::select_by_id_and_trip(db, &stop_id, &trip_id).await?
                }
                None => {
                    match update.stop_sequence {
                        Some(stop_sequence) => {
                            StopTime::select_by_sequence_and_trip(db, &stop_sequence, &trip_id).await?
                        }
                        None => {
                            eprintln!("Update for trip_id {:?} has no stop_id or _sequence,\n\thas arrival: {:?}\n\thas departure: {:?}", trip_update.trip.trip_id, update.arrival, update.departure);
                            continue;
                        }
                    }
                }
            };
            assert!(result.len() <= 1, "No more than one trip should have this id or sequence: {update:?}");

            // TODO use certainty and/or schedule relationship
            if let Some(db_stop_time) = result.first_mut() {
                let stop_id = parse_int(&db_stop_time.stop_id, "stop_id")
                    .map_err(|e| ParseError::Database(e, Box::new(db_stop_time.clone())))?;
                // TODO maybe also set if None?
                if let delay @ Some(_) = update.arrival.delay {
                    db_stop_time.arrival_delay = delay;
                }
                if let delay @ Some(_) = update.departure.delay {
                    db_stop_time.departure_delay = delay;
                }
                StopTime::update_by_id_and_trip(db, db_stop_time, &stop_id, &trip_id).await?;
            }
        }
        // Experimental delay field, delay in stop_time_update takes precedent
        // TODO use current departure or arrival delay instead if possible
        if let Some(delay) = trip_update.delay {
            let mut result = Trip::select_by_column(db, "trip_id", &trip_update.trip.trip_id).await?;
            assert!(result.len() <= 1, "No more than one trip should have this id");
            if let Some(db_trip) = result.first_mut() {
                db_trip.delay = Some(delay);
                Trip::update_by_column(db, db_trip, "trip_id").await?;
            }
        }
    }
    if let Some(vehicle_position) = entry.vehicle.as_ref() {}
    if let Some(alert) = entry.alert.as_ref() {}
    for (num, field) in entry.special_fields.unknown_fields().iter() {}

    Ok(())
}

pub async fn run_gtfs_realtime(db: &RBatis) -> Result<(), GtfsError> {
    let mut transaction = db.acquire_begin().await?;

    for stream_title in ["alerts", "trainUpdates", "tripUpdates", "vehiclePositions"] {
        let url = format!("https://gtfs.ovapi.nl/nl/{stream_title}.pb");
        let file_path = format!("{stream_title}.pb");

        download_gtfs_realtime(&url, &file_path).await?;
        parse_gtfs_realtime(&file_path, &transaction).await?;
    }

    transaction.commit().await?;

    Ok(())
}
