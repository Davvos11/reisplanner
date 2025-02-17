use async_trait::async_trait;
use rbatis::{impl_select, impl_update, rbdc};
use rbatis::executor::Executor;
use rbatis::rbdc::{Date, Error};
use rbatis::rbdc::db::ExecResult;
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

use crate::crud_trait;
use crate::rbatis_wrapper::DatabaseModel;
use crate::utils::{deserialize_date, deserialize_time_tuple, TimeTuple};

// Struct for agency.txt
#[derive(Debug, Deserialize, Serialize, Default)]
pub struct Agency {
    pub agency_id: String,
    pub agency_name: String,
    pub agency_url: String,
    pub agency_timezone: String,
    pub agency_phone: String,
}
crud_trait!(Agency {});

#[derive(Deserialize_repr, Serialize_repr, PartialEq, Debug, Default)]
#[repr(u8)]
pub enum ExceptionType {
    #[default]
    Added = 1,
    Removed = 2,
}

// Struct for calendar_dates.txt
#[derive(Debug, Deserialize, Serialize, Default)]
pub struct CalendarDate {
    pub service_id: u32,
    #[serde(deserialize_with = "deserialize_date")]
    pub date: Date,
    pub exception_type: ExceptionType,
}
crud_trait!(CalendarDate {});

// Struct for feed_info.txt
#[derive(Debug, Deserialize, Serialize, Default)]
pub struct FeedInfo {
    pub feed_publisher_name: String,
    pub feed_id: String,
    pub feed_publisher_url: String,
    pub feed_lang: String,
    #[serde(deserialize_with = "deserialize_date")]
    pub feed_start_date: Date,
    #[serde(deserialize_with = "deserialize_date")]
    pub feed_end_date: Date,
    pub feed_version: String,
}
crud_trait!(FeedInfo {});

#[derive(Deserialize_repr, Serialize_repr, PartialEq, Debug, Default, Clone)]
#[repr(u8)]
pub enum RouteType {
    #[default]
    Tram = 0,
    Metro = 1,
    Train = 2,
    Bus = 3,
    Ferry = 4,
    CableTram = 5,
    Lift = 6,
    Funicular = 7,
    TrolleyBus = 11,
    MonoRail = 12,
}

// Struct for routes.txt
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Route {
    pub route_id: u32,
    pub agency_id: String,
    pub route_short_name: String,
    pub route_long_name: String,
    pub route_desc: Option<String>,
    pub route_type: RouteType,
    pub route_color: Option<String>,
    pub route_text_color: Option<String>,
    pub route_url: Option<String>,
}

impl Default for Route {
    fn default() -> Self {
        Self {
            route_id: Default::default(),
            agency_id: Default::default(),
            route_short_name: Default::default(),
            route_long_name: Default::default(),
            route_desc: Some(Default::default()),
            route_type: Default::default(),
            route_color: Some(Default::default()),
            route_text_color: Some(Default::default()),
            route_url: Some(Default::default()),
        }
    }
}

crud_trait!(Route {});
impl_select!(Route {
    select_by_id(route_id:&u32) -> Option => "`where route_id = #{route_id}"
});

// Struct for shapes.txt
#[derive(Debug, Deserialize, Serialize)]
pub struct Shape {
    pub shape_id: u32,
    pub shape_pt_sequence: u32,
    pub shape_pt_lat: f64,
    pub shape_pt_lon: f64,
    pub shape_dist_traveled: Option<f64>,
}

impl Default for Shape {
    fn default() -> Self {
        Self {
            shape_id: Default::default(),
            shape_pt_sequence: Default::default(),
            shape_pt_lat: Default::default(),
            shape_pt_lon: Default::default(),
            shape_dist_traveled: Some(Default::default()),
        }
    }
}

crud_trait!(Shape {});


#[derive(Deserialize_repr, Serialize_repr, Default, PartialEq, Debug, Clone)]
#[repr(u8)]
pub enum LocationType {
    #[default]
    Stop = 0, // (Platform when defined within a parent_station)
    Station = 1,
    Entrance = 2,
    GenericNode = 3,
    BoardingArea = 4,
}


