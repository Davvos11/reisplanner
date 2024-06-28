extern crate protobuf;

use std::error::Error;
use crate::gtfs_realtime_parse::run_gtfs_realtime;

// Import the generated protobuf module
pub mod gtfs_realtime;
mod gtfs_realtime_parse;


fn main() -> Result<(), Box<dyn Error>> {
    run_gtfs_realtime()?;
    Ok(())
}
