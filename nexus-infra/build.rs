use std::env;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let proto_file = "proto/nexus.proto";
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .file_descriptor_set_path(out_dir.join("nexus_descriptor.bin"))
        .compile(&[proto_file], &["proto"])?;

    // Tell cargo to recompile if the proto file changes
    println!("cargo:rerun-if-changed={}", proto_file);

    Ok(())
}
