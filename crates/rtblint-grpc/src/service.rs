//! The `openadtech.rtblint.v1` service implementation.

use std::time::{Duration, Instant};

use rtblint_core as core;
use tokio::time::timeout;
use tonic::{Code, Request, Response, Status};

use crate::convert;
use crate::deadline;
use crate::metrics;
use crate::proto::rtblint_service_server::RtblintService;
use crate::proto::{
    ListVersionsRequest, ListVersionsResponse, MutationApplication, PayloadKind,
    ValidateArtfEnvelopeRequest, ValidateArtfEnvelopeResponse, ValidateArtfMutationsRequest,
    ValidateArtfMutationsResponse, ValidatePairRequest, ValidatePairResponse, ValidateRequest,
    ValidateResponse, VersionInfo,
};
use crate::provenance::provenance;

/// The service. Stateless: the catalog is static and validation carries no
/// session.
#[derive(Debug, Clone, Default)]
pub struct RtblintApi;

impl RtblintApi {
    pub fn new() -> Self {
        Self
    }
}

/// Runs CPU-bound validation without blocking the runtime, giving up when the
/// caller's deadline passes.
///
/// The same limitation as the sibling server: `spawn_blocking` cannot be
/// cancelled, so the caller gets `DEADLINE_EXCEEDED` promptly while the worker
/// runs to completion. That is the argument for a concurrency limit rather than
/// relying on deadlines to protect capacity, since a deadline stops the waiting
/// and not the working.
async fn run<T, F>(deadline: Option<Duration>, work: F) -> Result<T, Status>
where
    F: FnOnce() -> T + Send + 'static,
    T: Send + 'static,
{
    if deadline == Some(Duration::ZERO) {
        return Err(Status::deadline_exceeded(
            "deadline had already expired when the request arrived",
        ));
    }

    let handle = tokio::task::spawn_blocking(work);

    let joined = match deadline {
        Some(budget) => match timeout(budget, handle).await {
            Ok(joined) => joined,
            Err(_) => {
                return Err(Status::deadline_exceeded(
                    "validation did not complete within the caller's deadline",
                ))
            }
        },
        None => handle.await,
    };

    joined.map_err(|error| {
        Status::internal(if error.is_panic() {
            "validation worker panicked"
        } else {
            "validation worker was cancelled"
        })
    })
}

/// Times one RPC and records its outcome.
async fn observed<T, F>(method: &'static str, work: F) -> Result<Response<T>, Status>
where
    F: std::future::Future<Output = Result<Response<T>, Status>>,
{
    let started = Instant::now();
    let result = work.await;

    let status = match &result {
        Ok(_) => "ok",
        Err(status) => status_label(status.code()),
    };
    metrics::record_request(method, status, started.elapsed().as_secs_f64());

    result
}

/// Stable label for a gRPC status code. Written out rather than derived from
/// `Debug`, because a metric label is a contract with whatever dashboard reads
/// it and a formatting change upstream must not orphan a panel.
fn status_label(code: Code) -> &'static str {
    match code {
        Code::Ok => "ok",
        Code::InvalidArgument => "invalid_argument",
        Code::DeadlineExceeded => "deadline_exceeded",
        Code::ResourceExhausted => "resource_exhausted",
        Code::Internal => "internal",
        Code::Unavailable => "unavailable",
        Code::Cancelled => "cancelled",
        _ => "other",
    }
}

