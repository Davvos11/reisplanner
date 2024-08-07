use vergen::{CargoBuilder, Emitter};

fn main() {
    protobuf_codegen::Codegen::new()
        .out_dir("src/gtfs_realtime")
        .inputs(["proto/gtfs-realtime.proto", "proto/gtfs-realtime-ov-api.proto"])
        .include("proto")
        .run()
        .expect("protoc");
    
    
    let cargo = CargoBuilder::all_cargo().unwrap();
    Emitter::default()
        .add_instructions(&cargo).unwrap()
        .emit().unwrap();
}
