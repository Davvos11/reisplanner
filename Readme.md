# Zwiebers reisplanner
Work-in-progress journey planner for Dutch public transport.

Uses the GTFS feed from https://gtfs.ovapi.nl/nl/.


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
   - [X] CSA (proof of concept)
 - [ ] Display algorithm results
 - [ ] Display vehicle information

## Project structure

_All commands in this section should be run in this directory._

### reisplanner-gtfs
Downloads the static and realtime GTFS information from ovapi and parses it
into the sqlite database.  
This also contains the type definitions for the GTFS objects.

The programs downloads the data initially, then updates the realtime data
every minute and static data every day after 3:00 UTC.

I would not recommend running without `--release` since parsing the static GTFS
data takes a very long time otherwise.

```shell
cargo run -p reisplanner-gtfs --release
```

Note: the GTFS protobuf definitions generate Rust code.
If your IDE shows errors, try to run `cargo build` first.

### reisplanner-algorithm
Contains the journey planning algorithms.

Currently only contains a very naive CSA implementation.

```shell
cargo test -p reisplanner-algorithm --release -- --nocapture 
```