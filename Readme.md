# Zwiebers reisplanner
Work-in-progress journey planner for Dutch public transport.

Uses the GTFS feed from https://gtfs.ovapi.nl/nl/.

Currently just loads GTFS data into a SQLite database.

## Current features and rough to-do
 - [ ] GTFS feed
     - [x] Download GTFS data
     - [x] Parse static GTFS data
     - [x] Save static GTFS data to database
     - [x] Parse realtime GTFS data
     - [ ] Save realtime GTFS data to database
       - [ ] Trip updates
         - [x] Delays
         - [ ] Trip descriptors
       - [ ] Vehicle positions
       - [ ] Alerts
 - [ ] Journey planning algorithm
 - [ ] Display algorithm results
 - [ ] Display vehicle information

## Project structure

### reisplanner-gtfs
Downloads the static and realtime GTFS information from ovapi and parses it
into the sqlite database.  

The programs downloads the data initially, then updates the realtime data
every minute and static data every day after 3:00 UTC.

I would not recommend running without `--release` since parsing the static GTFS
data takes a very long time otherwise.

```shell
cargo run -p reisplanner-gtfs --release
```