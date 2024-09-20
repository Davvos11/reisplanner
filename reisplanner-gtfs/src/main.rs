extern crate protobuf;

use std::env;
use chrono::Utc;
use tokio::time::{Duration as TokioDuration, sleep};
use tracing::{error, info};
use tracing_subscriber::EnvFilter;
use reisplanner_gtfs::gtfs::run_gtfs;
use reisplanner_utils::database::drop_indices;
use crate::database::{add_indices, init_db};
use crate::gtfs_realtime_parse::run_gtfs_realtime;

mod gtfs_realtime_parse;
pub mod database;


const REALTIME_INTERVAL: u64 = 60;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let log_level = EnvFilter::try_from_default_env()
        .unwrap_or(EnvFilter::new("error,reisplanner=debug"));
    tracing_subscriber::fmt().with_env_filter(log_level).init();
    info!("Starting initial run");
    
    let args = env::args();
    let mut only_db = false;
    for arg in args {
        if arg == "--only-db" {
            info!("--only-db, will only create tables and indices");
            only_db = true;
        }
    }

    let db = init_db().await?;
    if only_db { 
        // If only db add indices now
        // Otherwise we will add them after insertion
        add_indices(&db).await?;
        return Ok(()) 
    }
    
    // Run initial GTFS download and database update (if needed)
    run_gtfs(&db).await?;
    // Add indices (after insertion)
    let mut indices = add_indices(&db).await?;
    // Run realtime updates
    run_gtfs_realtime(&db).await?;

    info!("Starting update loop");
    let mut previous_run = Utc::now().naive_utc();
    loop {
        sleep(TokioDuration::from_secs(REALTIME_INTERVAL)).await;
        let result = run_gtfs_realtime(&db).await;
        if let Err(e) = result {
            error!("Error in GTFS realtime loop {e:?}");
        }

        // Check if it has been 03:00 UTC
        let three_am = Utc::now().date_naive().and_hms_opt(3, 0, 0).unwrap();
        let now = Utc::now().naive_utc();
        if previous_run < three_am && now >= three_am {
            // Drop indices for faster insertion
            drop_indices(&db, &indices).await?;
            let result = run_gtfs(&db).await;
            if let Err(e) = result {
                error!("Error in GTFS static loop {e:?}");
            }
            // Add indices back again
            indices = add_indices(&db).await?;
        }

        previous_run = now;
    }
}
