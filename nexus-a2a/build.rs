#![allow(unsafe_code, missing_docs)]

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Vendored protoc avoids requiring a system install.
    let protoc = protoc_bin_vendored::protoc_bin_path()?;
    // SAFETY: single-threaded build script.
    unsafe {
        std::env::set_var("PROTOC", protoc);
    }

    // -- Our v1.4 proto (`a2a.v1` package).
    tonic_prost_build::configure()
        .build_server(true)
        .build_client(true)
        .compile_protos(&["proto/a2a/v1/a2a.proto"], &["proto"])?;

    // -- v1.4.2 (D-V1.4-B revised): vendored upstream proto, compiled
    //    into the `a2a.upstream.v1` package so the two protos live
    //    side-by-side in Rust without module-path collisions. Pure-Rust
    //    interop tests use the generated upstream client to drive our
    //    server.
    tonic_prost_build::configure()
        .build_server(false) // client-only — we never serve under the upstream package
        .build_client(true)
        .compile_protos(
            &["vendor/a2a-upstream/a2a.v1.proto"],
            &["vendor/a2a-upstream"],
        )?;

    println!("cargo:rerun-if-changed=proto/a2a/v1/a2a.proto");
    println!("cargo:rerun-if-changed=vendor/a2a-upstream/a2a.v1.proto");
    Ok(())
}
