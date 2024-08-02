use std::cmp::Ordering;
use std::collections::{BTreeSet, HashMap};
use std::iter;
use std::process::exit;
use indicatif::ProgressBar;
use rbatis::executor::Executor;
use rbatis::PageRequest;
use serde::{Deserialize, Serialize};
use tokio::time::Instant;

use reisplanner_gtfs::gtfs::types::{Route, Stop, StopTime, Trip};
use reisplanner_gtfs::utils::TimeTuple;
use crate::{benchmark, vec_to_hashmap, vec_to_hashmap_list};
use crate::getters::{get_parent_station_map, get_route, get_stop_str};
use crate::utils::{deserialize_from_disk, seconds_to_hms, serialize_to_disk};

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
pub struct Connection {
    departure_station: String,
    arrival_station: String,
    departure_timestamp: u32,
    arrival_timestamp: u32,
    route_description: String,
}


impl Ord for Connection {
    fn cmp(&self, other: &Self) -> Ordering {
        self.departure_timestamp.cmp(&other.departure_timestamp)
            .then_with(|| self.arrival_timestamp.cmp(&other.arrival_timestamp))
            .then_with(|| self.departure_station.cmp(&other.departure_station))
            .then_with(|| self.arrival_station.cmp(&other.arrival_station))
            .then_with(|| self.route_description.cmp(&other.route_description))
    }
}
impl PartialOrd for Connection {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Connection {
    pub async fn arrival_name(&self, db: &impl Executor) -> anyhow::Result<String> {
        Ok(get_stop_str(&self.arrival_station, db).await?.stop_name)
    }
    pub async fn departure_name(&self, db: &impl Executor) -> anyhow::Result<String> {
        Ok(get_stop_str(&self.departure_station, db).await?.stop_name)
    }
}

const PAGE_SIZE: u64 = 100000;

async fn generate_timetable(db: &impl Executor) -> anyhow::Result<Vec<Connection>> {
    eprintln!("Getting trips, routes and stations...");
    let mut t = Instant::now();
    let trips = Trip::select_all(db).await?;
    // let trips = Trip::select_by_column(db, "trip_long_name", "Intercity").await?;
    let trips = vec_to_hashmap!(trips, trip_id);
    benchmark!(&mut t, "Got trips");
    // let trip_ids: Vec<_> = trips.keys().collect();
    // benchmark!(&mut t, "Collected trip ids");
    let routes = Route::select_all(db).await?;
    let routes = vec_to_hashmap!(routes, route_id);
    benchmark!(&mut t, "Got routes");
    let stops = Stop::select_all(db).await?;
    let stops = vec_to_hashmap!(stops, stop_id);
    benchmark!(&mut t, "Got stops");

    let mut timetable = BTreeSet::new();

    let mut page_request = PageRequest::new(1, PAGE_SIZE);
    page_request = page_request.set_do_count(false);
    let mut stop_time_cache: Option<StopTime> = None;
    let mut highest_trip_id = 0;
    loop {
        let page = StopTime::select_all_grouped_paged_trip_id_gte(db, &page_request, &highest_trip_id).await?;
        benchmark!(&mut t, "Got stop_times");

        let stop_times = page.records;
        let count = stop_times.len();
        let stop_times: Vec<_>=
            if let Some(stop_time) = stop_time_cache {
                iter::once(stop_time).chain(stop_times.into_iter()).collect()
            } else {
                stop_times.into_iter().collect()
            };
        benchmark!(&mut t, "Did iterator shenanigans");

        let stop_connections = stop_times.windows(2);
        // let bar = ProgressBar::new(stop_connections.len() as u64);

        for window in stop_connections {
            // bar.inc(1);
            if let [dep_stop, arr_stop] = window {
                // Don't create a connection for stops on different trips
                if dep_stop.trip_id != arr_stop.trip_id { continue; }

                let dep_station = get_parent_station_map(dep_stop.stop_id, &stops)?;
                let arr_station = get_parent_station_map(arr_stop.stop_id, &stops)?;
                let route = get_route(&dep_stop.trip_id, &trips, &routes)?;
                let connection = Connection {
                    departure_station: dep_station.stop_id,
                    arrival_station: arr_station.stop_id,
                    departure_timestamp: dep_stop.departure_time.into(),
                    arrival_timestamp: arr_stop.arrival_time.into(),
                    route_description: format!("{} {} {}", route.agency_id, route.route_short_name, route.route_long_name),
                };
                timetable.insert(connection.clone());
            }
        }

        // bar.finish();
        benchmark!(&mut t, "Generated connections");

        if count != PAGE_SIZE as usize {
            break;
        } else {
            page_request.page_no += 1;
            stop_time_cache = stop_times.last().cloned();
            highest_trip_id = stop_times.last().unwrap().trip_id;
        }
    }

    let timetable: Vec<_> = timetable.into_iter().collect();
    // dbg!(&timetable);
    Ok(timetable)
}

fn csa_main_loop(
    timetable: &[Connection],
    arrival_station: String,
    earliest_arrival: &mut HashMap<String, u32>,
    in_connection: &mut HashMap<String, usize>,
) {
    let mut earliest = u32::MAX;

    for (i, connection) in timetable.iter().enumerate() {
        if connection.departure_timestamp >= *earliest_arrival.get(&connection.departure_station).unwrap_or(&u32::MAX)
            && connection.arrival_timestamp < *earliest_arrival.get(&connection.arrival_station).unwrap_or(&u32::MAX)
        {
            earliest_arrival.insert(connection.arrival_station.clone(), connection.arrival_timestamp);
            in_connection.insert(connection.arrival_station.clone(), i);

            if connection.arrival_station == arrival_station && connection.arrival_timestamp < earliest {
                earliest = connection.arrival_timestamp;
            }
        } else if connection.arrival_timestamp > earliest {
            break;
        }
    }
}

const TIMETABLE: &str = "timetable.blob";

pub async fn get_timetable(db: &impl Executor, cache: bool) -> anyhow::Result<Vec<Connection>> {
    if !cache {
        generate_timetable(db).await
    } else {
        // Get timetable from disk or generate
        match deserialize_from_disk(TIMETABLE) {
            Ok(timetable) => Ok(timetable),
            Err(_) => {
                let timetable = generate_timetable(db).await?;
                serialize_to_disk(&timetable, TIMETABLE)?;
                Ok(timetable)
            }
        }
    }
}

pub async fn run_csa(
    departure: &String,
    arrival: &String,
    departure_time: impl Into<u32>,
    timetable: &[Connection],
) -> anyhow::Result<Option<Vec<Connection>>> {
    let mut in_connection = HashMap::with_capacity(1000);
    let mut earliest_arrival = HashMap::with_capacity(1000);

    earliest_arrival.insert(departure.clone(), departure_time.into());

    let mut t = Instant::now();
    csa_main_loop(timetable, arrival.clone(), &mut earliest_arrival, &mut in_connection);
    benchmark!(&mut t, "Found route");

    if !in_connection.contains_key(arrival) {
        Ok(None)
    } else {
        let mut route = Vec::new();
        let mut last_connection_idx = in_connection.get(arrival);

        while let Some(index) = last_connection_idx {
            let connection = &timetable[*index];
            route.push(connection);
            last_connection_idx = in_connection.get(&connection.departure_station);
        }

        let route = route.into_iter().cloned().rev().collect();
        Ok(Some(route))
    }
}

pub async fn print_result(result: &[Connection], db: &impl Executor) -> anyhow::Result<()> {
    for connection in result {
        println!(
            "{} @ {} - {} @ {} using {}",
            connection.departure_name(db).await?,
            seconds_to_hms(connection.departure_timestamp),
            connection.arrival_name(db).await?,
            seconds_to_hms(connection.arrival_timestamp),
            connection.route_description
        )
    }

    Ok(())
}

