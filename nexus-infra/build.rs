use std::env;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let proto_file = "proto/nexus.proto";
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    // v1.1 integration: use vendored protoc so the build does not require a
    // system-installed protobuf compiler. Original overlay build did:
    //   tonic_build::configure().compile(...)
    // This `if let` keeps the existing behavior when `PROTOC` is already set
    // (e.g. on Windows / macOS CI runners with system protoc) but provides a
    // fallback for the Linux dev box that doesn't have it.
    if env::var_os("PROTOC").is_none() {
        if let Ok(p) = protoc_bin_vendored::protoc_bin_path() {
            env::set_var("PROTOC", p);
        }
    }

    tonic_build::configure()
        .build_server(true)
        .build_client(true)
        .file_descriptor_set_path(out_dir.join("nexus_descriptor.bin"))
        .compile(&[proto_file], &["proto"])?;

    // Tell cargo to recompile if the proto file changes
    println!("cargo:rerun-if-changed={}", proto_file);

    Ok(())
}
