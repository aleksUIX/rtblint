//! Generates the `openadtech.rtblint.v1` bindings and the encoded file
//! descriptor set.
//!
//! Compilation goes through `protox`, a protobuf compiler written in Rust,
//! rather than shelling out to `protoc`. The default `prost-build` path needs a
//! `protoc` binary on `PATH` at build time, which would mean a new install step
//! on every CI runner and, worse, would break `cargo install rtblint-grpc` for
//! anyone without it.

use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let proto_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../proto");
    let proto = proto_root.join("openadtech/rtblint/v1/rtblint.proto");
    let descriptor_set = PathBuf::from(std::env::var("OUT_DIR")?).join("rtblint_descriptor.bin");

    println!("cargo:rerun-if-changed={}", proto.display());

    let file_descriptors = protox::compile([&proto], [&proto_root])?;

    std::fs::write(&descriptor_set, {
        use prost::Message;
        file_descriptors.encode_to_vec()
    })?;

    tonic_prost_build::configure()
        .build_server(true)
        .build_client(true)
        .compile_fds(file_descriptors)?;

    Ok(())
}
