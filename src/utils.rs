use chrono::{Datelike, NaiveDate};
use rbatis::rbdc;
use serde::{Deserialize, Deserializer};

pub type TimeTuple = (u8, u8, u8);


#[derive(Deserialize)]
#[serde(untagged)]
enum StringOrTimeTuple {
    String(String),
    TimeTuple(TimeTuple),
}

pub fn deserialize_time_tuple<'de, D>(deserializer: D) -> Result<TimeTuple, D::Error>
where
    D: Deserializer<'de>,
{
    match StringOrTimeTuple::deserialize(deserializer)? {
        StringOrTimeTuple::String(s) => {
            let parts: Vec<&str> = s.split(':').collect();
            if parts.len() != 3 {
                return Err(serde::de::Error::custom("Invalid time format"));
            }
            let hours = parts[0].parse::<u8>().map_err(serde::de::Error::custom)?;
            let minutes = parts[1].parse::<u8>().map_err(serde::de::Error::custom)?;
            let seconds = parts[2].parse::<u8>().map_err(serde::de::Error::custom)?;
            Ok((hours, minutes, seconds))
        }
        StringOrTimeTuple::TimeTuple(t) => {
            Ok(t)
        }
    }
}

pub fn deserialize_date<'de, D>(deserializer: D) -> Result<rbdc::Date, D::Error>
where
    D: Deserializer<'de>,
{
    match StringOrDate::deserialize(deserializer)? {
        StringOrDate::String(s) => {
            let chronos_date = NaiveDate::parse_from_str(&s, "%Y%m%d").map_err(serde::de::Error::custom)?;
            let date = fastdate::Date {
                day: chronos_date.day() as u8,
                mon: chronos_date.month() as u8,
                year: chronos_date.year(),
            };
            Ok(rbdc::Date(date))
        }
        StringOrDate::Date(d) => {
            Ok(d)
        }
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
enum StringOrDate {
    String(String),
    Date(rbdc::Date),
}

