use chrono::NaiveDate;
use serde::{Deserialize, Deserializer};

pub type TimeTuple = (u8, u8, u8);

pub fn deserialize_time_tuple<'de, D>(deserializer: D) -> Result<TimeTuple, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() != 3 {
        return Err(serde::de::Error::custom("Invalid time format"));
    }
    let hours = parts[0].parse::<u8>().map_err(serde::de::Error::custom)?;
    let minutes = parts[1].parse::<u8>().map_err(serde::de::Error::custom)?;
    let seconds = parts[2].parse::<u8>().map_err(serde::de::Error::custom)?;
    Ok((hours, minutes, seconds))
}

pub fn deserialize_date<'de, D>(deserializer: D) -> Result<NaiveDate, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    NaiveDate::parse_from_str(&s, "%Y%m%d").map_err(serde::de::Error::custom)
}