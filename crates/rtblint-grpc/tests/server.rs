//! End-to-end tests over a real client and a real socket.
//!
//! The unit tests in `service.rs` call the trait directly, which skips
//! encoding, HTTP/2 framing, and the metadata path. These do not.

use std::net::SocketAddr;
use std::time::Duration;

use rtblint_grpc::proto::rtblint_service_client::RtblintServiceClient;
use rtblint_grpc::proto::rtblint_service_server::RtblintServiceServer;
use rtblint_grpc::proto::{
    JsonDialect, ListVersionsRequest, PayloadKind, Severity, ValidateArtfMutationsRequest,
    ValidatePairRequest, ValidateRequest, ValidationContext,
};
use rtblint_grpc::service::RtblintApi;
use tokio::net::TcpListener;
use tokio_stream::wrappers::TcpListenerStream;
use tonic::transport::{Channel, Server};
use tonic::Request;

const VALID_REQUEST: &str = r#"{"id":"r1","imp":[{"id":"i1","video":{"mimes":["video/mp4"]}}]}"#;

async fn start() -> RtblintServiceClient<Channel> {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind an ephemeral port");
    let addr: SocketAddr = listener.local_addr().expect("local addr");

    tokio::spawn(async move {
        Server::builder()
            .add_service(RtblintServiceServer::new(RtblintApi::new()))
            .serve_with_incoming(TcpListenerStream::new(listener))
            .await
            .expect("server runs");
    });

    RtblintServiceClient::connect(format!("http://{addr}"))
        .await
        .expect("client connects")
}

#[tokio::test]
async fn validate_round_trips_over_the_wire() {
    let mut client = start().await;

    let verdict = client
        .validate(Request::new(ValidateRequest {
            document: r#"{"id":"r1","imp":[{"id":"i1","video":{}}]}"#.to_string(),
            kind: PayloadKind::BidRequest as i32,
            context: None,
        }))
        .await
        .expect("validate succeeds")
        .into_inner()
        .verdict
        .expect("verdict present");

    assert!(!verdict.issues.is_empty());

    let issue = &verdict.issues[0];
    assert!(!issue.rule_id.is_empty());
    assert!(!issue.message.is_empty());
    assert_ne!(issue.severity, Severity::Unspecified as i32);

    let provenance = verdict.provenance.expect("provenance present");
    assert!(provenance.catalog_digest.starts_with("sha256:"));
}

#[tokio::test]
async fn a_valid_bid_request_passes() {
    let mut client = start().await;

    let verdict = client
        .validate(Request::new(ValidateRequest {
            document: VALID_REQUEST.to_string(),
            kind: PayloadKind::BidRequest as i32,
            context: None,
        }))
        .await
        .expect("validate succeeds")
        .into_inner()
        .verdict
        .expect("verdict present");

    assert!(
        verdict.valid,
        "unexpected findings: {:?}",
        verdict
            .issues
            .iter()
            .map(|i| &i.rule_id)
            .collect::<Vec<_>>()
    );
}

/// The one thing this service asks of callers that its sibling does not.
#[tokio::test]
async fn an_unspecified_payload_kind_is_rejected() {
    let mut client = start().await;

    let status = client
        .validate(Request::new(ValidateRequest {
            document: VALID_REQUEST.to_string(),
            kind: PayloadKind::Unspecified as i32,
            context: None,
        }))
        .await
        .expect_err("kind is required");

    assert_eq!(status.code(), tonic::Code::InvalidArgument);
    assert!(status.message().contains("PAYLOAD_KIND_BID_REQUEST"));
}

#[tokio::test]
async fn an_unknown_version_is_rejected_and_lists_what_is_available() {
    let mut client = start().await;

    let status = client
        .validate(Request::new(ValidateRequest {
            document: VALID_REQUEST.to_string(),
            kind: PayloadKind::BidRequest as i32,
            context: Some(ValidationContext {
                version: "2.6-209901".to_string(),
                dialect: JsonDialect::Unspecified as i32,
            }),
        }))
        .await
        .expect_err("unknown version is rejected");

    assert_eq!(status.code(), tonic::Code::InvalidArgument);
    assert!(status.message().contains("2.6-202606"));
}