#[tonic::async_trait]
impl RtblintService for RtblintApi {
    async fn validate(
        &self,
        request: Request<ValidateRequest>,
    ) -> Result<Response<ValidateResponse>, Status> {
        observed("Validate", async move {
            let budget = deadline::remaining(request.metadata());
            let request = request.into_inner();
            let version = convert::version(request.context.as_ref())?;
            let dialect = convert::dialect(request.context.as_ref());
            let profile = convert::profile(request.context.as_ref())?;

            // Required, and rejected rather than guessed. A bid request and a
            // bid response are both JSON objects with an `id`, so any sniffing
            // heuristic would fail hardest on the malformed payloads this
            // service exists to diagnose.
            let kind = PayloadKind::try_from(request.kind)
                .map_err(|_| Status::invalid_argument("unrecognised payload kind"))?;

            let document = request.document;
            let result = match kind {
                PayloadKind::BidRequest => {
                    run(budget, move || {
                        core::validate_bid_request_with_profile(
                            version, dialect, profile, &document,
                        )
                    })
                    .await?
                }
                PayloadKind::BidResponse => {
                    run(budget, move || {
                        core::validate_bid_response_with_profile(
                            version, dialect, profile, &document,
                        )
                    })
                    .await?
                }
                PayloadKind::Unspecified => {
                    return Err(Status::invalid_argument(
                        "kind is required: set PAYLOAD_KIND_BID_REQUEST or \
                         PAYLOAD_KIND_BID_RESPONSE. OpenRTB payloads carry no marker \
                         distinguishing them, so the server cannot infer it",
                    ))
                }
            };

            Ok(Response::new(ValidateResponse {
                verdict: Some(convert::verdict(&result, version)),
            }))
        })
        .await
    }

    async fn validate_pair(
        &self,
        request: Request<ValidatePairRequest>,
    ) -> Result<Response<ValidatePairResponse>, Status> {
        observed("ValidatePair", async move {
            let budget = deadline::remaining(request.metadata());
            let request = request.into_inner();
            let version = convert::version(request.context.as_ref())?;
            let dialect = convert::dialect(request.context.as_ref());
            let profile = convert::profile(request.context.as_ref())?;

            let bid_request = request.bid_request;
            let bid_response = request.bid_response;

            let result = run(budget, move || {
                core::validate_bid_response_against_request_with_profile(
                    version,
                    dialect,
                    profile,
                    &bid_request,
                    &bid_response,
                )
            })
            .await?;

            Ok(Response::new(ValidatePairResponse {
                verdict: Some(convert::verdict(&result, version)),
            }))
        })
        .await
    }

    async fn validate_artf_envelope(
        &self,
        request: Request<ValidateArtfEnvelopeRequest>,
    ) -> Result<Response<ValidateArtfEnvelopeResponse>, Status> {
        observed("ValidateArtfEnvelope", async move {
            let budget = deadline::remaining(request.metadata());
            let request = request.into_inner();
            let context = request.context.as_ref();
            let version = convert::version(context)?;
            convert::reject_dialect_on_artf(context)?;
            convert::reject_profile_on_artf(context)?;

            let rtb_request = request.rtb_request;
            let result = run(budget, move || {
                core::validate_artf_request(version, &rtb_request)
            })
            .await?;

            Ok(Response::new(ValidateArtfEnvelopeResponse {
                verdict: Some(convert::verdict(&result, version)),
            }))
        })
        .await
    }

    async fn validate_artf_mutations(
        &self,
        request: Request<ValidateArtfMutationsRequest>,
    ) -> Result<Response<ValidateArtfMutationsResponse>, Status> {
        observed("ValidateArtfMutations", async move {
            let budget = deadline::remaining(request.metadata());
            let request = request.into_inner();
            let context = request.context.as_ref();
            let version = convert::version(context)?;
            convert::reject_dialect_on_artf(context)?;
            convert::reject_profile_on_artf(context)?;

            let rtb_request = request.rtb_request;
            let rtb_response = request.rtb_response;

            if !request.apply {
                let result = run(budget, move || {
                    core::validate_artf_response_against_request(
                        version,
                        &rtb_request,
                        &rtb_response,
                    )
                })
                .await?;

                return Ok(Response::new(ValidateArtfMutationsResponse {
                    verdict: Some(convert::verdict(&result, version)),
                    application: None,
                }));
            }

            let outcome = run(budget, move || {
                core::validate_artf_mutations_applied(version, &rtb_request, &rtb_response)
            })
            .await?;

            Ok(Response::new(ValidateArtfMutationsResponse {
                verdict: Some(convert::verdict(&outcome.result, version)),
                application: Some(MutationApplication {
                    // Proto3 has no null: an envelope that carried no payload
                    // of that kind produces the empty string, which is not a
                    // JSON document and so cannot be mistaken for one.
                    bid_request: outcome.application.bid_request.unwrap_or_default(),
                    bid_response: outcome.application.bid_response.unwrap_or_default(),
                    applied: convert::indexes(&outcome.application.applied),
                    skipped: convert::indexes(&outcome.application.skipped),
                }),
            }))
        })
        .await
    }

