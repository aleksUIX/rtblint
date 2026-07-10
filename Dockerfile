# Builds the rtblint MCP server. Glama's evaluator starts this container and
# sends an MCP `initialize` request over stdio; the binary answers, which is
# all that's needed to pass the introspection check and earn a quality score.

FROM rust:1.83-slim AS build
WORKDIR /src
COPY . .
RUN cargo build --release -p rtblint-mcp

FROM debian:bookworm-slim
COPY --from=build /src/target/release/rtblint-mcp /usr/local/bin/rtblint-mcp
# The server speaks MCP over stdio (newline-delimited JSON-RPC).
ENTRYPOINT ["rtblint-mcp"]
