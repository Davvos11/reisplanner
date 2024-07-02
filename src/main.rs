extern crate protobuf;

use std::error::Error;
use crate::database::init_db;
use crate::gtfs::run_gtfs;
use crate::gtfs_realtime_parse::run_gtfs_realtime;

pub mod gtfs_realtime;
pub mod gtfs;
mod gtfs_realtime_parse;
pub mod utils;
pub mod database;
pub mod rbatis_wrapper;


#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let db = init_db().await?;
    // println!("Run realtime");
    // run_gtfs_realtime()?;
    println!("Run planning");
    run_gtfs(db, false).await?;
    Ok(())
}
