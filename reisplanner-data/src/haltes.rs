use crate::download::download_html;
use crate::types::{HaltesExport, PlaceTransfer, StopPlace};
use anyhow::{anyhow, bail, Context};
use rbatis::executor::Executor;
use regex::Regex;
use reisplanner_gtfs::gtfs::types::Stop;
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use tracing::warn;

const HALTES_URL: &str = "https://data.ndovloket.nl/haltes/";
const HALTES_REGEX: &str = r#"href="(ExportCHB\d+\.xml\.gz)""#;

pub async fn get_haltes_url() -> anyhow::Result<String> {
    let haltes_parent = download_html(HALTES_URL).await?;
    let haltes_re = Regex::new(HALTES_REGEX)?;
    let haltes_url = haltes_re
        .captures(&haltes_parent)
        .ok_or(anyhow!("Could not find haltes url on page"))?
        .get(1)
        .ok_or(anyhow!("Could not find haltes url on page"))?
        .as_str();
    let haltes_url = format!("{HALTES_URL}/{haltes_url}");

    Ok(haltes_url)
}

pub async fn parse_haltes(
    path: &PathBuf,
    db: &impl Executor,
) -> anyhow::Result<Vec<PlaceTransfer>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let export: HaltesExport = quick_xml::de::from_reader(reader)?;

    let mut place_transfers = Vec::new();
    let place_iter = export
        .stopplaces
        .stopplaces
        .iter()
        .filter(|p| p.placecode.is_some());
    for place in place_iter {
        let entries = parse_place(place, db).await?;
        place_transfers.extend(entries);
    }

    Ok(place_transfers)
}

/// Note: assumes that `place.placecode.is_some()`
async fn parse_place(place: &StopPlace, db: &impl Executor) -> anyhow::Result<Vec<PlaceTransfer>> {
    let stop_ids: Vec<String> = match place.stopplacetype.as_str() {
        "onstreetBus" | "onstreetTram" | "combiTramBus" | "metroStation" | "combiMetroTram"
        | "ferryPort" | "other" | "busStation" | "tramStation" => {
            let stop_codes: Vec<_> = place
                .quays
                .quays
                .iter()
                .map(|q| q.id.split(":").last())
                .map(|id| id.map(|id| id.to_string()))
                .map(|id| id.ok_or(anyhow!("Cannot parse quay id")))
                .collect::<Result<_, _>>()?;

            let mut ids = Vec::new();
            for code in stop_codes {
                if let Some(stop) = Stop::select_by_code(db, &code).await?.first() {
                    ids.push(stop.stop_id.clone());
                } else {
                    warn!("{} with stop_code {} not found", place.stopplacetype, code)
                }
            }
            ids
        }
        "railStation" => {
            let code = place
                .stopplacecode
                .split(":")
                .last()
                .ok_or(anyhow!("Cannot parse stop place code"))?;
            let zone_id = format!("IFF:{code}");
            Stop::select_by_zone_id(db, &zone_id)
                .await?
                .into_iter()
                .map(|stop| stop.stop_id)
                .collect()
        }
        other => {
            return Err(anyhow!("Unknown stopplacetype: {other}"));
        }
    };

    let transfers = stop_ids
        .iter()
        .map(|id| PlaceTransfer {
            code: place.placecode.clone().unwrap(),
            stop_id: id.to_string(),
        })
        .collect();

    Ok(transfers)
}
