use std::collections::HashMap;
use std::str::FromStr;

use anyhow::Context;
use rbatis::RBatis;
use serde::Deserialize;

use reisplanner_gtfs::utils::TimeTuple;

#[derive(Deserialize)]
struct StationParent {
    stop_id: String,
    parent_station: Option<String>,
}

pub async fn get_parent_station_map(db: &RBatis) -> anyhow::Result<HashMap<u32, u32>> {
    let stops: Vec<StationParent> = db
        .query_decode("select stop_id, parent_station from stop", vec![])
        .await?;

    let mut map = HashMap::with_capacity(stops.len());

    for StationParent { stop_id, parent_station } in stops {
        let stop_id = parse_stop_id(&stop_id)?;
        match parent_station {
            None => {
                map.insert(stop_id, stop_id);
            }
            Some(parent_id) => {
                let parent_id = parse_stop_id(&parent_id)?;
                map.insert(stop_id, parent_id);
            }
        }
    }

    Ok(map)
}

pub fn parse_stop_id(stop_id: &String) -> anyhow::Result<u32> {
    // stop_id is "stoparea:123456", so we parse to just 123456
    // we assume that no regular stop 123456 exists
    if stop_id.starts_with('s') {
        u32::from_str(&stop_id[9..])
            .context(format!("Tried to parse parent_station {stop_id}"))
    } else {
        u32::from_str(stop_id)
            .context(format!("Tried to parse parent_station {stop_id}"))
    }
}

#[derive(Deserialize)]
pub struct StopTimeShort {
    pub id: Option<u32>,
    pub trip_id: u32,
    pub stop_id: u32,
    pub departure_time: TimeTuple,
    pub arrival_time: TimeTuple,
}

pub async fn get_stop_times(last_id: u32, page_size: u64, db: &RBatis) -> anyhow::Result<Vec<StopTimeShort>> {
    // TODO deserialise to u32 instead of timetuple?
    let stop_times: Vec<StopTimeShort> = db
        .query_decode(
            format!("select id, trip_id, stop_id, departure_time, arrival_time 
            from stop_time
            where id >= {last_id} order by id limit {page_size}").as_str(),
            vec![],
        ).await?;

    Ok(stop_times)
}

pub async fn count_stop_times(db: &RBatis) -> anyhow::Result<u64> {
    let count: u64 = db.query_decode("select count(*) from stop_time", vec![]).await?;
    Ok(count)
}


#[derive(Deserialize)]
struct TripRoute {
    trip_id: u32,
    route_id: u32,
}


pub async fn get_trip_route_map(db: &RBatis) -> anyhow::Result<HashMap<u32, u32>> {
    let trips: Vec<TripRoute> = db
        .query_decode("select trip_id, route_id from trip", vec![])
        .await?;

    let mut map = HashMap::with_capacity(trips.len());

    for TripRoute { trip_id, route_id } in trips {
        map.insert(trip_id, route_id);
    }

    Ok(map)
}


#[derive(Deserialize)]
struct TransferTime {
    stop_id: String,
    transfer_time: u32,
}


pub async fn get_transfer_times(db: &RBatis) -> anyhow::Result<HashMap<u32, u32>> {
    let times: Vec<TransferTime> = db
        .query_decode(
            "select s.stop_id, t.transfer_time from station_transfer t
                join stop parent_stop on t.station_code = parent_stop.stop_code
                join stop s on s.parent_station = parent_stop.stop_id",
            vec![])
        .await?;

    let mut map = HashMap::with_capacity(times.len());

    for TransferTime { stop_id, transfer_time } in times {
        map.insert(stop_id.parse()?, transfer_time);
    }

    Ok(map)
}