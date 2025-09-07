fn main() {
    tonic_build::compile_protos("../nexus-infra/proto/nexus.proto")
        .unwrap_or_else(|e| panic!("Failed to compile protos: {}", e));
}
