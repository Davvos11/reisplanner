use crate::algorithms::raptor::visualiser::visualise_earliest_arrivals;
use crate::database::queries::{count_stop_times, get_parent_station_map, get_stop_times, get_transfer_times};
use crate::getters::get_stop_readable;
use crate::types::JourneyPart;
use crate::utils::{deserialize_from_disk, seconds_to_hms, serialize_to_disk};
use anyhow::anyhow;
use indicatif::{ProgressBar, ProgressStyle};
use rbatis::executor::Executor;
use rbatis::RBatis;
use reisplanner_gtfs::gtfs::types::{Route, Trip};
use serde::{Deserialize, Serialize};
use std::cmp::min;
use std::collections::{HashMap, HashSet};
use std::env;
use std::hash::Hash;
use tracing::field::debug;
use tracing::{debug, error, trace, warn};
use crate::algorithms::raptor::Mode::NotApplicable;

mod visualiser;

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq, Copy, Hash)]
pub struct Location {
    pub stop_id: u32,
    /// Parent stop_id but without the `stopearea:` prefix
    pub parent_id: u32,
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct TripLeg {
    pub from_stop: Location,
    pub to_stop: Location,
    pub departure: u32,
    pub arrival: u32,
    pub trip_id: u32,
}

#[derive(Debug, Clone, Eq, PartialEq, Copy)]
pub struct Arrival {
    time: u32,
    stop: Location,
    /// The stop where we boarded on the trip to get here
    /// These are only None when this is our departure station
    departure_stop: Option<Location>,
    departure_time: Option<u32>,
    mode: Mode,
}

#[derive(Debug, Clone, Eq, PartialEq, Copy)]
pub enum Mode {
    NotApplicable,
    Trip(u32),
    Transfer,
}


#[derive(Debug, Clone, Eq, PartialEq, Copy)]
pub struct Transfer {
    pub from_stop: Location,
    pub duration: u32,
}

impl Arrival {
    pub fn new_trip(time: u32, stop: Location, departure_time: u32, departure_stop: Location, trip_id: u32) -> Self {
        Self { time, stop, departure_stop: Some(departure_stop), departure_time: Some(departure_time), mode: Mode::Trip(trip_id) }
    }

    pub fn new_transfer(time: u32, stop: Location, departure_time: u32, departure_stop: Location) -> Self {
        Self { time, stop, departure_stop: Some(departure_stop), departure_time: Some(departure_time), mode: Mode::Transfer }
    }

