extern crate protobuf;

use chrono::Utc;
use tokio::time::{Duration as TokioDuration, sleep};
use tracing::{error, info};
use tracing_subscriber::EnvFilter;
use reisplanner_gtfs::gtfs::run_gtfs;
use crate::database::init_db;
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

    let db = init_db().await?;
    // Run initial GTFS download and database update (if needed)
    run_gtfs(&db).await?;
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
            let result = run_gtfs(&db).await;
            if let Err(e) = result {
                error!("Error in GTFS static loop {e:?}");
            }
        }

        previous_run = now;
    }
}