/// The version string is what a caller sends, so the round trip through
/// encoding is exactly where a mistake would hide.
#[tokio::test]
async fn every_advertised_version_is_actually_usable() {
    let mut client = start().await;

    let versions = client
        .list_versions(Request::new(ListVersionsRequest {}))
        .await
        .expect("list_versions succeeds")
        .into_inner()
        .versions;

    assert!(!versions.is_empty());

    for version in &versions {
        let verdict = client
            .validate(Request::new(ValidateRequest {
                document: VALID_REQUEST.to_string(),
                kind: PayloadKind::BidRequest as i32,
                context: Some(ValidationContext {
                    version: version.id.clone(),
                    dialect: JsonDialect::Unspecified as i32,
                }),
            }))
            .await
            .unwrap_or_else(|error| {
                panic!("advertised version {} was rejected: {error}", version.id)
            })
            .into_inner()
            .verdict
            .expect("verdict present");

        assert_eq!(
            verdict.effective_version, version.id,
            "the verdict must name the version it actually used"
        );
    }
}

#[tokio::test]
async fn validate_pair_finds_what_a_single_payload_check_cannot() {
    let mut client = start().await;

    let response = r#"{"id":"r1","seatbid":[{"bid":[{"id":"b1","impid":"nope","price":1.0}]}]}"#;

    let paired = client
        .validate_pair(Request::new(ValidatePairRequest {
            bid_request: VALID_REQUEST.to_string(),
            bid_response: response.to_string(),
            context: None,
        }))
        .await
        .expect("validate_pair succeeds")
        .into_inner()
        .verdict
        .expect("verdict present");

    assert!(!paired.valid, "a bid on an impid the request never offered");
}

/// ARTF mandates gRPC for the extension point, so the payload an orchestrator
/// checks arrives over the wire. The rewritten bid request has to survive the
/// round trip intact, since it is what the orchestrator forwards next.
#[tokio::test]
async fn applying_artf_mutations_round_trips_the_rewritten_payload() {
    let mut client = start().await;

    let envelope = r#"{
        "id": "ep-1",
        "tmax": 120,
        "lifecycle": "LIFECYCLE_PUBLISHER_BID_REQUEST",
        "applicable_intents": ["ACTIVATE_DEALS"],
        "originator": { "type": "TYPE_EXCHANGE", "id": "x-1" },
        "bid_request": {
            "id": "auction-1",
            "imp": [{ "id": "imp-1", "banner": { "w": 300, "h": 250 } }],
            "site": { "id": "s-1", "domain": "news.example" }
        }
    }"#;
    let mutations = r#"{
        "id": "ep-1",
        "mutations": [
            {
                "intent": "ACTIVATE_DEALS",
                "op": "OPERATION_ADD",
                "path": "/imp/imp-1",
                "ids": { "id": ["deal-curated"] }
            }
        ],
        "metadata": { "api_version": "1.0.0", "model_version": "m" }
    }"#;

    let response = client
        .validate_artf_mutations(Request::new(ValidateArtfMutationsRequest {
            rtb_request: envelope.to_string(),
            rtb_response: mutations.to_string(),
            apply: true,
            context: None,
        }))
        .await
        .expect("validate_artf_mutations succeeds")
        .into_inner();

    assert!(response.verdict.expect("verdict present").valid);

    let application = response.application.expect("application present");
    assert_eq!(application.applied, vec![0]);

    let rewritten: serde_json::Value =
        serde_json::from_str(&application.bid_request).expect("the rewritten request is JSON");
    assert_eq!(
        rewritten["imp"][0]["pmp"]["deals"][0]["id"], "deal-curated",
        "the curated deal should have survived the round trip"
    );
}

#[tokio::test]
async fn a_client_deadline_reaches_the_server() {
    let mut client = start().await;

    let mut request = Request::new(ValidateRequest {
        document: VALID_REQUEST.to_string(),
        kind: PayloadKind::BidRequest as i32,
        context: None,
    });
    request.set_timeout(Duration::from_secs(30));

    assert!(client.validate(request).await.is_ok());
}