    pub fn departure_station(time: u32, stop: Location) -> Self {
        Self { time, stop, departure_stop: None, departure_time: None, mode: NotApplicable }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
pub struct Connection {
    pub departure_station: Location,
    pub arrival_station: Location,
    pub departure: u32,
    pub arrival: u32,
    pub trip_id: u32,
}

impl Connection {
    pub async fn arrival_name(&self, db: &impl Executor) -> anyhow::Result<String> {
        get_stop_readable(&self.arrival_station.stop_id, db).await
    }
    pub async fn departure_name(&self, db: &impl Executor) -> anyhow::Result<String> {
        get_stop_readable(&self.departure_station.stop_id, db).await
    }
    pub async fn route_information(&self, db: &impl Executor) -> anyhow::Result<String> {
        let trip = Trip::select_by_id(db, &self.trip_id).await?
            .ok_or(anyhow::Error::msg("Cannot get trip"))?;
        let route = Route::select_by_id(db, &trip.route_id).await?
            .ok_or(anyhow::Error::msg("Cannot get route"))?;
        Ok(format!("{} {} {}", route.agency_id, route.route_short_name, route.route_long_name))
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
/// Note: while this is the same concept as a GTFS route, they are not equal.
/// In the OvApi GTFS set, most routes only have one trip.
/// In order to efficiently use Raptor, trips with the same stops should be
/// combined in the same routes.
pub struct RRoute {
    stops: Vec<Location>,
    connections: Vec<Vec<Connection>>,
}

impl RRoute {
    pub fn new(stops: Vec<Location>) -> Self {
        Self {
            stops,
            connections: Vec::new(),
        }
    }

    pub fn is_before(&self, p1: u32, p2: u32) -> bool {
        for stop in &self.stops {
            if stop.parent_id == p1 {
                return true;
            } else if stop.parent_id == p2 {
                return false;
            }
        }

        panic!("Stops {p1} and {p2} not in this route")
    }

    pub fn stops_from(&self, p: &u32) -> Vec<(usize, Location)> {
        // TODO try not to clone
        for (i, stop) in self.stops.iter().enumerate() {
            if stop.parent_id == *p {
                return self.stops[i..].iter().cloned()
                    .zip(i..)
                    .map(|(item, i)| (i, item))
                    .collect();
            }
        }

        panic!("Stop {p} not in this route")
    }

    pub fn trip_from_station(&self, parent_id: u32, start_time: u32) -> Option<&[Connection]> {
        let index = self.stops.iter().enumerate()
            .find(|&(_, stop)| stop.parent_id == parent_id)
            .map(|(i, _)| i)
            .expect(&format!("Stop {parent_id} not in this route"));

        self.trip_from(index, start_time)
    }

    pub fn trip_from(&self, stop_index: usize, start_time: u32) -> Option<&[Connection]> {
        for trips in &self.connections {
            if trips[stop_index].departure >= start_time {
                return Some(&trips[stop_index..]);
            }
        }

        None
    }

    pub fn contains_station(&self, stop_id: &u32) -> bool {
        self.stops.iter().map(|l| l.parent_id)
            .any(|s| s == *stop_id)
    }
}

const PAGE_SIZE: u64 = 1_000_000;

async fn generate_timetable(db: &RBatis) -> anyhow::Result<HashMap<u32, RRoute>> {
    debug!("Getting trips, routes and stations...");
    let parent_stations = get_parent_station_map(db).await?;
    let total_count = count_stop_times(db).await?;

    debug("Generating timetable from stop_times, this will take a while...");
    let mut routes = HashMap::new(); // [stop_id] -> RRoute
    let bar = ProgressBar::new(total_count);
    bar.set_style(ProgressStyle::default_bar()
        .template("{wide_bar} Elapsed: {elapsed_precise}, ETA: {eta_precise}")?);

    let mut highest_id = 0;
    loop {
        let stop_times = get_stop_times(highest_id, PAGE_SIZE, db).await?;
        let count = stop_times.len();

        let stop_connections = stop_times.windows(2);

        let mut current_trip_stops = Vec::new();
        let mut current_trip_connections = Vec::new();

        for window in stop_connections {
            if let [dep_stop, arr_stop] = window {
                if dep_stop.trip_id == arr_stop.trip_id {
                    let dep_parent_station = parent_stations.get(&dep_stop.stop_id)
                        .ok_or(anyhow::Error::msg("Parent station not found"))?;
                    let arr_parent_station = parent_stations.get(&arr_stop.stop_id)
                        .ok_or(anyhow::Error::msg("Parent station not found"))?;

                    // Use parent station for trip "identifier"
                    current_trip_stops.push(Location { stop_id: dep_stop.stop_id, parent_id: *dep_parent_station });
                    let connection = Connection {
                        departure_station: Location { stop_id: dep_stop.stop_id, parent_id: *dep_parent_station },
                        arrival_station: Location { stop_id: arr_stop.stop_id, parent_id: *arr_parent_station },
                        departure: dep_stop.departure_time.into(),
                        arrival: arr_stop.arrival_time.into(),
                        trip_id: dep_stop.trip_id,
                    };
                    current_trip_connections.push(connection);
                } else {
                    // Reset temporary storages and write values to result
                    // This if statement does not trigger at the very last iteration
                    // That's why we also write this at count != PAGE_SIZE below.
                    // TODO clone
                    let route = routes.entry(current_trip_stops.clone())
                        .or_insert(RRoute::new(current_trip_stops));
                    route.connections.push(current_trip_connections);

                    current_trip_stops = Vec::new();
                    current_trip_connections = Vec::new();
                }
            }
        }

        bar.inc(count as u64);
        if count != PAGE_SIZE as usize {
            let route = routes.entry(current_trip_stops.clone())
                .or_insert(RRoute::new(current_trip_stops));
            route.connections.push(current_trip_connections);
            break;
        } else {
            highest_id = stop_times.last().unwrap().id.unwrap();
        }
    }
    bar.finish();

    // Change key type to be an integer
    let routes = routes.into_iter()
        .enumerate()
        .map(|(i, (_, v))| { (i as u32, v) })
        .collect();

    Ok(routes)
}

pub async fn generate_transfer_times(db: &RBatis) -> anyhow::Result<HashMap<u32, u32>> {
    let transfers = get_transfer_times(db).await?;
    Ok(transfers)
}

const TIMETABLE: &str = "raptor_timetable.blob";

// TODO make generic
pub async fn get_timetable(db: &RBatis, cache: bool) -> anyhow::Result<HashMap<u32, RRoute>> {
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

const MAX_K: usize = 8;

/// Source: https://www.microsoft.com/en-us/research/wp-content/uploads/2012/01/raptor_alenex.pdf
pub async fn run_raptor<'a>(
    departure_stop: u32,
    arrival_stop: u32,
    departure_time: impl Into<u32>,
    timetable: &'a HashMap<u32, RRoute>,
    transfer_times: &'a HashMap<u32, u32>,
    db: &impl Executor,
) -> anyhow::Result<Option<Vec<JourneyPart>>> {
    let departure_time = departure_time.into();
    let departure_location = Location { stop_id: departure_stop, parent_id: departure_stop };

    // The algorithm associates with each stop p a multilabel (τ0(p), τ1(p), ..  , τK (p)),
    // where τi(p) represents the earliest known arrival time at p with/ up to i trips.
    // All values in all labels are initialized to ∞.
    let mut tau_k = vec![HashMap::new(); MAX_K + 1];
    // We then set τ0(ps) = τ .
    tau_k[0].insert(departure_stop, Arrival::departure_station(departure_time, departure_location));
    // Another useful technique is local pruning. For each stop pi, we keep a value τ ∗(pi)
    // representing the earliest known arrival time at pi.
    let mut tau_star = tau_k[0].clone();

    // Map to keep the found transfers to each trip/stop
    let mut transfers = HashMap::new();

    let mut marked = HashSet::new();
    marked.insert(departure_location);

    for k in 1..=MAX_K {
        // The first stage of round k sets τk(p) = τk−1(p) for all stops p:
        // this sets an upper bound on the earliest arrival time at p with at most k trips.
        // Note that local pruning allows us to drop the first stage (copying the labels
        // from the previous round), since τ ∗(pi) automatically keeps track of the earliest
        // possible time to get to pi.

        // [Section 3:]
        // The second stage then processes each route in the timetable exactly once.
        // Consider a route r, and let T(r) = (t0, t1, ... , t|T (r)|−1) be the sequence
        // of trips that follow route r, from earliest to latest.
        // When processing route r, we consider journeys where the last (k’th) trip taken
        // is in route r.
        // [Section 3.1:]
        // More precisely, during round k, it suffices to traverse only routes that
        // contain at least one stop reached with exactly k − 1 trips.
        // To implement this improved version of the algorithm, we mark during round k − 1
        // those stops pi for which we improved the arrival time τk−1(pi). At the beginning
        // of round k, we loop through all marked stops to find all routes that contain them.
        // Only routes from the resulting set Q are considered for scanning in round k.
        let mut q: HashMap<u32, Location> = HashMap::new();
        for p in &marked {
            for (&route_id, r) in timetable {
                if !r.contains_station(&p.parent_id) {
                    continue;
                }

                // Moreover, since the marked stops are exactly those where we potentially
                // “hop on” a trip in round k, we only have to traverse a route beginning
                // at the earliest marked stop it contains. To enable this, while adding
                // routes to Q, we also remember the earliest marked stop in each route.
                // See also Figure 1.

                // if (r, p′) ∈ Q for some stop p′ then
                if let Some(&p_prime) = q.get(&route_id) {
                    // Substitute (r, p′) by (r, p) in Q if p comes before p′ in r
                    if r.is_before(p.parent_id, p_prime.parent_id) {
                        q.insert(route_id, p_prime);
                    }
                } else {
                    // Add (r, p) to Q
                    q.insert(route_id, *p);
                }
            }
        }

        // unmark p, for each marked stop p
        marked = HashSet::new();

        for (route_id, p) in q {
            let r = timetable.get(&route_id).expect("Route id from Q should be in timetable");

            // t ← ⊥ // the current trip
            let mut t: Option<&[Connection]> = None;

            for (i, p_i) in r.stops.iter().enumerate() {
                // Let et(r, pi) be the earliest trip in route r that one can catch at stop pi,
                // i. e., the earliest trip t such that τdep(t, pi) ≥ τk−1(pi).
                // (Note that this trip may not exist, in which case et(r, pi) is undefined.)
                // To process the route, we visit its stops in order until we find a stop pi
                // such that et(r, pi) is defined
                // Moreover, we may need to update the current trip for k:
                // at each stop pi along r it may be possible to catch an
                // earlier trip (because a quicker path to pi has been found
                // in a previous round). Thus, we have to check if
                // τk−1(pi) < τarr(t, pi) and update t by recomputing et(r, pi).
                if let Some(possible_arrival) = tau_k[k - 1].get(&p_i.parent_id) {
                    if let Some(new_trip) = r.trip_from_station(p_i.parent_id, possible_arrival.time) {
                        // Determine transfer information (if we are on a trip, so not for departure)
                        let transfer_time = if t.is_some() {
                            transfer_times.get(&p_i.stop_id).map(|t| t * 60)
                        } else { None };
                        if t.is_none() || (i < t.unwrap().len() && possible_arrival.time + transfer_time.unwrap() < t.unwrap()[i].departure) {
                            if let Some(transfer_time) = transfer_time {
                                // Save transfer here (using stop_id (= platform) instead of parent_id)
                                // Note: possible_arrival = the arrival from the trip using which we  are here earliest.
                                //  and: new_trip[0] = the connection which we are transferring to
                                transfers.insert(possible_arrival.stop.parent_id, Transfer{duration: transfer_time, from_stop: possible_arrival.stop});
                            }
                            t = Some(new_trip);
                        }
                    }
                }
                // Can the label be improved in this round? Includes local and target pruning
                // For each subsequent stop pj, we can update τk(pj) using
                // this trip. To reconstruct the journey, we set a parent
                // pointer to the stop at which t was boarded.
                if let Some(t) = t {
                    // p_h = the stop at which we "hop on" the trip
                    let p_h = &t[0].departure_station;
                    let departure = t[0].departure;

                    if let Some(connection) = t.get(i) {
                        let p_j = connection.arrival_station;
                        let earliest_at_pj = tau_star.get(&p_j.parent_id)
                            .map(|a| a.time).unwrap_or(u32::MAX);
                        let earliest_at_arr = tau_star.get(&arrival_stop)
                            .map(|a| a.time).unwrap_or(u32::MAX);
                        if connection.arrival < min(earliest_at_pj, earliest_at_arr) {
                            if let Some(transfer) = transfers.get(&p_h.parent_id) {
                                let transfer_arrival = Arrival::new_transfer(
                                    departure + transfer.duration, *p_h,
                                    departure, transfer.from_stop,
                                );
                                tau_k[k].insert(p_h.stop_id, transfer_arrival);
                                tau_star.insert(p_h.stop_id, transfer_arrival);
                            }
                            let arrival = Arrival::new_trip(
                                connection.arrival, p_j,
                                departure, *p_h, connection.trip_id,
                            );
                            tau_k[k].insert(p_j.parent_id, arrival);
                            tau_star.insert(p_j.parent_id, arrival);
                            marked.insert(p_j);
                        }
                    }
                }
            }
        }
        // TODO document this feature
        if env::var("SHOW_DOTS").is_ok_and(|v|v == "1") {
            visualise_earliest_arrivals(&tau_star, k, arrival_stop, db).await?;
        }

        // If no new stops are marked, the route cannot be improved
        if marked.is_empty() { break; }
    }

    // Construct the path, starting at the arrival
    let mut path = Vec::new();
    let mut current_stop = arrival_stop;
    let mut seen = HashSet::new();
    while let Some(arrival) = tau_star.get(&current_stop) {
        // path.push(JourneyPart::Station(arrival.stop));
        match arrival.mode {
            Mode::Trip(trip_id) => {
                // Assume that departure_stop and departure_time are Some, if mode is Trip
                let leg = TripLeg {
                    from_stop: arrival.departure_stop.unwrap(),
                    to_stop: arrival.stop,
                    departure: arrival.departure_time.unwrap(),
                    arrival: arrival.time,
                    trip_id,
                };
                path.push(JourneyPart::Vehicle(leg));
            }
            Mode::Transfer => {
                // Assume that departure_stop and departure_time are Some, if mode is Transfer
                path.push(JourneyPart::Transfer(
                    arrival.departure_stop.unwrap().stop_id,
                    arrival.stop.stop_id,
                    arrival.time - arrival.departure_time.unwrap(),
                ));
            }
            NotApplicable => {}
        }
        if let Some(stop) = arrival.departure_stop {
            // First try stop_id to find the "transfer" connection
            current_stop = stop.stop_id;
            // If there is no associated arrival, get the parent stop, for the "vehicle" connection
            if !tau_star.contains_key(&current_stop) {
                current_stop = stop.parent_id;
            }
            let old_length = seen.len();
            seen.insert(current_stop);
            if old_length == seen.len() {
                error!("Found loop in path construction");
                break;
            }
        } else {
            if arrival.stop.parent_id != departure_stop {
                error!("Incorrect departure stop")
            }
            break;
        }
    }

    path.reverse();

    Ok(Some(path))
}

pub async fn print_result(result: &Vec<JourneyPart>, db: &impl Executor) -> anyhow::Result<()> {
    for part in result {
        println!("{}", part.to_string(db).await?);
    }

    Ok(())
}