    async fn list_versions(
        &self,
        _request: Request<ListVersionsRequest>,
    ) -> Result<Response<ListVersionsResponse>, Status> {
        observed("ListVersions", async move {
            let versions = core::OpenRtbVersion::all()
                .iter()
                .map(|version| VersionInfo {
                    id: version.id().to_string(),
                    family: convert::family(version.family()) as i32,
                    is_default: *version == convert::DEFAULT_VERSION,
                })
                .collect();

            Ok(Response::new(ListVersionsResponse {
                versions,
                provenance: Some(provenance()),
            }))
        })
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_REQUEST: &str =
        r#"{"id":"r1","imp":[{"id":"i1","video":{"mimes":["video/mp4"]}}]}"#;

    fn service() -> RtblintApi {
        RtblintApi::new()
    }

    fn validate_request(document: &str, kind: PayloadKind) -> Request<ValidateRequest> {
        Request::new(ValidateRequest {
            document: document.to_string(),
            kind: kind as i32,
            context: None,
        })
    }

    #[tokio::test]
    async fn a_valid_bid_request_passes_with_provenance() {
        let verdict = service()
            .validate(validate_request(VALID_REQUEST, PayloadKind::BidRequest))
            .await
            .expect("validate succeeds")
            .into_inner()
            .verdict
            .expect("verdict present");

        assert!(verdict.valid, "unexpected findings: {:?}", verdict.issues);
        assert_eq!(verdict.effective_version, "2.6-202606");

        let provenance = verdict.provenance.expect("provenance present");
        assert!(provenance.catalog_digest.starts_with("sha256:"));
    }

    #[tokio::test]
    async fn a_broken_bid_request_reports_findings() {
        let verdict = service()
            .validate(validate_request("{ not json", PayloadKind::BidRequest))
            .await
            .expect("validate succeeds")
            .into_inner()
            .verdict
            .expect("verdict present");

        assert!(!verdict.valid);
        assert!(!verdict.issues.is_empty());
        assert!(verdict.summary.expect("summary").errors > 0);
    }

    /// The one place this service asks something of callers that vastlint does
    /// not. Guessing would be worse than refusing, so refusing has to be tested.
    #[tokio::test]
    async fn an_unspecified_payload_kind_is_rejected_with_guidance() {
        let status = service()
            .validate(validate_request(VALID_REQUEST, PayloadKind::Unspecified))
            .await
            .expect_err("kind is required");

        assert_eq!(status.code(), tonic::Code::InvalidArgument);
        assert!(status.message().contains("PAYLOAD_KIND_BID_REQUEST"));
        assert!(status.message().contains("cannot infer"));
    }

    #[tokio::test]
    async fn an_unknown_version_is_rejected() {
        let status = service()
            .validate(Request::new(ValidateRequest {
                document: VALID_REQUEST.to_string(),
                kind: PayloadKind::BidRequest as i32,
                context: Some(crate::proto::ValidationContext {
                    version: "9.9".to_string(),
                    dialect: crate::proto::JsonDialect::Unspecified as i32,
                    profile: String::new(),
                }),
            }))
            .await
            .expect_err("unknown version is rejected");

        assert_eq!(status.code(), tonic::Code::InvalidArgument);
    }

    /// The version is not decoration: the same payload gets a different answer
    /// under a different specification revision, which is why the verdict
    /// echoes the version it used.
    #[tokio::test]
    async fn the_effective_version_echoes_what_was_requested() {
        let verdict = service()
            .validate(Request::new(ValidateRequest {
                document: VALID_REQUEST.to_string(),
                kind: PayloadKind::BidRequest as i32,
                context: Some(crate::proto::ValidationContext {
                    version: "2.5".to_string(),
                    dialect: crate::proto::JsonDialect::Unspecified as i32,
                    profile: String::new(),
                }),
            }))
            .await
            .expect("validate succeeds")
            .into_inner()
            .verdict
            .expect("verdict present");

        assert_eq!(verdict.effective_version, "2.5");
    }

    #[tokio::test]
    async fn list_versions_reports_every_tracked_version_and_one_default() {
        let response = service()
            .list_versions(Request::new(ListVersionsRequest {}))
            .await
            .expect("list_versions succeeds")
            .into_inner();

        assert_eq!(response.versions.len(), core::OpenRtbVersion::all().len());

        let defaults: Vec<_> = response
            .versions
            .iter()
            .filter(|version| version.is_default)
            .collect();
        assert_eq!(defaults.len(), 1, "exactly one version is the default");
        assert_eq!(defaults[0].id, convert::DEFAULT_VERSION.id());

        assert!(response
            .versions
            .iter()
            .all(|version| version.family != crate::proto::OpenrtbFamily::Unspecified as i32));
    }

    #[tokio::test]
    async fn validate_pair_cross_checks_the_two_payloads() {
        // A response bidding on an impression the request never offered. Valid
        // on its own, wrong as an answer to this request.
        let response =
            r#"{"id":"r1","seatbid":[{"bid":[{"id":"b1","impid":"nope","price":1.0}]}]}"#;

        let verdict = service()
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

        assert!(!verdict.valid, "a bid on an unknown impid is not valid");
        assert!(
            verdict
                .issues
                .iter()
                .any(|issue| issue.rule_id == "openrtb.bid.impid_unknown"),
            "expected the cross-payload finding, got {:?}",
            verdict
                .issues
                .iter()
                .map(|i| &i.rule_id)
                .collect::<Vec<_>>()
        );

        // The point of the separate RPC: this response is well formed. Only
        // comparing it against the request reveals the problem.
        let alone = service()
            .validate(validate_request(response, PayloadKind::BidResponse))
            .await
            .expect("validate succeeds")
            .into_inner()
            .verdict
            .expect("verdict present");

        assert!(
            !alone
                .issues
                .iter()
                .any(|issue| issue.rule_id == "openrtb.bid.impid_unknown"),
            "a single-payload check cannot see a cross-payload problem"
        );
    }

    /// The flag encoding is a fact the caller has and the server does not, so
    /// the same payload has to get opposite verdicts under the two dialects.
    #[tokio::test]
    async fn the_dialect_decides_how_flag_fields_are_read() {
        let proto_flavoured =
            r#"{"id":"r1","imp":[{"id":"i1","secure":true,"banner":{"w":300,"h":250}}]}"#;

        let as_spec = service()
            .validate(validate_request(proto_flavoured, PayloadKind::BidRequest))
            .await
            .expect("validate succeeds")
            .into_inner()
            .verdict
            .expect("verdict present");

        assert!(!as_spec.valid);
        assert!(as_spec
            .issues
            .iter()
            .any(|issue| issue.rule_id == "openrtb.dialect.bool_for_integer"));

        let as_proto = service()
            .validate(Request::new(ValidateRequest {
                document: proto_flavoured.to_string(),
                kind: PayloadKind::BidRequest as i32,
                context: Some(crate::proto::ValidationContext {
                    version: String::new(),
                    dialect: crate::proto::JsonDialect::Proto as i32,
                    profile: String::new(),
                }),
            }))
            .await
            .expect("validate succeeds")
            .into_inner()
            .verdict
            .expect("verdict present");

        assert!(as_proto.valid, "unexpected findings: {:?}", as_proto.issues);
    }

    /// Google's auction type 3 is not in the spec's {1, 2} set, so the same
    /// payload is invalid under the spec profile and valid under google-ab.
    #[tokio::test]
    async fn the_profile_decides_whether_fixed_price_is_an_auction_type() {
        let google = r#"{
            "id": "r1",
            "at": 3,
            "imp": [{
                "id": "i1",
                "banner": { "w": 300, "h": 250 },
                "ext": { "billing_id": ["1"] }
            }]
        }"#;

        let as_spec = service()
            .validate(validate_request(google, PayloadKind::BidRequest))
            .await
            .expect("validate succeeds")
            .into_inner()
            .verdict
            .expect("verdict present");

        assert!(!as_spec.valid);
        assert!(as_spec
            .issues
            .iter()
            .any(|issue| issue.rule_id == "openrtb.value.invalid" && issue.path == "at"));

        let as_google = service()
            .validate(Request::new(ValidateRequest {
                document: google.to_string(),
                kind: PayloadKind::BidRequest as i32,
                context: Some(crate::proto::ValidationContext {
                    version: String::new(),
                    dialect: crate::proto::JsonDialect::Unspecified as i32,
                    profile: "google-ab".to_string(),
                }),
            }))
            .await
            .expect("validate succeeds")
            .into_inner()
            .verdict
            .expect("verdict present");

        assert!(
            as_google.valid,
            "unexpected findings: {:?}",
            as_google.issues
        );
    }