#[derive(Deserialize_repr, Serialize_repr, Default, PartialEq, Debug, Clone)]
#[repr(u8)]
pub enum WheelchairBoarding {
    #[default]
    Empty = 0,
    SomePossible = 1,
    NotPossible = 2,
}

// Struct for stops.txt
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Stop {
    pub stop_id: String,
    pub stop_code: Option<String>,
    pub stop_name: String,
    pub stop_lat: f64,
    pub stop_lon: f64,
    pub location_type: LocationType,
    pub parent_station: Option<String>,
    pub stop_timezone: Option<String>,
    pub wheelchair_boarding: Option<WheelchairBoarding>,
    pub platform_code: Option<String>,
    pub zone_id: Option<String>,
}

impl Default for Stop {
    fn default() -> Self {
        Self {
            stop_id: Default::default(),
            stop_code: Some(Default::default()),
            stop_name: Default::default(),
            stop_lat: Default::default(),
            stop_lon: Default::default(),
            location_type: Default::default(),
            parent_station: Some(Default::default()),
            stop_timezone: Some(Default::default()),
            wheelchair_boarding: Some(Default::default()),
            platform_code: Some(Default::default()),
            zone_id: Some(Default::default()),
        }
    }
}

crud_trait!(Stop {});
impl_select!(Stop {
    select_by_id(stop_id: &u32) -> Option => "`where stop_id = #{stop_id}`"
});

impl_select!(Stop {
    select_by_id_str(stop_id: &String) -> Option => "`where stop_id = #{stop_id}`"
});


impl_select!(Stop {
    select_by_zone_id(zone_id: &String) -> Vec => "`where zone_id = #{zone_id}`"
});

impl_select!(Stop {
    select_by_code(stop_code: &String) -> Vec => "`where stop_code = #{stop_code}`"
});

#[derive(Deserialize_repr, Serialize_repr, Default, PartialEq, Debug, Clone)]
#[repr(u8)]
pub enum PickupType {
    #[default]
    Regular = 0,
    NotAvailable = 1,
    MustPhone = 2,
    MustCoordinate = 3,
    // pickup_type=0 forbidden if start_pickup_drop_off_window or end_pickup_drop_off_window are defined.
    // pickup_type=3 forbidden if start_pickup_drop_off_window or end_pickup_drop_off_window are defined.
    // drop_off_type=0 forbidden if start_pickup_drop_off_window or end_pickup_drop_off_window are defined.
}

// Struct for stop_times.txt
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct StopTime {
    pub trip_id: u32,
    pub stop_sequence: u32,
    pub stop_id: u32,
    pub stop_headsign: Option<String>,
    #[serde(deserialize_with = "deserialize_time_tuple")]
    pub arrival_time: TimeTuple,
    #[serde(deserialize_with = "deserialize_time_tuple")]
    pub departure_time: TimeTuple,
    pub pickup_type: PickupType,
    pub drop_off_type: PickupType,
    pub timepoint: i32,
    pub shape_dist_traveled: Option<f64>,
    pub fare_units_traveled: Option<i32>,
    // Field added after sorting
    #[serde(default)]
    pub id: Option<i32>,
    // Fields added by realtime updates
    #[serde(default)]
    pub arrival_delay: Option<i32>,
    #[serde(default)]
    pub departure_delay: Option<i32>,
}

impl Default for StopTime {
    fn default() -> Self {
        Self {
            trip_id: Default::default(),
            stop_sequence: Default::default(),
            stop_id: Default::default(),
            stop_headsign: Some(Default::default()),
            arrival_time: Default::default(),
            departure_time: Default::default(),
            pickup_type: Default::default(),
            drop_off_type: Default::default(),
            timepoint: Default::default(),
            shape_dist_traveled: Some(Default::default()),
            fare_units_traveled: Some(Default::default()),
            id: Some(Default::default()),
            arrival_delay: Some(Default::default()),
            departure_delay: Some(Default::default()),
        }
    }
}

