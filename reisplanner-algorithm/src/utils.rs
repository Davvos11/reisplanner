use std::fs::File;
use std::io;
use std::io::{Read, Write};

use serde::{Deserialize, Serialize};

#[macro_export]
macro_rules! benchmark {
    ($timer:expr, $fmt:expr) => {
        println!("{} in {:?}", $fmt, $timer.elapsed());
        *$timer = Instant::now();
    };
    ($timer:expr, $fmt:expr, $($arg:tt)*) => {
        println!("{} in {:?}", format!($fmt, $($arg)*), $timer.elapsed());
        *$timer = Instant::now();
    };
}

pub fn serialize_to_disk<T: Serialize>(data: &T, filename: &str) -> io::Result<()> {
    let encoded: Vec<u8> = bincode::serialize(data).unwrap();
    let mut file = File::create(filename)?;
    file.write_all(&encoded)?;
    Ok(())
}

pub fn deserialize_from_disk<T: for<'de> Deserialize<'de>>(filename: &str) -> io::Result<T> {
    let mut file = File::open(filename)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    let decoded: T = bincode::deserialize(&buffer).unwrap();
    Ok(decoded)
}

pub fn seconds_to_hms(seconds: u32) -> String {
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let seconds = seconds % 60;
    format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
}
