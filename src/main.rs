extern crate protobuf;

use anyhow::Context;
use crate::database::init_db;
use crate::errors::GtfsError;
use crate::gtfs::{DOWNLOAD_ERROR, PARSE_ERROR, remove_error_files, run_gtfs, write_error_file};
use crate::gtfs_realtime_parse::run_gtfs_realtime;

pub mod gtfs_realtime;
pub mod gtfs;
mod gtfs_realtime_parse;
pub mod utils;
pub mod database;
pub mod rbatis_wrapper;
mod errors;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let db = init_db().await
        .context("Test")?;
    println!("Run planning");
    match run_gtfs(&db, false).await {
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
    println!("Run realtime");
    run_gtfs_realtime(&db).await?;
    Ok(())
}
