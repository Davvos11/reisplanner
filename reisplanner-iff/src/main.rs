use std::fs::File;
use anyhow::Context;
use csv::{DeserializeRecordsIter, ReaderBuilder, Trim};
use encoding_rs::WINDOWS_1252;
use encoding_rs_io::DecodeReaderBytesBuilder;
use rbatis::executor::Executor;
use serde::{Deserialize, Serialize};
use tracing::debug;
use tracing_subscriber::EnvFilter;
use crate::database::init_db;
use crate::types::{ConnectionMode, ContConnection, Station, Transfer};

mod database;
mod types;
mod utils;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let log_level = EnvFilter::try_from_default_env()
        .unwrap_or(EnvFilter::new("error,reisplanner=debug"));
    tracing_subscriber::fmt().with_env_filter(log_level).init();
    
    let db = init_db().await?;
    
    // TODO download and unzip

    debug!("Parsing IFF files...");
    let connection_modes: Vec<ConnectionMode> = parse_csv("iff/connmode.dat").await
        .context("Parsing connmode")?;
    let connection_modes = vec_to_hashmap!(connection_modes, con_code);
    let cont_connections: Vec<ContConnection> = parse_csv("iff/contconn.dat").await
        .context("Parsing contconn")?;
    let stations: Vec<Station> = parse_csv("iff/stations.dat").await
        .context("Parsing stations")?;

    debug!("Processing transfers...");
    let mut result = Vec::new();
    // Process contconns
    for connection in cont_connections {
        result.push(Transfer {
            stop_code_from: connection.from_station,
            stop_code_to: Some(connection.to_station),
            transfer_time: connection.transfer_time,
            transfer_type: Some(connection.transfer_type),
            transfer_description: Some(connection_modes.get(&connection.transfer_type).unwrap().con_mode.clone()),
        })
    }
    
    // Process stations
    for station in stations {
        result.push(Transfer {
            stop_code_from: station.station_abr,
            stop_code_to: None,
            transfer_time: station.transfer_time,
            transfer_type: None,
            transfer_description: None,
        })
    }
    
    debug!("Updating database...");
    Transfer::delete_all(&db).await?;
    Transfer::insert_batch(&db, &result, 1000).await?;
    
    Ok(())
}


async fn parse_csv<T>(file_path: &str) -> anyhow::Result<Vec<T>>
where
    T: for<'de> Deserialize<'de> + Serialize,
{
    let file = File::open(file_path)?;
    // Use ISO 8859-1 (Latin 1) encoding
    let file = DecodeReaderBytesBuilder::new()
        .encoding(Some(WINDOWS_1252))
        .build(file);
    let mut reader = ReaderBuilder::new()
        .has_headers(false)
        .comment(Some(b'@'))
        .trim(Trim::All)
        .from_reader(file);
    let records: DeserializeRecordsIter<_, T> = reader.deserialize();

    let mut result = Vec::new();
    for record in records {
        result.push(record?);
    }

    Ok(result)
}
