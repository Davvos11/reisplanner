extern crate protobuf;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use chrono::Utc;
use tokio::signal::ctrl_c;
use tokio::task;
use tokio::time::{Duration as TokioDuration, sleep};

use crate::database::init_db;
use crate::gtfs::{DOWNLOAD_ERROR, run_gtfs, write_error_file};
use crate::gtfs_realtime_parse::run_gtfs_realtime;

pub mod gtfs_realtime;
pub mod gtfs;
mod gtfs_realtime_parse;
pub mod utils;
pub mod database;
pub mod rbatis_wrapper;
mod errors;


const REALTIME_INTERVAL: u64 = 60;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // TODO use tracing instead of println
    let db = init_db().await?;
    // Run initial GTFS download and database update (if needed)
    run_gtfs(&db).await?;
    run_gtfs_realtime(&db).await?;
    
    println!("Starting loop");
    let mut previous_run = Utc::now().naive_utc();
    loop {
        sleep(TokioDuration::from_secs(REALTIME_INTERVAL)).await;
        let result = run_gtfs_realtime(&db).await;
        if let Err(e) = result {
            // TODO better printing
            eprintln!("Error in GTFS realtime loop {e:?}");
        }

        // Check if it has been 03:00 UTC
        let three_am = Utc::now().date_naive().and_hms_opt(3, 0, 0).unwrap();
        let now = Utc::now().naive_utc();
        if previous_run < three_am && now >= three_am {
            let result = run_gtfs(&db).await;
            if let Err(e) = result {
                eprintln!("Error in GTFS static loop {e:?}");
            }
        }

        previous_run = now;
    }
}
