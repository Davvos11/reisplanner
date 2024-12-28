use crate::utils::bool_from_int;
use rbatis::{crud, impl_delete};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct StationTransfer {
    /// Station the transfer is from
    pub station_code: String,
    /// Transfer time in minutes
    pub transfer_time: u32,
}

impl From<Station> for StationTransfer {
    fn from(value: Station) -> Self {
        StationTransfer {
            station_code: value.station_abr,
            transfer_time: value.transfer_time,
        }
    }
}

crud!(StationTransfer {});
impl_delete!(StationTransfer {delete_all() => "``"});

/// Een overstapverbinding hoeft niet altijd binnen een station te liggen, maar kan ook op een (buurt)
/// station liggen. Om de overstap te overbruggen kan het mogelijk zijn gebruik te maken van een
/// alternatieve vervoersmodaliteit. Het connmode bestand bevat alle toegestane vormen van overstap
/// mogelijkheden.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ConnectionMode {
    pub con_code: u32,
    pub con_type: u32,
    pub con_mode: String,
}

/// Het bestand contconn legt de (overstap) relatie tussen twee (buurt) stations en de overstap vorm
/// (connmode) vast
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ContConnection {
    pub from_station: String,
    pub to_station: String,
    /// Transfer time in minutes
    pub transfer_time: u32,
    /// Type as `ConnectionMode.con_code`
    pub transfer_type: u32,
}

// TODO xfootnote
// TODO xchanges

/// Het bestand Stations bevat alle station gerelateerde gegevens. Het is noodzakelijk dit bestand te
/// gebruiken om de gegevens van stations in (trein) dienstregeling publicaties correct weer te geven
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Station {
    #[serde(deserialize_with = "bool_from_int")]
    pub transfer: bool,
    /// `stop_code` in GTFS `stops` table
    pub station_abr: String,
    /// Standard transfer time in minutes
    pub transfer_time: u32,
    /// Maximum transfer time in minutes
    /// (maximale overstaptijd is hier altijd gelijk aan de standaard overstaptijd;
    /// afwijkingen op de standaard overstaptijd zijn vastgelegd in: Changes (zie 5.2.3))
    /// However, I think/hope this data is already in the GTFS data?
    pub max_transfer_time: u32,
    pub country_code: String,
    pub time_zone: u32,
    _empty: String,
    pub x_coord: i32,
    pub y_coord: i32,
    pub station_name: String,
}

///////////////////////////////////////////////////////////////////////////////////////////
/// Haltes
///////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug, Deserialize)]
pub(crate) struct HaltesExport {
    pub stopplaces: StopPlaces,
}

#[derive(Debug, Deserialize)]
pub(crate) struct StopPlaces {
    #[serde(rename = "stopplace")]
    pub stopplaces: Vec<StopPlace>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct StopPlace {
    #[serde(rename = "@placecode")]
    pub placecode: Option<String>,
    pub stopplacecode: String,
    // TODO make enum
    pub stopplacetype: String,
    #[serde(default)]
    pub quays: Quays,
}

#[derive(Debug, Deserialize, Default)]
pub(crate) struct Quays {
    #[serde(rename = "quay", default)]
    pub quays: Vec<Quay>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Quay {
    #[serde(rename = "ID")]
    pub id: String,
}

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct PlaceTransfer {
    pub code: String,
    pub stop_id: String,
}

crud!(PlaceTransfer {});
impl_delete!(PlaceTransfer {delete_all() => "``"});
