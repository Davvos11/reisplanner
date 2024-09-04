use std::cmp::Ordering;
use std::collections::{BTreeSet, HashMap};

use indicatif::{ProgressBar, ProgressStyle};
use rbatis::executor::Executor;
use rbatis::RBatis;
use serde::{Deserialize, Serialize};
use tracing::debug;
use tracing::field::debug;

use reisplanner_gtfs::gtfs::types::{Route, Trip};

use crate::database::queries::{count_stop_times, get_parent_station_map, get_stop_times};
use crate::getters::get_stop;
use crate::utils::{deserialize_from_disk, seconds_to_hms, serialize_to_disk};

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
pub struct Connection {
    /// Parent stop_id but without the `stopearea:` prefix
    departure_station: u32,
    /// Parent stop_id but without the `stopearea:` prefix
    arrival_station: u32,
    departure_timestamp: u32,
    arrival_timestamp: u32,
    trip_id: u32,
}


impl Ord for Connection {
    fn cmp(&self, other: &Self) -> Ordering {
        self.departure_timestamp.cmp(&other.departure_timestamp)
            .then_with(|| self.arrival_timestamp.cmp(&other.arrival_timestamp))
            .then_with(|| self.departure_station.cmp(&other.departure_station))
            .then_with(|| self.arrival_station.cmp(&other.arrival_station))
            .then_with(|| self.trip_id.cmp(&other.trip_id))
    }
}
impl PartialOrd for Connection {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Connection {
    pub async fn arrival_name(&self, db: &impl Executor) -> anyhow::Result<String> {
        Ok(get_stop(&self.arrival_station, db).await?.stop_name)
    }
    pub async fn departure_name(&self, db: &impl Executor) -> anyhow::Result<String> {
        Ok(get_stop(&self.departure_station, db).await?.stop_name)
    }
    pub async fn route_information(&self, db: &impl Executor) -> anyhow::Result<String> {
        let trip = Trip::select_by_id(db, &self.trip_id).await?
            .ok_or(anyhow::Error::msg("Cannot get trip"))?;
        let route = Route::select_by_id(db, &trip.route_id).await?
            .ok_or(anyhow::Error::msg("Cannot get route"))?;
        Ok(format!("{} {} {}", route.agency_id, route.route_short_name, route.route_long_name))
    }
}

const PAGE_SIZE: u64 = 1_000_000;

async fn generate_timetable(db: &RBatis) -> anyhow::Result<Vec<Connection>> {
    debug!("Getting trips, routes and stations...");
    // let mut t = Instant::now();
    // let mut t1 = Instant::now();
    let parent_stations = get_parent_station_map(db).await?;
    // benchmark!(&mut t, "Got stops");
    let total_count = count_stop_times(db).await?;
    // benchmark!(&mut t, "Got stop_time count");

    debug("Generating timetable from stop_times, this will take a while...");
    let mut timetable = BTreeSet::new();
    let bar = ProgressBar::new(total_count);
    bar.set_style(ProgressStyle::default_bar()
        .template("{wide_bar} Elapsed: {elapsed_precise}, ETA: {eta_precise}")
        .unwrap());

    let mut highest_id = 0;
    loop {
        let stop_times = get_stop_times(highest_id, PAGE_SIZE, db).await?;
        let count = stop_times.len();
        // benchmark!(&mut t, "Got {count} stop_times");

        let stop_connections = stop_times.windows(2);
        // let mut t2 = Instant::now();
        let mut first = true;

        for window in stop_connections {
            if let [dep_stop, arr_stop] = window {
                // Don't create a connection for stops on different trips
                if dep_stop.trip_id != arr_stop.trip_id { continue; }

                // t2 = Instant::now();
                let dep_station = parent_stations.get(&dep_stop.stop_id)
                    .ok_or(anyhow::Error::msg("Parent station not found"))?;
                // if first { benchmark!(&mut t2, "\tGot dep parent"); };
                let arr_station = parent_stations.get(&arr_stop.stop_id)
                    .ok_or(anyhow::Error::msg("Parent station not found"))?;
                // if first { benchmark!(&mut t2, "\tGot arr parent"); };
                let connection = Connection {
                    departure_station: *dep_station,
                    arrival_station: *arr_station,
                    departure_timestamp: dep_stop.departure_time.into(),
                    arrival_timestamp: arr_stop.arrival_time.into(),
                    trip_id: dep_stop.trip_id,
                };
                // if first { benchmark!(&mut t2, "\tCreated connection"); };
                timetable.insert(connection.clone());
                if first {
                    // benchmark!(&mut t2, "\tInserted connection");
                    // println!();
                };
            }
            if first { first = false; }
        }

        // benchmark!(&mut t, "Generated connections");

        bar.inc(count as u64);
        if count != PAGE_SIZE as usize {
            break;
        } else {
            highest_id = stop_times.last().unwrap().id.unwrap();
        }
    }
    bar.finish();

    // benchmark!(&mut t1, "Generated timetable bintree");
    let timetable: Vec<_> = timetable.into_iter().collect();
    // benchmark!(&mut t1, "Collected timetable vec");
    Ok(timetable)
}

fn csa_main_loop(
    timetable: &[Connection],
    arrival_station: u32,
    earliest_arrival: &mut HashMap<u32, u32>,
    in_connection: &mut HashMap<u32, usize>,
) {
    let mut earliest = u32::MAX;

    for (i, connection) in timetable.iter().enumerate() {
        if connection.departure_timestamp >= *earliest_arrival.get(&connection.departure_station).unwrap_or(&u32::MAX)
            && connection.arrival_timestamp < *earliest_arrival.get(&connection.arrival_station).unwrap_or(&u32::MAX)
        {
            earliest_arrival.insert(connection.arrival_station, connection.arrival_timestamp);
            in_connection.insert(connection.arrival_station, i);

            if connection.arrival_station == arrival_station && connection.arrival_timestamp < earliest {
                earliest = connection.arrival_timestamp;
            }
        } else if connection.arrival_timestamp > earliest {
            break;
        }
    }
}

const TIMETABLE: &str = "csa_timetable.blob";

pub async fn get_timetable(db: &RBatis, cache: bool) -> anyhow::Result<Vec<Connection>> {
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
    departure: u32,
    arrival: u32,
    departure_time: impl Into<u32>,
    timetable: &[Connection],
) -> anyhow::Result<Option<Vec<Connection>>> {
    let mut in_connection = HashMap::with_capacity(1000);
    let mut earliest_arrival = HashMap::with_capacity(1000);

    earliest_arrival.insert(departure, departure_time.into());

    // let mut t = Instant::now();
    csa_main_loop(timetable, arrival, &mut earliest_arrival, &mut in_connection);
    // benchmark!(&mut t, "Found route");

    if !in_connection.contains_key(&arrival) {
        Ok(None)
    } else {
        let mut route = Vec::new();
        let mut last_connection_idx = in_connection.get(&arrival);

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
            connection.route_information(db).await?
        )
    }

    Ok(())
}

