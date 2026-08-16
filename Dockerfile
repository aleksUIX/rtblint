# syntax=docker/dockerfile:1

# Build the RTBlint MCP server (stdio, JSON-RPC 2.0 over stdin/stdout).
# Smithery builds this image from smithery.yaml and bridges the stdio server
# to a remote endpoint. rtblint-core is pure Rust (serde only), so the build
# needs no C toolchain, and the server does no network I/O, so the runtime
# needs no CA certificates.

FROM rust:1-slim-bookworm@sha256:2775a09d208ff0d7c1f50490c45b62db929e87ba1dcbc3f2132ac71a704bcdd3 AS builder
WORKDIR /build

# Copy the workspace manifests and all member crates. cargo needs every
# workspace member's Cargo.toml to resolve the graph even though we only
# build rtblint-mcp. crates/rtblint-core/specs (read by its build.rs) rides
# along with the crates copy.
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

RUN cargo build --release -p rtblint-mcp --bin rtblint-mcp

FROM debian:bookworm-slim@sha256:abd67ffcfa541b485a3dff59865ab629aa048a6c613e639d36e7456b0b229241
COPY --from=builder /build/target/release/rtblint-mcp /usr/local/bin/rtblint-mcp
ENTRYPOINT ["/usr/local/bin/rtblint-mcp"]
