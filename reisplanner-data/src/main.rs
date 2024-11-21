use std::fs::File;
use anyhow::Context;
use csv::{DeserializeRecordsIter, ReaderBuilder, Trim};
use encoding_rs::WINDOWS_1252;
use encoding_rs_io::DecodeReaderBytesBuilder;
use serde::{Deserialize, Serialize};
use tracing::debug;
use tracing_subscriber::EnvFilter;
use crate::database::init_db;
use crate::types::{ConnectionMode, ContConnection, Station, StationTransfer};

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
    let _connection_modes = vec_to_hashmap!(connection_modes, con_code);
    let _cont_connections: Vec<ContConnection> = parse_csv("iff/contconn.dat").await
        .context("Parsing contconn")?;
    let stations: Vec<Station> = parse_csv("iff/stations.dat").await
        .context("Parsing stations")?;

    // TODO use _cont_connections as footpaths
    
    debug!("Updating database...");
    let station_transfers: Vec<_> = 
        stations.into_iter().map(StationTransfer::from).collect();
    StationTransfer::delete_all(&db).await?;
    StationTransfer::insert_batch(&db, &station_transfers, 1000).await?;
    
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