crud_trait!(StopTime {});
impl_select!(StopTime {
   select_all_grouped() => "`order by trip_id, stop_sequence`"
});
impl_select!(StopTime {
   select_all_grouped_filter(trip_ids: &[&u32])  =>
    "` where trip_id in (`
          trim ',': for _,item in trip_ids:
             #{item},
      `) order by trip_id, stop_sequence`"
});
impl_select!(StopTime {
    select_by_id_and_trip(stop_id:&u32,trip_id:&u32) => "`where stop_id = #{stop_id} and trip_id = #{trip_id}`"
});
impl_select!(StopTime {
    select_by_trip_id(trip_id:&u32) => "`where trip_id = #{trip_id} order by stop_sequence`"
});
impl_select!(StopTime {
    select_by_sequence_and_trip(stop_sequence:&u32,trip_id:&u32) => "`where stop_sequence = #{stop_sequence} and trip_id = #{trip_id}`"
});
impl_update!(StopTime {
    update_by_id_and_trip(stop_id:&u32,trip_id:&u32) => "`where stop_id = #{stop_id} and trip_id = #{trip_id}`"
});


#[derive(Deserialize_repr, Serialize_repr, Default, PartialEq, Debug)]
#[repr(u8)]
pub enum TransferType {
    #[default]
    Recommended = 0,
    Timed = 1, // Departing vehicle is expected to wait
    MinimumTimeRequired = 2, // Time required is specified in min_transfer_time
    NotPossible = 3,
    InSeatTransfer = 4,
    MustReBoard = 5,
}

// Struct for transfers.txt
#[derive(Debug, Deserialize, Serialize)]
pub struct Transfer {
    pub from_stop_id: String,
    pub to_stop_id: String,
    pub from_route_id: Option<u32>,
    pub to_route_id: Option<u32>,
    pub from_trip_id: Option<u32>,
    pub to_trip_id: Option<u32>,
    pub transfer_type: TransferType,
    pub min_transfer_time: Option<i32>,
}

impl Default for Transfer {
    fn default() -> Self {
        Self {
            from_stop_id: Default::default(),
            to_stop_id: Default::default(),
            from_route_id: Some(Default::default()),
            to_route_id: Some(Default::default()),
            from_trip_id: Some(Default::default()),
            to_trip_id: Some(Default::default()),
            transfer_type: Default::default(),
            min_transfer_time: Some(Default::default()),
        }
    }
}

crud_trait!(Transfer {});

#[derive(Deserialize_repr, Serialize_repr, Default, PartialEq, Debug, Clone)]
#[repr(u8)]
pub enum AllowedType {
    #[default]
    NoInformation = 0,
    Allowed = 1,
    NotAllowed = 2,
}


// Struct for trips.txt
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Trip {
    pub route_id: u32,
    pub service_id: u32,
    pub trip_id: u32,
    pub realtime_trip_id: String,
    pub trip_headsign: String,
    pub trip_short_name: Option<String>,
    pub trip_long_name: Option<String>,
    pub direction_id: i32,
    pub block_id: Option<String>,
    pub shape_id: Option<u32>,
    pub wheelchair_accessible: Option<AllowedType>,
    pub bikes_allowed: Option<AllowedType>,
    // Fields added by realtime updates:
    #[serde(default)]
    pub delay: Option<i32>,
}

impl Default for Trip {
    fn default() -> Self {
        Self {
            route_id: Default::default(),
            service_id: Default::default(),
            trip_id: Default::default(),
            realtime_trip_id: Default::default(),
            trip_headsign: Default::default(),
            trip_short_name: Some(Default::default()),
            trip_long_name: Some(Default::default()),
            direction_id: Default::default(),
            block_id: Some(Default::default()),
            shape_id: Some(Default::default()),
            wheelchair_accessible: Some(Default::default()),
            bikes_allowed: Some(Default::default()),
            delay: Some(Default::default()),
        }
    }
}
crud_trait!(Trip {});
impl_select!(Trip {
    select_by_id(trip_id:&u32) -> Option => "`where trip_id = #{trip_id}"
});

#[derive(Default, Debug, Deserialize, Serialize)]
pub struct LastUpdated {
    pub last_updated: rbdc::DateTime,
}
crud_trait!(LastUpdated {});