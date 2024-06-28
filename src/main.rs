extern crate protobuf;

use std::error::Error;
use crate::gtfs::run_gtfs;
use crate::gtfs_realtime_parse::run_gtfs_realtime;

pub mod gtfs_realtime;
pub mod gtfs;
mod gtfs_realtime_parse;
pub mod utils;


fn main() -> Result<(), Box<dyn Error>> {
    // println!("Run realtime");
    // run_gtfs_realtime()?;
    println!("Run planning");
    run_gtfs()?;
    Ok(())
}
