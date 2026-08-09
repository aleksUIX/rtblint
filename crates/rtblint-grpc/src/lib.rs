//! gRPC server for rtblint.
//!
//! Serves `openadtech.rtblint.v1` over HTTP/2, with server reflection and
//! `grpc.health.v1`. The transport is a thin adapter: every RPC delegates to
//! `rtblint-core`, so the gRPC surface and the CLI cannot disagree about what a
//! payload means.
//!
//! ## On being the second one
//!
//! This crate is the sibling of `vastlint-grpc`, and building it was partly a
//! test of whether that design generalises. Two things came out of it that are
//! worth knowing before reading further.
//!
//! The contract did not mirror. Three of its decisions inverted under rtblint's
//! facts: the payload kind has to be supplied rather than inferred, the spec
//! version is a string rather than an enum, and severity has two levels rather
//! than three. Each is argued at the point of divergence in the proto file. A
//! mechanical copy would have been wrong in all three places.
//!
//! The ingress layer did mirror, and that is the problem. `limit`, `ratelimit`,
//! `metrics`, `config`, and `deadline` are near-copies of their vastlint
//! counterparts, living in a different repository with nothing keeping them in
//! sync. That is the exact drift the "one core, many surfaces" architecture
//! exists to prevent, reintroduced one layer up. See the note in
//! [`limit`] and the README.

pub mod config;
pub mod convert;
pub mod deadline;
pub mod limit;
pub mod metrics;
pub mod provenance;
pub mod ratelimit;
pub mod service;

/// Generated bindings for `openadtech.rtblint.v1`.
pub mod proto {
    tonic::include_proto!("openadtech.rtblint.v1");

    /// The encoded file descriptor set, served by reflection so that `grpcurl`
    /// can describe and call this server with no local copy of the proto.
    pub const FILE_DESCRIPTOR_SET: &[u8] =
        tonic::include_file_descriptor_set!("rtblint_descriptor");
}
