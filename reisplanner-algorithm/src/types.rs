use crate::algorithms::raptor::{Location, TripLeg};
use crate::getters::get_stop_readable;
use crate::utils::seconds_to_hms;
use rbatis::executor::Executor;
use reisplanner_gtfs::gtfs::types::{Route, Trip};

#[derive(Clone, Debug)]
pub enum JourneyPart {
    Station(Location),
    Vehicle(TripLeg),
    // (from, to, duration)
    Transfer(u32, u32, u32),
}

impl JourneyPart {
    pub async fn to_string(&self, db: &impl Executor) -> anyhow::Result<String> {
        match self {
            JourneyPart::Station(location) => {
                get_stop_readable(&location.parent_id, db).await
            }
            JourneyPart::Vehicle(trip_leg) => {
                let dep_name = get_stop_readable(&trip_leg.from_stop.stop_id, db).await?;
                let dep_time = seconds_to_hms(trip_leg.departure);
                let arr_name = get_stop_readable(&trip_leg.to_stop.stop_id, db).await?;
                let arr_time = seconds_to_hms(trip_leg.arrival);
                let trip = Trip::select_by_id(db, &trip_leg.trip_id).await?
                    .ok_or(anyhow::Error::msg("Cannot get trip"))?;
                let route = Route::select_by_id(db, &trip.route_id).await?
                    .ok_or(anyhow::Error::msg("Cannot get route"))?;
                Ok(format!("From {dep_name} at {dep_time} to {arr_name} at {arr_time} using {} {} {}", route.agency_id, route.route_short_name, route.route_long_name))
            }
            JourneyPart::Transfer(from, to, duration) => {
                let from_name = get_stop_readable(from, db).await?;
                let to_name = get_stop_readable(to, db).await?;
                Ok(format!("Transfer from {from_name} to {to_name} ({} mins)", duration / 60))
            }
        }
    }
}