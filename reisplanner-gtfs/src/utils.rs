use std::any::type_name;
use std::fmt;
use std::num::ParseIntError;
use std::str::FromStr;

use chrono::{Datelike, NaiveDate};
use rbatis::rbdc;
use serde::{de, Deserialize, Deserializer};
use serde::de::{SeqAccess, Visitor};

use crate::errors::FieldParseError;

pub type TimeTuple = (u8, u8, u8);


pub fn deserialize_time_tuple<'de, D>(deserializer: D) -> Result<TimeTuple, D::Error>
where
    D: Deserializer<'de>,
{
    struct TimeTupleVisitor;

    impl<'de> Visitor<'de> for TimeTupleVisitor {
        type Value = TimeTuple;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a time string in the format HH:MM:SS or a valid time tuple")
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            let parts: Vec<&str> = value.split(':').collect();
            if parts.len() != 3 {
                return Err(de::Error::custom("Invalid time format"));
            }
            let hours = parts[0].parse::<u8>().map_err(de::Error::custom)?;
            let minutes = parts[1].parse::<u8>().map_err(de::Error::custom)?;
            let seconds = parts[2].parse::<u8>().map_err(de::Error::custom)?;
            Ok((hours, minutes, seconds))
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: SeqAccess<'de>,
        {
            let hours = seq.next_element::<u8>()?.ok_or_else(|| de::Error::invalid_length(0, &self))?;
            let minutes = seq.next_element::<u8>()?.ok_or_else(|| de::Error::invalid_length(1, &self))?;
            let seconds = seq.next_element::<u8>()?.ok_or_else(|| de::Error::invalid_length(2, &self))?;
            Ok((hours, minutes, seconds))
        }

        fn visit_map<M>(self, map: M) -> Result<Self::Value, M::Error>
        where
            M: de::MapAccess<'de>,
        {
            Deserialize::deserialize(de::value::MapAccessDeserializer::new(map))
        }
    }

    deserializer.deserialize_any(TimeTupleVisitor)
}


pub fn deserialize_date<'de, D>(deserializer: D) -> Result<rbdc::Date, D::Error>
where
    D: Deserializer<'de>,
{
    struct DateVisitor;

    impl<'de> Visitor<'de> for DateVisitor {
        type Value = rbdc::Date;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a date string in the format YYYYMMDD or a valid date object")
        }

        fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            self.visit_str(&value.to_string())
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            let chronos_date = NaiveDate::parse_from_str(value, "%Y%m%d")
                .map_err(de::Error::custom)?;
            let date = fastdate::Date {
                day: chronos_date.day() as u8,
                mon: chronos_date.month() as u8,
                year: chronos_date.year(),
            };
            Ok(rbdc::Date(date))
        }

        fn visit_map<M>(self, map: M) -> Result<Self::Value, M::Error>
        where
            M: de::MapAccess<'de>,
        {
            Deserialize::deserialize(de::value::MapAccessDeserializer::new(map))
        }
    }

    deserializer.deserialize_any(DateVisitor)
}

pub fn parse_int<F>(item: &String, name: &'static str) -> Result<F, FieldParseError>
where
    F: FromStr<Err=ParseIntError>,
{
    item.parse()
        .map_err(|e: ParseIntError| FieldParseError::Conversion(e.into(), name, type_name::<F>()))
}

pub fn parse_optional_int<F>(item: Option<&String>, name: &'static str) -> Result<F, FieldParseError>
where
    F: FromStr<Err=ParseIntError>,
{
    parse_int(item.ok_or_else(|| FieldParseError::Empty(name.to_string()))?, name)
}

pub fn parse_optional_int_option<F>(item: Option<&String>, name: &'static str) -> Result<Option<F>, FieldParseError>
where
    F: FromStr<Err=ParseIntError>,
{
    Ok(
        match item {
            None => { None }
            Some(item) => { Some(parse_int(item, name)?) }
        }
    )
}