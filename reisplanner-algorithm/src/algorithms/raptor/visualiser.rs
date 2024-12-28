use crate::algorithms::raptor::Arrival;
use crate::getters::get_stop_readable;
use crate::utils::seconds_to_hms;
use chrono::Local;
use dot_writer::{Attributes, Color, DotWriter, Style};
use rbatis::executor::Executor;
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::process::Command;

pub async fn visualise_earliest_arrivals(
    tau: &HashMap<u32, Arrival>,
    k: usize,
    destination: u32,
    db: &impl Executor,
) -> anyhow::Result<()> {
    let mut output_bytes = Vec::new();
    {
        let mut writer = DotWriter::from(&mut output_bytes);

        let mut digraph = writer.digraph();

        for (_, arrival) in tau.iter() {
            let name = get_stop_readable(&arrival.stop.parent_id, db).await?;
            if arrival.departure_stop.is_some() {
                let from_station = get_stop_readable(&arrival.departure_stop.unwrap().parent_id, db).await?;
                // let route = c2.route_information(db).await?;
                let arrival = seconds_to_hms(arrival.time);

                digraph.edge(format!("\"{from_station}\""), format!("\"{name}\""))
                    .attributes().set_label(&arrival.to_string());
            } else {
                digraph.node_named(format!("\"{name}\""))
                    .set_color(Color::Red).set_style(Style::Filled);
            }
        }
        
        let destination_name = get_stop_readable(&destination, db).await?;
        digraph.node_named(format!("\"{destination_name}\""))
            .set_color(Color::PaleGreen).set_style(Style::Filled);
    }
    // Get the system's temporary directory
    let mut temp_path = std::env::temp_dir();

    // Append your custom folder and file name
    temp_path.push("reisplanner");
    let datetime_string = Local::now().format("%Y%m%d%H%M%S").to_string();
    temp_path.push(format!("graph-{datetime_string}-k={k}.dot"));

    // Ensure the directory exists
    if let Some(parent) = temp_path.parent() {
        fs::create_dir_all(parent)?;
    }

    // Write to the file
    let mut file = File::create(&temp_path)?;
    file.write_all(&output_bytes)?;

    Command::new("xdot").arg(temp_path).arg("-f").arg("fdp").spawn()?;

    Ok(())
}