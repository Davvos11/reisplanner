use std::thread;
use chrono::Utc;
use rbatis::RBatis;
use tokio::runtime::Runtime;
use tokio::time::{Duration as TokioDuration, sleep};

use crate::gtfs::run_gtfs;
use crate::gtfs_realtime_parse::run_gtfs_realtime;

const REALTIME_INTERVAL: u64 = 60;

pub fn start_gtfs_daemon(db: &RBatis) -> anyhow::Result<()> {
    let db_clone = db.clone();
    let handle = thread::spawn(move || {
        let rt = Runtime::new().unwrap();
        rt.block_on(gtfs_daemon(&db_clone));
    });

    handle.join().unwrap();
    Ok(())
}

async fn gtfs_daemon(db: &RBatis) {
    println!("Starting background loop");
    let mut previous_run = Utc::now().naive_utc();
    loop {
        sleep(TokioDuration::from_secs(REALTIME_INTERVAL)).await;
        let result = run_gtfs_realtime(db).await;
        if let Err(e) = result {
            // TODO better printing
            eprintln!("Error in GTFS realtime loop {e:?}");
        }

        // Check if it has been 03:00 UTC
        let three_am = Utc::now().date_naive().and_hms_opt(3, 0, 0).unwrap();
        let now = Utc::now().naive_utc();
        if previous_run < three_am && now >= three_am {
            let result = run_gtfs(db, false).await;
            if let Err(e) = result {
                eprintln!("Error in GTFS static loop {e:?}");
            }
        }

        previous_run = now;
    }
}
