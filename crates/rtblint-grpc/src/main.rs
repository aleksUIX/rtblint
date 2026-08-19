//! rtblint gRPC server.
//!
//! ```text
//! RTBLINT_GRPC_ADDR=0.0.0.0:50061 rtblint-grpc
//! grpcurl -plaintext localhost:50061 list
//! curl localhost:9091/metrics
//! ```

use std::sync::Arc;

use mimalloc::MiMalloc;
use rtblint_grpc::config::Config;
use rtblint_grpc::limit::{AdaptiveConcurrencyLayer, AdaptiveLimiter};
use rtblint_grpc::metrics;
use rtblint_grpc::proto::rtblint_service_server::RtblintServiceServer;
use rtblint_grpc::proto::FILE_DESCRIPTOR_SET;
use rtblint_grpc::ratelimit::{RateLimitLayer, RateLimiter};
use rtblint_grpc::service::RtblintApi;
use tonic::transport::Server;

/// Validation parses JSON into an owned tree per call, so under concurrency the
/// system allocator's shared free list becomes the bottleneck first.
#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::from_env()?;

    // The runtime is built by hand so the blocking pool can be sized. That pool
    // is where validation runs, and its default ceiling of 512 threads is meant
    // for blocking I/O, where threads wait. These never wait, so the default
    // buys context switching rather than throughput and puts the real admission
    // decision somewhere nobody configured.
    let mut runtime = tokio::runtime::Builder::new_multi_thread();
    runtime
        .enable_all()
        .max_blocking_threads(config.blocking_threads);
    if let Some(threads) = config.worker_threads {
        runtime.worker_threads(threads);
    }

    runtime.build()?.block_on(serve(config))
}

async fn serve(config: Config) -> Result<(), Box<dyn std::error::Error>> {
    let limiter = Arc::new(AdaptiveLimiter::new(config.limit.clone()));
    let rate_limiter = Arc::new(RateLimiter::new(config.rate_limit.clone()));
    metrics::set_concurrency_limit(limiter.limit());

    let (health_reporter, health_service) = tonic_health::server::health_reporter();
    health_reporter
        .set_serving::<RtblintServiceServer<RtblintApi>>()
        .await;

    // The health descriptor is registered alongside rtblint's own. Reflection
    // reports only what is in its descriptor pool, so without it `grpcurl list`
    // omits grpc.health.v1.Health and calling it fails with "target server does
    // not expose service" while the service is running perfectly well. Learned
    // on the sibling server; carried over rather than rediscovered.
    let reflection_v1 = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(FILE_DESCRIPTOR_SET)
        .register_encoded_file_descriptor_set(tonic_health::pb::FILE_DESCRIPTOR_SET)
        .build_v1()?;
    let reflection_v1alpha = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(FILE_DESCRIPTOR_SET)
        .register_encoded_file_descriptor_set(tonic_health::pb::FILE_DESCRIPTOR_SET)
        .build_v1alpha()?;

    if let Some(metrics_addr) = config.metrics_addr {
        tokio::spawn(async move {
            if let Err(error) = metrics::serve(metrics_addr).await {
                eprintln!("metrics endpoint stopped: {}", error);
            }
        });
        eprintln!("metrics on http://{metrics_addr}/metrics");
    }

    banner(&config, &limiter);

    let rtblint = RtblintServiceServer::new(RtblintApi::new())
        .max_decoding_message_size(config.max_message_bytes);

    Server::builder()
        // Rate limiting sits outside the concurrency limiter, so a caller over
        // its allowance is refused before it can occupy a concurrency slot.
        .layer(RateLimitLayer::new(
            Arc::clone(&rate_limiter),
            config.caller_header.clone(),
        ))
        .layer(AdaptiveConcurrencyLayer::new(Arc::clone(&limiter)))
        .add_service(health_service)
        .add_service(reflection_v1)
        .add_service(reflection_v1alpha)
        .add_service(rtblint)
        .serve_with_shutdown(config.addr, shutdown())
        .await?;

    Ok(())
}

/// Prints the effective configuration once, so a latency graph can be matched
/// to the settings that produced it.
fn banner(config: &Config, limiter: &AdaptiveLimiter) {
    eprintln!(
        "rtblint-grpc {} listening on {}",
        env!("CARGO_PKG_VERSION"),
        config.addr
    );
    eprintln!("catalog {}", rtblint_grpc::provenance::catalog_digest());

    if config.limit.enabled {
        eprintln!(
            "concurrency limit: adaptive, starting at {} in [{}, {}], target latency {:?}, backoff {}",
            limiter.limit(),
            config.limit.min,
            config.limit.max,
            config.limit.target_latency,
            config.limit.backoff_ratio,
        );
    } else {
        eprintln!("concurrency limit: DISABLED, no shedding");
    }

    if config.rate_limit.per_second > 0 {
        eprintln!(
            "rate limit: {}/s per caller, burst {}, identified by {}",
            config.rate_limit.per_second, config.rate_limit.burst, config.caller_header,
        );
    } else {
        eprintln!("rate limit: disabled");
    }

    eprintln!("max message size: {} bytes", config.max_message_bytes);
    eprintln!(
        "validation threads: {} ({} async workers)",
        config.blocking_threads,
        config
            .worker_threads
            .map(|threads| threads.to_string())
            .unwrap_or_else(|| "default".to_string())
    );
}

/// Stops accepting on SIGINT so in-flight requests finish.
async fn shutdown() {
    let _ = tokio::signal::ctrl_c().await;
    eprintln!("shutting down");
}