    #[tokio::test]
    async fn the_profile_requires_a_prebid_bidder() {
        let payload = r#"{
            "id": "r1",
            "imp": [{ "id": "i1", "banner": { "w": 300, "h": 250 } }]
        }"#;

        let as_spec = service()
            .validate(validate_request(payload, PayloadKind::BidRequest))
            .await
            .expect("validate succeeds")
            .into_inner()
            .verdict
            .expect("verdict present");
        assert!(as_spec.valid, "unexpected findings: {:?}", as_spec.issues);

        let as_pbs = service()
            .validate(Request::new(ValidateRequest {
                document: payload.to_string(),
                kind: PayloadKind::BidRequest as i32,
                context: Some(crate::proto::ValidationContext {
                    version: String::new(),
                    dialect: crate::proto::JsonDialect::Unspecified as i32,
                    profile: "prebid-server".to_string(),
                }),
            }))
            .await
            .expect("validate succeeds")
            .into_inner()
            .verdict
            .expect("verdict present");

        assert!(!as_pbs.valid);
        assert!(as_pbs.issues.iter().any(|issue| {
            issue.rule_id == "openrtb.profile.prebid.bidder_required" && issue.path == "imp[0].ext"
        }));
    }

    #[tokio::test]
    async fn an_unknown_profile_is_rejected() {
        let status = service()
            .validate(Request::new(ValidateRequest {
                document: VALID_REQUEST.to_string(),
                kind: PayloadKind::BidRequest as i32,
                context: Some(crate::proto::ValidationContext {
                    version: String::new(),
                    dialect: crate::proto::JsonDialect::Unspecified as i32,
                    profile: "magnite".to_string(),
                }),
            }))
            .await
            .expect_err("unknown profile is rejected");

        assert_eq!(status.code(), tonic::Code::InvalidArgument);
        assert!(status.message().contains("magnite"));
    }

    // -- ARTF --

    const ARTF_ENVELOPE: &str = r#"{
        "id": "ep-1",
        "tmax": 120,
        "lifecycle": "LIFECYCLE_PUBLISHER_BID_REQUEST",
        "originator": { "type": "TYPE_EXCHANGE", "id": "x-1" },
        "applicable_intents": ["ACTIVATE_DEALS"],
        "bid_request": {
            "id": "auction-1",
            "imp": [
                {
                    "id": "imp-1",
                    "secure": true,
                    "banner": { "w": 300, "h": 250 },
                    "pmp": { "private_auction": true, "deals": [{ "id": "deal-1" }] }
                }
            ],
            "site": { "id": "s-1", "domain": "news.example" }
        }
    }"#;

    fn mutations(body: &str) -> String {
        format!(
            r#"{{
                "id": "ep-1",
                "mutations": [{body}],
                "metadata": {{ "api_version": "1.0.0", "model_version": "m" }}
            }}"#
        )
    }

    fn activate_deal(imp_id: &str, deal_id: &str) -> String {
        format!(
            r#"{{
                "intent": "ACTIVATE_DEALS",
                "op": "OPERATION_ADD",
                "path": "/imp/{imp_id}",
                "ids": {{ "id": ["{deal_id}"] }}
            }}"#
        )
    }

    #[tokio::test]
    async fn an_artf_envelope_validates_its_carried_payload_as_protobuf_json() {
        let verdict = service()
            .validate_artf_envelope(Request::new(ValidateArtfEnvelopeRequest {
                rtb_request: ARTF_ENVELOPE.to_string(),
                context: None,
            }))
            .await
            .expect("validate_artf_envelope succeeds")
            .into_inner()
            .verdict
            .expect("verdict present");

        // "secure": true is correct inside ARTF and wrong in spec JSON. The
        // envelope RPC has to know that without being told.
        assert!(verdict.valid, "unexpected findings: {:?}", verdict.issues);
        assert_eq!(verdict.effective_version, "2.6-202606");
    }

    #[tokio::test]
    async fn an_artf_envelope_reports_carried_payload_findings_under_a_prefixed_path() {
        let broken = ARTF_ENVELOPE.replace(r#""secure": true"#, r#""secure": 1"#);

        let verdict = service()
            .validate_artf_envelope(Request::new(ValidateArtfEnvelopeRequest {
                rtb_request: broken,
                context: None,
            }))
            .await
            .expect("validate_artf_envelope succeeds")
            .into_inner()
            .verdict
            .expect("verdict present");

        assert!(!verdict.valid);
        assert!(
            verdict.issues.iter().any(|issue| {
                issue.rule_id == "openrtb.dialect.integer_for_bool"
                    && issue.path == "bid_request.imp[0].secure"
            }),
            "expected a prefixed carried-payload path, got {:?}",
            verdict.issues.iter().map(|i| &i.path).collect::<Vec<_>>()
        );
    }

    #[tokio::test]
    async fn a_declared_dialect_is_refused_on_the_artf_rpcs() {
        let status = service()
            .validate_artf_envelope(Request::new(ValidateArtfEnvelopeRequest {
                rtb_request: ARTF_ENVELOPE.to_string(),
                context: Some(crate::proto::ValidationContext {
                    version: String::new(),
                    dialect: crate::proto::JsonDialect::Proto as i32,
                    profile: String::new(),
                }),
            }))
            .await
            .expect_err("a declared dialect is refused");

        assert_eq!(status.code(), tonic::Code::InvalidArgument);
        assert!(status.message().contains("protobuf JSON by definition"));
    }

    #[tokio::test]
    async fn a_declared_profile_is_refused_on_the_artf_rpcs() {
        let status = service()
            .validate_artf_envelope(Request::new(ValidateArtfEnvelopeRequest {
                rtb_request: ARTF_ENVELOPE.to_string(),
                context: Some(crate::proto::ValidationContext {
                    version: String::new(),
                    dialect: crate::proto::JsonDialect::Unspecified as i32,
                    profile: "google-ab".to_string(),
                }),
            }))
            .await
            .expect_err("a declared profile is refused");

        assert_eq!(status.code(), tonic::Code::InvalidArgument);
        assert!(status.message().contains("not an exchange"));
    }

    #[tokio::test]
    async fn a_mutation_targeting_a_missing_impression_is_reported() {
        let response = service()
            .validate_artf_mutations(Request::new(ValidateArtfMutationsRequest {
                rtb_request: ARTF_ENVELOPE.to_string(),
                rtb_response: mutations(&activate_deal("imp-404", "deal-2")),
                apply: false,
                context: None,
            }))
            .await
            .expect("validate_artf_mutations succeeds")
            .into_inner();

        let verdict = response.verdict.expect("verdict present");
        assert!(!verdict.valid);
        assert!(verdict
            .issues
            .iter()
            .any(|issue| issue.rule_id == "artf.mutation.imp_unknown"));

        // No application without apply: the caller did not ask for the payload
        // to be rewritten and must not be handed one.
        assert!(response.application.is_none());
    }

    #[tokio::test]
    async fn applying_returns_the_rewritten_payload_and_which_mutations_went_in() {
        let response = service()
            .validate_artf_mutations(Request::new(ValidateArtfMutationsRequest {
                rtb_request: ARTF_ENVELOPE.to_string(),
                rtb_response: mutations(&activate_deal("imp-1", "deal-2")),
                apply: true,
                context: None,
            }))
            .await
            .expect("validate_artf_mutations succeeds")
            .into_inner();

        let verdict = response.verdict.expect("verdict present");
        assert!(verdict.valid, "unexpected findings: {:?}", verdict.issues);

        let application = response.application.expect("application present");
        assert_eq!(application.applied, vec![0]);
        assert!(application.skipped.is_empty());
        assert!(application.bid_request.contains("deal-2"));
        // The envelope carried no bid response, and proto3 has no null.
        assert!(application.bid_response.is_empty());
    }

    /// The point of the applied pass: a mutation that is well formed on its own
    /// and leaves the auction invalid once written in.
    #[tokio::test]
    async fn applying_reports_what_the_mutation_broke() {
        let metric = r#"{
            "intent": "ADD_METRICS",
            "op": "OPERATION_ADD",
            "path": "/imp/imp-1",
            "metrics": { "metric": [{ "type": "viewability", "value": "high" }] }
        }"#;
        let envelope = ARTF_ENVELOPE.replace(r#"["ACTIVATE_DEALS"]"#, r#"["ADD_METRICS"]"#);

        let statically_fine = service()
            .validate_artf_mutations(Request::new(ValidateArtfMutationsRequest {
                rtb_request: envelope.clone(),
                rtb_response: mutations(metric),
                apply: false,
                context: None,
            }))
            .await
            .expect("validate_artf_mutations succeeds")
            .into_inner()
            .verdict
            .expect("verdict present");
        assert!(
            statically_fine.valid,
            "the mutation passes the static checks: {:?}",
            statically_fine.issues
        );

        let applied = service()
            .validate_artf_mutations(Request::new(ValidateArtfMutationsRequest {
                rtb_request: envelope,
                rtb_response: mutations(metric),
                apply: true,
                context: None,
            }))
            .await
            .expect("validate_artf_mutations succeeds")
            .into_inner()
            .verdict
            .expect("verdict present");

        assert!(!applied.valid);
        assert!(applied.issues.iter().any(|issue| {
            issue.rule_id == "openrtb.type.mismatch"
                && issue.path == "bid_request.imp[0].metric[0].value"
        }));
    }

    #[tokio::test]
    async fn an_expired_deadline_is_refused_before_any_work_starts() {
        let mut request = validate_request(VALID_REQUEST, PayloadKind::BidRequest);
        request
            .metadata_mut()
            .insert("grpc-timeout", "0m".parse().unwrap());

        let status = service()
            .validate(request)
            .await
            .expect_err("an expired deadline is refused");

        assert_eq!(status.code(), tonic::Code::DeadlineExceeded);
    }
}
