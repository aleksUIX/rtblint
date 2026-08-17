//! ARTF: the IAB Tech Lab Agentic Real Time Framework.
//!
//! ARTF wraps an OpenRTB 2.6 payload in an `RTBRequest` envelope, hands it to
//! an agent, and takes back an `RTBResponse` carrying *mutations*: proposed,
//! independently acceptable changes to the auction. The orchestrator applies
//! whichever it accepts.
//!
//! Nothing in the framework checks that the result is still a legal bid
//! request, and the mutation itself is only meaningful relative to the request
//! it targets, so three passes live here:
//!
//! 1. [`validate_artf_request`] checks the envelope and validates the OpenRTB
//!    payloads it carries. Those payloads travel as protobuf JSON, so they are
//!    validated in [`Dialect::ProtoJson`].
//! 2. [`validate_artf_response_against_request`] checks every mutation against
//!    the request it answers: declared intent, operation, payload shape, and
//!    whether the semantic path resolves to anything that exists.
//! 3. [`validate_artf_mutations_applied`] applies the mutations and revalidates,
//!    reporting only the OpenRTB findings the mutations introduced.
//!
//! Paths are semantic references (`/imp/imp-1/pmp/deals/deal-premium`), not
//! JSON pointers: they name business entities by id, so an agent can address a
//! deal without knowing the document layout. The ARTF v1.0 document and the
//! reference implementation's example docs disagree about whether a deal sits
//! at `/imp/{id}/deals/{id}` or `/imp/{id}/pmp/deals/{id}`, so both are
//! accepted.

use serde::Serialize;
use serde_json::{json, Map, Value};

use crate::{pair, validator, Dialect, Issue, OpenRtbVersion, Profile, Severity, ValidationResult};

/// Envelope members the ARTF proto defines.
const RTB_REQUEST_MEMBERS: &[&str] = &[
    "lifecycle",
    "id",
    "tmax",
    "bid_request",
    "bid_response",
    "originator",
    "applicable_intents",
    "ext",
];

/// Envelope members the proto marks required.
const RTB_REQUEST_REQUIRED: &[&str] = &["id", "lifecycle", "tmax", "bid_request"];

const RTB_RESPONSE_MEMBERS: &[&str] = &["id", "mutations", "metadata", "ext"];

const LIFECYCLES: &[&str] = &[
    "LIFECYCLE_UNSPECIFIED",
    "LIFECYCLE_PUBLISHER_BID_REQUEST",
    "LIFECYCLE_DSP_BID_RESPONSE",
];

const ORIGINATOR_TYPES: &[&str] = &[
    "TYPE_UNSPECIFIED",
    "TYPE_PUBLISHER",
    "TYPE_SSP",
    "TYPE_EXCHANGE",
    "TYPE_DSP",
];

const INTENTS: &[&str] = &[
    "INTENT_UNSPECIFIED",
    "ACTIVATE_SEGMENTS",
    "ACTIVATE_DEALS",
    "SUPPRESS_DEALS",
    "ADJUST_DEAL_FLOOR",
    "ADJUST_DEAL_MARGIN",
    "BID_SHADE",
    "ADD_METRICS",
    "ADD_CIDS",
];

const OPERATIONS: &[&str] = &[
    "OPERATION_UNSPECIFIED",
    "OPERATION_ADD",
    "OPERATION_REMOVE",
    "OPERATION_REPLACE",
];

/// Mutation payload members (the proto's `value` oneof).
const PAYLOAD_MEMBERS: &[&str] = &[
    "ids",
    "adjust_deal",
    "adjust_bid",
    "metrics",
    "content_data",
];

/// The intent vocabulary the ARTF v1.0 document's own examples use, which is
/// not the vocabulary its `.proto` defines. Implementers who work from the
/// document produce the left-hand spellings; the reference server only accepts
/// the right-hand ones.
const LEGACY_INTENT_NAMES: &[(&str, &str)] = &[
    ("activateDeals", "ACTIVATE_DEALS"),
    ("activateSegments", "ACTIVATE_SEGMENTS"),
    ("addMetrics", "ADD_METRICS"),
    ("adjustDeals", "ADJUST_DEAL_FLOOR"),
    ("bidShade", "BID_SHADE"),
    ("expireDeals", "SUPPRESS_DEALS"),
];

const LEGACY_OPERATION_NAMES: &[(&str, &str)] = &[
    ("add", "OPERATION_ADD"),
    ("remove", "OPERATION_REMOVE"),
    ("replace", "OPERATION_REPLACE"),
];

/// Legacy payload wrappers from the document's examples
/// (`"value": {"IDsPayload": [...]}`).
const LEGACY_PAYLOAD_MEMBERS: &[&str] = &[
    "IDsPayload",
    "AdjustDealPayload",
    "AdjustBidPayload",
    "AddMetricsPayload",
];

/// tmax past which an in-auction agent call stops being plausible. The
/// reference samples run at 150ms and the whole point of the extension point
/// is that it fits inside an auction the exchange is already timing out.
const TMAX_CEILING_MS: i64 = 1_000;

/// What applying a mutation set produced.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct ArtfApplication {
    /// The bid request after every applicable mutation was applied, pretty
    /// printed. `None` when the envelope carried no usable bid request.
    pub bid_request: Option<String>,
    /// The bid response after every applicable mutation was applied. `None`
    /// when the envelope carried none.
    pub bid_response: Option<String>,
    /// Indexes into `mutations` that were applied.
    pub applied: Vec<usize>,
    /// Indexes into `mutations` that were not applied, either because the
    /// target did not resolve or because the intent has no OpenRTB field to
    /// write to.
    pub skipped: Vec<usize>,
}

/// The result of applying a mutation set and revalidating.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[non_exhaustive]
pub struct ArtfMutationOutcome {
    /// Mutation findings, plus the OpenRTB findings the mutations introduced.
    pub result: ValidationResult,
    pub application: ArtfApplication,
}

/// Validates an ARTF `RTBRequest` envelope and the OpenRTB payloads it carries.
///
/// The OpenRTB payloads are validated in [`Dialect::ProtoJson`], since ARTF
/// transports them as protobuf messages; their findings are reported under
/// `bid_request.` and `bid_response.` paths.
pub fn validate_artf_request(version: OpenRtbVersion, input: &str) -> ValidationResult {
    let root = match parse_root(input, "RTBRequest") {
        Ok(root) => root,
        Err(issues) => return validator::finalize_result(issues),
    };

    let mut issues = Vec::new();
    validate_envelope_members(&root, RTB_REQUEST_MEMBERS, "RTBRequest", &mut issues);

    for member in RTB_REQUEST_REQUIRED {
        if !root.contains_key(*member) {
            issues.push(issue(
                "artf.field.required",
                Severity::Error,
                format!("{member} is required on an ARTF RTBRequest."),
                Some((*member).to_string()),
            ));
        }
    }

    validate_tmax(root.get("tmax"), &mut issues);
    validate_lifecycle(&root, &mut issues);
    validate_originator(root.get("originator"), &mut issues);
    validate_applicable_intents(root.get("applicable_intents"), &mut issues);

    if let Some(bid_request) = root.get("bid_request") {
        issues.extend(validate_embedded_payload(
            version,
            bid_request,
            "bid_request",
            true,
        ));
    }
    if let Some(bid_response) = root.get("bid_response") {
        issues.extend(validate_embedded_payload(
            version,
            bid_response,
            "bid_response",
            false,
        ));
    }

    validator::finalize_result(issues)
}

/// Validates an ARTF `RTBResponse` against the `RTBRequest` it answers.
///
/// `request_input` is the RTBRequest envelope, not a bare bid request: the
/// mutation checks need its id, its `applicable_intents`, and the OpenRTB
/// payloads the semantic paths resolve against.
pub fn validate_artf_response_against_request(
    version: OpenRtbVersion,
    request_input: &str,
    response_input: &str,
) -> ValidationResult {
    // The mutation checks are structural and version independent; `version`
    // is taken so the signature does not have to change when a later ARTF
    // revision ties intents to specific OpenRTB snapshots.
    let _ = version;
    let response = match parse_root(response_input, "RTBResponse") {
        Ok(response) => response,
        Err(issues) => return validator::finalize_result(issues),
    };

    let mut issues = Vec::new();
    validate_envelope_members(&response, RTB_RESPONSE_MEMBERS, "RTBResponse", &mut issues);

    let request = match serde_json::from_str::<Value>(request_input)
        .ok()
        .and_then(|value| value.as_object().cloned())
    {
        Some(request) => request,
        None => {
            issues.push(issue(
                "artf.request_unusable",
                Severity::Error,
                String::from(
                    "The ARTF RTBRequest supplied for cross-validation is not a JSON object, so \
                     no mutation can be checked against the auction it targets.",
                ),
                None,
            ));
            return validator::finalize_result(issues);
        }
    };

    let context = RequestContext::index(&request);

    match response.get("id").and_then(Value::as_str) {
        None => issues.push(issue(
            "artf.response.id_missing",
            Severity::Error,
            String::from(
                "RTBResponse.id is required: it is how the orchestrator ties mutations back to \
                 the extension point request it issued.",
            ),
            Some(String::from("id")),
        )),
        Some(response_id) => {
            if let Some(request_id) = context.id {
                if request_id != response_id {
                    issues.push(issue(
                        "artf.response.id_mismatch",
                        Severity::Error,
                        format!(
                            "RTBResponse.id \"{response_id}\" does not echo the RTBRequest id \
                             \"{request_id}\". The envelope id is the extension point request \
                             id, not the bid request id."
                        ),
                        Some(String::from("id")),
                    ));
                }
            }
        }
    }

    if response
        .get("metadata")
        .and_then(Value::as_object)
        .is_none()
    {
        issues.push(issue(
            "artf.response.metadata_missing",
            Severity::Warning,
            String::from(
                "No metadata object: without api_version and model_version an orchestrator \
                 cannot record which agent build produced these mutations.",
            ),
            None,
        ));
    }

    let mutations = response.get("mutations").and_then(Value::as_array);
    match mutations.map(Vec::as_slice) {
        None | Some([]) => issues.push(issue(
            "artf.mutations.empty",
            Severity::Warning,
            String::from(
                "The response proposes no mutations; the orchestrator paid for the call and has \
                 nothing to evaluate.",
            ),
            Some(String::from("mutations")),
        )),
        Some(mutations) => {
            for (index, mutation) in mutations.iter().enumerate() {
                validate_mutation(&context, mutation, index, &mut issues);
            }
        }
    }

    validator::finalize_result(issues)
}

/// Applies a mutation set to the payloads the RTBRequest envelope carries.
///
/// Only the intents that name an OpenRTB field are applied. `ADJUST_DEAL_MARGIN`
/// has no OpenRTB home and `ADD_CIDS` names no target in v1.0, so both are
/// reported as skipped rather than guessed at. `ACTIVATE_SEGMENTS` appends to
/// the first `user.data` entry, or creates one, because the mutation names no
/// data provider.
pub fn apply_artf_mutations(request_input: &str, response_input: &str) -> ArtfApplication {
    let empty = ArtfApplication {
        bid_request: None,
        bid_response: None,
        applied: Vec::new(),
        skipped: Vec::new(),
    };

    let (Ok(request), Ok(response)) = (
        serde_json::from_str::<Value>(request_input),
        serde_json::from_str::<Value>(response_input),
    ) else {
        return empty;
    };
    let (Some(request), Some(response)) = (request.as_object(), response.as_object()) else {
        return empty;
    };

    let mut bid_request = request.get("bid_request").cloned();
    let mut bid_response = request.get("bid_response").cloned();
    let mut applied = Vec::new();
    let mut skipped = Vec::new();

    for (index, mutation) in response
        .get("mutations")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .enumerate()
    {
        let Some(mutation) = mutation.as_object() else {
            skipped.push(index);
            continue;
        };
        let reading = MutationReading::of(mutation);
        let target = Target::parse(reading.path);

        let did_apply = match (reading.intent, target) {
            (Some("ACTIVATE_SEGMENTS"), Target::UserSegments) => bid_request
                .as_mut()
                .is_some_and(|request| apply_segments(request, &reading)),
            (Some("ACTIVATE_DEALS"), Target::Imp { imp_id })
            | (Some("SUPPRESS_DEALS"), Target::Imp { imp_id }) => bid_request
                .as_mut()
                .is_some_and(|request| apply_deal_ids(request, imp_id, &reading)),
            (Some("ADJUST_DEAL_FLOOR"), Target::Deal { imp_id, deal_id }) => bid_request
                .as_mut()
                .is_some_and(|request| apply_deal_floor(request, imp_id, deal_id, &reading)),
            (Some("ADD_METRICS"), Target::Imp { imp_id }) => bid_request
                .as_mut()
                .is_some_and(|request| apply_metrics(request, imp_id, &reading)),
            (Some("BID_SHADE"), Target::Bid { seat, bid_id }) => bid_response
                .as_mut()
                .is_some_and(|response| apply_bid_price(response, seat, bid_id, &reading)),
            _ => false,
        };

        if did_apply {
            applied.push(index);
        } else {
            skipped.push(index);
        }
    }

    ArtfApplication {
        bid_request: bid_request.as_ref().map(pretty),
        bid_response: bid_response.as_ref().map(pretty),
        applied,
        skipped,
    }
}

/// Applies a mutation set and revalidates the result, reporting the mutation
/// findings plus every OpenRTB finding the mutations introduced.
///
/// Findings the payload already had are filtered out: the question this
/// answers is what the agent broke, not what arrived broken.
pub fn validate_artf_mutations_applied(
    version: OpenRtbVersion,
    request_input: &str,
    response_input: &str,
) -> ArtfMutationOutcome {
    let mut result = validate_artf_response_against_request(version, request_input, response_input);
    let application = apply_artf_mutations(request_input, response_input);

    let envelope = serde_json::from_str::<Value>(request_input)
        .ok()
        .and_then(|value| value.as_object().cloned())
        .unwrap_or_default();

    if let Some(mutated) = application.bid_request.as_deref() {
        let before = envelope
            .get("bid_request")
            .map(pretty)
            .map(|json| validate_openrtb(version, &json, true))
            .unwrap_or_default();
        let after = validate_openrtb(version, mutated, true);
        result
            .issues
            .extend(introduced(before, after, "bid_request"));
    }

    if let (Some(mutated_response), Some(mutated_request)) = (
        application.bid_response.as_deref(),
        application.bid_request.as_deref(),
    ) {
        let before = match (envelope.get("bid_request"), envelope.get("bid_response")) {
            (Some(request), Some(response)) => {
                pair::validate_bid_response_against_request(
                    version,
                    Dialect::ProtoJson,
                    Profile::Spec,
                    &pretty(request),
                    &pretty(response),
                )
                .issues
            }
            _ => Vec::new(),
        };
        let after = pair::validate_bid_response_against_request(
            version,
            Dialect::ProtoJson,
            Profile::Spec,
            mutated_request,
            mutated_response,
        )
        .issues;
        result
            .issues
            .extend(introduced(before, after, "bid_response"));
    }

    ArtfMutationOutcome {
        result: validator::finalize_result(result.issues),
        application,
    }
}

// ---------------------------------------------------------------- envelope --

fn parse_root(input: &str, label: &str) -> Result<Map<String, Value>, Vec<Issue>> {
    let value = serde_json::from_str::<Value>(input).map_err(|error| {
        vec![issue(
            "artf.payload.invalid_json",
            Severity::Error,
            format!("Invalid JSON payload: {error}"),
            None,
        )]
    })?;

    value.as_object().cloned().ok_or_else(|| {
        vec![issue(
            "artf.payload.root_not_object",
            Severity::Error,
            format!("An ARTF {label} is a JSON object at the top level."),
            None,
        )]
    })
}

fn validate_envelope_members(
    root: &Map<String, Value>,
    known: &[&str],
    label: &str,
    issues: &mut Vec<Issue>,
) {
    for member in root.keys() {
        if !known.contains(&member.as_str()) {
            issues.push(issue(
                "artf.field.undefined",
                Severity::Error,
                format!("{member} is not a member of the ARTF {label} message."),
                Some(member.clone()),
            ));
        }
    }
}

fn validate_tmax(value: Option<&Value>, issues: &mut Vec<Issue>) {
    let Some(value) = value else {
        return;
    };
    let Some(tmax) = value.as_i64() else {
        issues.push(issue(
            "artf.tmax.not_integer",
            Severity::Error,
            format!(
                "tmax is the milliseconds the exchange allows for mutations; {value} is not an \
                 integer."
            ),
            Some(String::from("tmax")),
        ));
        return;
    };

    if tmax <= 0 {
        issues.push(issue(
            "artf.tmax.non_positive",
            Severity::Error,
            format!("tmax {tmax} leaves the agent no time to answer."),
            Some(String::from("tmax")),
        ));
    } else if tmax > TMAX_CEILING_MS {
        issues.push(issue(
            "artf.tmax.implausible",
            Severity::Warning,
            format!(
                "tmax {tmax}ms exceeds {TMAX_CEILING_MS}ms. The extension point runs inside an \
                 auction the exchange is already timing out; a budget this large usually means \
                 seconds were sent where milliseconds were meant."
            ),
            Some(String::from("tmax")),
        ));
    }
}

fn validate_lifecycle(root: &Map<String, Value>, issues: &mut Vec<Issue>) {
    let lifecycle = match read_enum(root.get("lifecycle"), LIFECYCLES) {
        EnumRead::Missing => return,
        EnumRead::Known(name) => name,
        EnumRead::Unknown(rendered) => {
            issues.push(issue(
                "artf.lifecycle.unknown",
                Severity::Error,
                format!(
                    "lifecycle {rendered} is not an ARTF Lifecycle value. Known values: {}.",
                    LIFECYCLES.join(", ")
                ),
                Some(String::from("lifecycle")),
            ));
            return;
        }
    };

    let has_response = root.get("bid_response").is_some_and(Value::is_object);

    match lifecycle {
        "LIFECYCLE_UNSPECIFIED" => issues.push(issue(
            "artf.lifecycle.unspecified",
            Severity::Error,
            String::from(
                "lifecycle is LIFECYCLE_UNSPECIFIED, so the agent cannot tell which auction \
                 stage it is being called at or which payload is authoritative.",
            ),
            Some(String::from("lifecycle")),
        )),
        "LIFECYCLE_DSP_BID_RESPONSE" if !has_response => issues.push(issue(
            "artf.lifecycle.payload_mismatch",
            Severity::Error,
            String::from(
                "lifecycle is LIFECYCLE_DSP_BID_RESPONSE but the envelope carries no \
                 bid_response, so response-stage intents such as BID_SHADE have nothing to \
                 target.",
            ),
            Some(String::from("bid_response")),
        )),
        "LIFECYCLE_PUBLISHER_BID_REQUEST" if has_response => issues.push(issue(
            "artf.lifecycle.payload_unexpected",
            Severity::Warning,
            String::from(
                "lifecycle is LIFECYCLE_PUBLISHER_BID_REQUEST, which runs before any DSP has \
                 answered, yet the envelope carries a bid_response.",
            ),
            Some(String::from("bid_response")),
        )),
        _ => {}
    }
}

fn validate_originator(value: Option<&Value>, issues: &mut Vec<Issue>) {
    let Some(originator) = value.and_then(Value::as_object) else {
        return;
    };

    if let EnumRead::Unknown(rendered) = read_enum(originator.get("type"), ORIGINATOR_TYPES) {
        issues.push(issue(
            "artf.originator.type_unknown",
            Severity::Error,
            format!(
                "originator.type {rendered} is not an ARTF Originator.Type value. Known values: \
                 {}.",
                ORIGINATOR_TYPES.join(", ")
            ),
            Some(String::from("originator.type")),
        ));
    }

    if originator
        .get("id")
        .and_then(Value::as_str)
        .map_or(true, str::is_empty)
    {
        issues.push(issue(
            "artf.originator.id_missing",
            Severity::Warning,
            String::from(
                "originator.id is empty: the agent cannot tell which business entity owns the \
                 payload it is mutating.",
            ),
            Some(String::from("originator.id")),
        ));
    }
}

fn validate_applicable_intents(value: Option<&Value>, issues: &mut Vec<Issue>) {
    let Some(value) = value else {
        issues.push(issue(
            "artf.intents.missing",
            Severity::Warning,
            String::from(
                "No applicable_intents: the agent is not told which intents it may return, and \
                 the orchestrator has no declared basis for rejecting the ones it gets.",
            ),
            None,
        ));
        return;
    };

    let Some(intents) = value.as_array() else {
        issues.push(issue(
            "artf.intents.not_array",
            Severity::Error,
            String::from("applicable_intents is a repeated Intent, so it must be an array."),
            Some(String::from("applicable_intents")),
        ));
        return;
    };

    if intents.is_empty() {
        issues.push(issue(
            "artf.intents.empty",
            Severity::Warning,
            String::from(
                "applicable_intents is empty, so every mutation the agent returns is out of \
                 scope by definition.",
            ),
            Some(String::from("applicable_intents")),
        ));
    }

    for (index, intent) in intents.iter().enumerate() {
        if let EnumRead::Unknown(rendered) = read_enum(Some(intent), INTENTS) {
            issues.push(issue(
                "artf.intent.unknown",
                Severity::Error,
                format!(
                    "applicable_intents[{index}] is {rendered}, which is not an ARTF Intent \
                     value. Known values: {}.",
                    INTENTS.join(", ")
                ),
                Some(format!("applicable_intents[{index}]")),
            ));
        }
    }
}

/// Validates an embedded OpenRTB payload and reports it under `prefix`.
fn validate_embedded_payload(
    version: OpenRtbVersion,
    value: &Value,
    prefix: &str,
    is_request: bool,
) -> Vec<Issue> {
    if !value.is_object() {
        return vec![issue(
            "artf.field.type_mismatch",
            Severity::Error,
            format!("{prefix} must be an OpenRTB object."),
            Some(String::from(prefix)),
        )];
    }

    let issues = validate_openrtb(version, &pretty(value), is_request);
    reprefix(issues, prefix)
}

fn validate_openrtb(version: OpenRtbVersion, payload: &str, is_request: bool) -> Vec<Issue> {
    if is_request {
        validator::validate_bid_request(version, Dialect::ProtoJson, Profile::Spec, payload).issues
    } else {
        validator::validate_bid_response(version, Dialect::ProtoJson, Profile::Spec, payload).issues
    }
}

fn reprefix(issues: Vec<Issue>, prefix: &str) -> Vec<Issue> {
    issues
        .into_iter()
        .map(|mut issue| {
            issue.path = Some(match issue.path {
                Some(path) => format!("{prefix}.{path}"),
                None => String::from(prefix),
            });
            issue
        })
        .collect()
}

/// The findings present after applying that were not present before, reported
/// as consequences of the mutation set.
fn introduced(before: Vec<Issue>, after: Vec<Issue>, prefix: &str) -> Vec<Issue> {
    let baseline: Vec<(String, Option<String>)> = before
        .into_iter()
        .map(|issue| (issue.id, issue.path))
        .collect();

    let fresh: Vec<Issue> = after
        .into_iter()
        .filter(|issue| {
            !baseline
                .iter()
                .any(|(id, path)| id == &issue.id && path == &issue.path)
        })
        .map(|mut issue| {
            issue.message = format!("After applying the mutations: {}", issue.message);
            issue
        })
        .collect();

    reprefix(fresh, prefix)
}

// ---------------------------------------------------------------- mutations --

/// Pass one over the RTBRequest: everything a mutation can be checked against.
struct RequestContext<'a> {
    id: Option<&'a str>,
    applicable_intents: Option<Vec<String>>,
    has_bid_response: bool,
    imps: Vec<ImpContext<'a>>,
    /// `(seat, bid ids)` for every seatbid in the carried bid response.
    seats: Vec<(&'a str, Vec<&'a str>)>,
}

struct ImpContext<'a> {
    id: &'a str,
    deal_ids: Vec<&'a str>,
}

impl<'a> RequestContext<'a> {
    fn index(request: &'a Map<String, Value>) -> Self {
        let bid_request = request.get("bid_request").and_then(Value::as_object);
        let bid_response = request.get("bid_response").and_then(Value::as_object);

        let imps = bid_request
            .and_then(|bid_request| bid_request.get("imp"))
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_object)
            .filter_map(|imp| {
                let id = imp.get("id").and_then(Value::as_str)?;
                let deal_ids = imp
                    .get("pmp")
                    .and_then(Value::as_object)
                    .and_then(|pmp| pmp.get("deals"))
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .filter_map(|deal| deal.get("id").and_then(Value::as_str))
                    .collect();
                Some(ImpContext { id, deal_ids })
            })
            .collect();

        let seats = bid_response
            .and_then(|bid_response| bid_response.get("seatbid"))
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_object)
            .map(|seatbid| {
                let seat = seatbid.get("seat").and_then(Value::as_str).unwrap_or("");
                let bid_ids = seatbid
                    .get("bid")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .filter_map(|bid| bid.get("id").and_then(Value::as_str))
                    .collect();
                (seat, bid_ids)
            })
            .collect();

        Self {
            id: request.get("id").and_then(Value::as_str),
            applicable_intents: request
                .get("applicable_intents")
                .and_then(Value::as_array)
                .map(|intents| {
                    intents
                        .iter()
                        .map(|intent| match read_enum(Some(intent), INTENTS) {
                            EnumRead::Known(name) => String::from(name),
                            EnumRead::Unknown(rendered) => rendered,
                            EnumRead::Missing => String::new(),
                        })
                        .collect()
                }),
            has_bid_response: bid_response.is_some(),
            imps,
            seats,
        }
    }

    fn imp(&self, imp_id: &str) -> Option<&ImpContext<'a>> {
        self.imps.iter().find(|imp| imp.id == imp_id)
    }
}

/// A mutation read in whichever encoding it arrived in.
struct MutationReading<'a> {
    /// Resolved Intent name, `None` when it could not be read at all.
    intent: Option<&'static str>,
    intent_rendered: String,
    operation: Option<&'static str>,
    operation_rendered: String,
    path: &'a str,
    path_present: bool,
    /// Present `value` oneof members.
    payload_members: Vec<&'a str>,
    payload: Option<&'a Value>,
    /// The mutation used the ARTF v1.0 document's vocabulary rather than the
    /// proto's.
    legacy_encoding: bool,
}

impl<'a> MutationReading<'a> {
    fn of(mutation: &'a Map<String, Value>) -> Self {
        let mut legacy_encoding = false;

        let (intent, intent_rendered) = match read_enum(mutation.get("intent"), INTENTS) {
            EnumRead::Known(name) => (Some(name), String::from(name)),
            EnumRead::Missing => (None, String::from("absent")),
            EnumRead::Unknown(rendered) => {
                match LEGACY_INTENT_NAMES
                    .iter()
                    .find(|(legacy, _)| rendered.trim_matches('"') == *legacy)
                {
                    Some((_, mapped)) => {
                        legacy_encoding = true;
                        (Some(*mapped), rendered)
                    }
                    None => (None, rendered),
                }
            }
        };

        let (operation, operation_rendered) = match read_enum(mutation.get("op"), OPERATIONS) {
            EnumRead::Known(name) => (Some(name), String::from(name)),
            EnumRead::Missing => (None, String::from("absent")),
            EnumRead::Unknown(rendered) => {
                match LEGACY_OPERATION_NAMES
                    .iter()
                    .find(|(legacy, _)| rendered.trim_matches('"') == *legacy)
                {
                    Some((_, mapped)) => {
                        legacy_encoding = true;
                        (Some(*mapped), rendered)
                    }
                    None => (None, rendered),
                }
            }
        };

        let mut payload_members: Vec<&str> = PAYLOAD_MEMBERS
            .iter()
            .copied()
            .filter(|member| mutation.contains_key(*member))
            .collect();

        // The document wraps the payload in a `value` object keyed by the
        // payload message name; the proto puts the oneof members at the top
        // level of the Mutation.
        let mut payload = payload_members
            .first()
            .and_then(|member| mutation.get(*member));

        if let Some(wrapper) = mutation.get("value").and_then(Value::as_object) {
            for member in LEGACY_PAYLOAD_MEMBERS {
                if let Some(value) = wrapper.get(*member) {
                    legacy_encoding = true;
                    payload_members.push(*member);
                    payload = Some(value);
                }
            }
        }

        let path = mutation.get("path").and_then(Value::as_str);

        Self {
            intent,
            intent_rendered,
            operation,
            operation_rendered,
            path: path.unwrap_or_default(),
            path_present: path.is_some(),
            payload_members,
            payload,
            legacy_encoding,
        }
    }
}

/// The business entity a semantic path names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Target<'a> {
    UserSegments,
    Imp { imp_id: &'a str },
    Deal { imp_id: &'a str, deal_id: &'a str },
    Bid { seat: &'a str, bid_id: &'a str },
    Unrecognized,
}

impl<'a> Target<'a> {
    fn parse(path: &'a str) -> Self {
        let segments: Vec<&str> = path.split('/').filter(|part| !part.is_empty()).collect();
        match segments.as_slice() {
            ["user", "data", "segment"] => Self::UserSegments,
            ["imp", imp_id] => Self::Imp { imp_id },
            ["imp", imp_id, "deals", deal_id] | ["imp", imp_id, "pmp", "deals", deal_id] => {
                Self::Deal { imp_id, deal_id }
            }
            ["seatbid", seat, "bid", bid_id] => Self::Bid { seat, bid_id },
            _ => Self::Unrecognized,
        }
    }

    /// The intents this target can carry, for the path/intent coherence check.
    fn intents(self) -> &'static [&'static str] {
        match self {
            Self::UserSegments => &["ACTIVATE_SEGMENTS", "ADD_CIDS"],
            Self::Imp { .. } => &["ACTIVATE_DEALS", "SUPPRESS_DEALS", "ADD_METRICS"],
            Self::Deal { .. } => &["ADJUST_DEAL_FLOOR", "ADJUST_DEAL_MARGIN"],
            Self::Bid { .. } => &["BID_SHADE"],
            Self::Unrecognized => &[],
        }
    }
}

/// The `value` oneof member each intent carries.
fn payload_members_for(intent: &str) -> &'static [&'static str] {
    match intent {
        "ACTIVATE_SEGMENTS" | "ACTIVATE_DEALS" | "SUPPRESS_DEALS" => &["ids"],
        "ADJUST_DEAL_FLOOR" | "ADJUST_DEAL_MARGIN" => &["adjust_deal"],
        "BID_SHADE" => &["adjust_bid"],
        "ADD_METRICS" => &["metrics"],
        "ADD_CIDS" => &["ids", "content_data"],
        _ => &[],
    }
}

fn validate_mutation(
    context: &RequestContext<'_>,
    mutation: &Value,
    index: usize,
    issues: &mut Vec<Issue>,
) {
    let base = format!("mutations[{index}]");
    let Some(mutation) = mutation.as_object() else {
        issues.push(issue(
            "artf.mutation.not_object",
            Severity::Error,
            String::from("Each entry in mutations is a Mutation object."),
            Some(base),
        ));
        return;
    };

    let reading = MutationReading::of(mutation);

    if reading.legacy_encoding {
        issues.push(issue(
            "artf.mutation.legacy_spec_encoding",
            Severity::Warning,
            format!(
                "This mutation uses the vocabulary of the ARTF v1.0 document's examples \
                 (intent \"{}\", op \"{}\", or a value wrapper such as IDsPayload) rather than \
                 the enum names and oneof members its .proto defines. The reference server \
                 speaks the proto encoding, so send INTENT/OPERATION enum names and top-level \
                 oneof members.",
                reading.intent_rendered.trim_matches('"'),
                reading.operation_rendered.trim_matches('"')
            ),
            Some(base.clone()),
        ));
    }

    validate_mutation_intent(context, &reading, &base, issues);
    validate_mutation_operation(&reading, &base, issues);
    let target = validate_mutation_path(context, &reading, &base, issues);
    validate_mutation_payload(&reading, &base, issues);

    if let (Some(intent), Some(target)) = (reading.intent, target) {
        if !target.intents().is_empty() && !target.intents().contains(&intent) {
            issues.push(issue(
                "artf.mutation.path_intent_mismatch",
                Severity::Warning,
                format!(
                    "Intent {intent} targets \"{}\", which names {}. Paths that carry {intent} \
                     look like {}.",
                    reading.path,
                    target_label(target),
                    example_path_for(intent)
                ),
                Some(format!("{base}.path")),
            ));
        }

        if intent == "ADJUST_DEAL_MARGIN" {
            issues.push(issue(
                "artf.mutation.no_openrtb_target",
                Severity::Warning,
                String::from(
                    "ADJUST_DEAL_MARGIN has no field in OpenRTB 2.6 to write to: margin is not \
                     part of the Deal object, so the orchestrator has to apply it out of band \
                     and no validator can check the result.",
                ),
                Some(base.clone()),
            ));
        }
    }
}

fn validate_mutation_intent(
    context: &RequestContext<'_>,
    reading: &MutationReading<'_>,
    base: &str,
    issues: &mut Vec<Issue>,
) {
    let path = Some(format!("{base}.intent"));

    match reading.intent {
        None => {
            issues.push(issue(
                "artf.mutation.intent_unknown",
                Severity::Error,
                format!(
                    "intent {} is not an ARTF Intent value. Known values: {}.",
                    reading.intent_rendered,
                    INTENTS.join(", ")
                ),
                path,
            ));
            return;
        }
        Some("INTENT_UNSPECIFIED") => {
            issues.push(issue(
                "artf.mutation.intent_unspecified",
                Severity::Error,
                String::from(
                    "intent is INTENT_UNSPECIFIED. Every mutation must declare why it is being \
                     proposed, or the orchestrator cannot evaluate it independently.",
                ),
                path,
            ));
            return;
        }
        Some(_) => {}
    }

    let intent = reading.intent.expect("checked above");
    if let Some(applicable) = context.applicable_intents.as_deref() {
        if !applicable.iter().any(|allowed| allowed == intent) {
            issues.push(issue(
                "artf.mutation.intent_not_applicable",
                Severity::Error,
                format!(
                    "Intent {intent} is not among the applicable_intents the request declared \
                     ({}). The orchestrator listed what it is prepared to accept; anything else \
                     is rejected on arrival.",
                    if applicable.is_empty() {
                        String::from("none")
                    } else {
                        applicable.join(", ")
                    }
                ),
                Some(format!("{base}.intent")),
            ));
        }
    }
}

fn validate_mutation_operation(reading: &MutationReading<'_>, base: &str, issues: &mut Vec<Issue>) {
    let path = Some(format!("{base}.op"));

    match reading.operation {
        None => issues.push(issue(
            "artf.mutation.op_unknown",
            Severity::Error,
            format!(
                "op {} is not an ARTF Operation value. Known values: {}.",
                reading.operation_rendered,
                OPERATIONS.join(", ")
            ),
            path,
        )),
        Some("OPERATION_UNSPECIFIED") => issues.push(issue(
            "artf.mutation.op_unspecified",
            Severity::Error,
            String::from(
                "op is OPERATION_UNSPECIFIED, so the orchestrator cannot tell whether to add, \
                 replace, or remove at the target path.",
            ),
            path,
        )),
        Some("OPERATION_REMOVE") => {
            if matches!(
                reading.intent,
                Some("ADJUST_DEAL_FLOOR" | "ADJUST_DEAL_MARGIN" | "BID_SHADE")
            ) {
                issues.push(issue(
                    "artf.mutation.op_not_applicable",
                    Severity::Warning,
                    format!(
                        "op is OPERATION_REMOVE on intent {}, which carries a new value rather \
                         than deleting one. OPERATION_REPLACE is the operation that adjusts a \
                         price or a floor.",
                        reading.intent.unwrap_or_default()
                    ),
                    path,
                ));
            }
        }
        Some(_) => {}
    }
}

/// Checks the semantic path and resolves it against the auction. Returns the
/// parsed target when the path was readable.
fn validate_mutation_path<'a>(
    context: &RequestContext<'_>,
    reading: &'a MutationReading<'a>,
    base: &str,
    issues: &mut Vec<Issue>,
) -> Option<Target<'a>> {
    let path_field = format!("{base}.path");

    if !reading.path_present || reading.path.is_empty() {
        issues.push(issue(
            "artf.mutation.path_missing",
            Severity::Error,
            String::from(
                "path is required: it is the semantic reference naming which auction entity the \
                 mutation applies to.",
            ),
            Some(path_field),
        ));
        return None;
    }

    if !reading.path.starts_with('/') {
        issues.push(issue(
            "artf.mutation.path_not_absolute",
            Severity::Error,
            format!(
                "path \"{}\" does not start with \"/\". ARTF semantic paths are rooted at the \
                 payload, like /imp/imp-1 or /user/data/segment.",
                reading.path
            ),
            Some(path_field.clone()),
        ));
        return None;
    }

    let target = Target::parse(reading.path);
    match target {
        Target::Unrecognized => {
            issues.push(issue(
                "artf.mutation.path_unrecognized",
                Severity::Warning,
                format!(
                    "path \"{}\" is not one of the semantic references ARTF v1.0 documents \
                     (/user/data/segment, /imp/{{imp id}}, /imp/{{imp id}}/pmp/deals/{{deal \
                     id}}, /seatbid/{{seat}}/bid/{{bid id}}), so it can only be resolved by \
                     prior agreement with the orchestrator.",
                    reading.path
                ),
                Some(path_field),
            ));
        }
        Target::UserSegments => {}
        Target::Imp { imp_id } => {
            if context.imp(imp_id).is_none() {
                issues.push(issue(
                    "artf.mutation.imp_unknown",
                    Severity::Error,
                    format!(
                        "path \"{}\" names impression \"{imp_id}\", which is not among the \
                         impressions the bid request carries ({}).",
                        reading.path,
                        rendered_ids(context.imps.iter().map(|imp| imp.id))
                    ),
                    Some(path_field),
                ));
            }
        }
        Target::Deal { imp_id, deal_id } => match context.imp(imp_id) {
            None => issues.push(issue(
                "artf.mutation.imp_unknown",
                Severity::Error,
                format!(
                    "path \"{}\" names impression \"{imp_id}\", which is not among the \
                     impressions the bid request carries ({}).",
                    reading.path,
                    rendered_ids(context.imps.iter().map(|imp| imp.id))
                ),
                Some(path_field),
            )),
            Some(imp) => {
                if !imp.deal_ids.contains(&deal_id) {
                    issues.push(issue(
                        "artf.mutation.deal_unknown",
                        Severity::Error,
                        format!(
                            "path \"{}\" names deal \"{deal_id}\", which impression \
                             \"{imp_id}\" does not offer ({}). A floor or margin can only be \
                             adjusted on a deal the auction already carries.",
                            reading.path,
                            rendered_ids(imp.deal_ids.iter().copied())
                        ),
                        Some(path_field),
                    ));
                }
            }
        },
        Target::Bid { seat, bid_id } => {
            if !context.has_bid_response {
                issues.push(issue(
                    "artf.mutation.target_payload_missing",
                    Severity::Error,
                    format!(
                        "path \"{}\" targets a bid, but the request envelope carries no \
                         bid_response to mutate.",
                        reading.path
                    ),
                    Some(path_field),
                ));
            } else if !context.seats.iter().any(|(candidate_seat, bid_ids)| {
                *candidate_seat == seat && bid_ids.contains(&bid_id)
            }) {
                issues.push(issue(
                    "artf.mutation.bid_unknown",
                    Severity::Error,
                    format!(
                        "path \"{}\" names bid \"{bid_id}\" from seat \"{seat}\", which the bid \
                         response does not contain.",
                        reading.path
                    ),
                    Some(path_field),
                ));
            }
        }
    }

    Some(target)
}

fn validate_mutation_payload(reading: &MutationReading<'_>, base: &str, issues: &mut Vec<Issue>) {
    let Some(intent) = reading.intent else {
        return;
    };
    let expected = payload_members_for(intent);

    if reading.payload_members.is_empty() {
        issues.push(issue(
            "artf.mutation.payload_missing",
            Severity::Error,
            format!(
                "Intent {intent} carries its value in {}, and the mutation sets none of the \
                 value oneof members.",
                rendered_ids(expected.iter().copied())
            ),
            Some(base.to_string()),
        ));
        return;
    }

    if reading.payload_members.len() > 1 {
        issues.push(issue(
            "artf.mutation.payload_ambiguous",
            Severity::Error,
            format!(
                "value is a oneof, so exactly one payload member may be set; this mutation sets \
                 {}.",
                rendered_ids(reading.payload_members.iter().copied())
            ),
            Some(base.to_string()),
        ));
        return;
    }

    let member = reading.payload_members[0];
    let member_matches = expected.contains(&member)
        || LEGACY_PAYLOAD_MEMBERS.contains(&member) && legacy_member_matches(intent, member);
    if !member_matches {
        issues.push(issue(
            "artf.mutation.payload_intent_mismatch",
            Severity::Error,
            format!(
                "Intent {intent} carries {}, but the mutation sets {member}.",
                rendered_ids(expected.iter().copied())
            ),
            Some(format!("{base}.{member}")),
        ));
        return;
    }

    let Some(payload) = reading.payload else {
        return;
    };
    let payload_path = format!("{base}.{member}");

    match member {
        "ids" | "IDsPayload" => {
            // The proto nests the list under `id`; the document's examples put
            // the bare array in the wrapper.
            let ids = payload
                .get("id")
                .and_then(Value::as_array)
                .or_else(|| payload.as_array());
            if ids.map_or(true, |ids| ids.is_empty()) {
                issues.push(issue(
                    "artf.mutation.ids_empty",
                    Severity::Error,
                    format!("Intent {intent} proposes no ids, so the mutation is a no-op."),
                    Some(payload_path),
                ));
            }
        }
        "adjust_deal" | "AdjustDealPayload" => {
            if let Some(bidfloor) = payload.get("bidfloor").and_then(Value::as_f64) {
                if bidfloor < 0.0 {
                    issues.push(issue(
                        "artf.mutation.bidfloor_negative",
                        Severity::Error,
                        format!("Proposed deal bidfloor {bidfloor} is negative."),
                        Some(format!("{payload_path}.bidfloor")),
                    ));
                }
            }
            validate_margin(payload.get("margin"), &payload_path, issues);
        }
        "adjust_bid" | "AdjustBidPayload" => match payload.get("price").and_then(Value::as_f64) {
            None => issues.push(issue(
                "artf.mutation.price_missing",
                Severity::Error,
                String::from("BID_SHADE carries the shaded price in adjust_bid.price."),
                Some(format!("{payload_path}.price")),
            )),
            Some(price) if price < 0.0 => issues.push(issue(
                "artf.mutation.price_negative",
                Severity::Error,
                format!("Proposed bid price {price} is negative."),
                Some(format!("{payload_path}.price")),
            )),
            Some(_) => {}
        },
        "metrics" | "AddMetricsPayload" => {
            let metrics = payload
                .get("metric")
                .and_then(Value::as_array)
                .or_else(|| payload.as_array());
            match metrics {
                None => {}
                Some(metrics) if metrics.is_empty() => issues.push(issue(
                    "artf.mutation.metrics_empty",
                    Severity::Error,
                    String::from("ADD_METRICS proposes no metrics, so the mutation is a no-op."),
                    Some(payload_path),
                )),
                Some(metrics) => {
                    for (index, metric) in metrics.iter().enumerate() {
                        if metric.get("type").and_then(Value::as_str).is_none() {
                            issues.push(issue(
                                "artf.mutation.metric_type_missing",
                                Severity::Error,
                                String::from(
                                    "OpenRTB requires Metric.type; a metric without one cannot \
                                     be interpreted by whoever receives the mutated request.",
                                ),
                                Some(format!("{payload_path}.metric[{index}].type")),
                            ));
                        }
                    }
                }
            }
        }
        _ => {}
    }
}

fn validate_margin(margin: Option<&Value>, payload_path: &str, issues: &mut Vec<Issue>) {
    let Some(margin) = margin.and_then(Value::as_object) else {
        return;
    };
    let Some(value) = margin.get("value").and_then(Value::as_f64) else {
        return;
    };

    let calculation_type = margin
        .get("calculation_type")
        .and_then(Value::as_str)
        .unwrap_or("CPM");

    if value < 0.0 {
        issues.push(issue(
            "artf.mutation.margin_negative",
            Severity::Error,
            format!("Proposed margin {value} is negative."),
            Some(format!("{payload_path}.margin.value")),
        ));
    } else if calculation_type == "PERCENT" && value > 100.0 {
        issues.push(issue(
            "artf.mutation.margin_implausible",
            Severity::Warning,
            format!(
                "Margin {value} is a PERCENT adjustment above 100, which would take the whole \
                 bid and more."
            ),
            Some(format!("{payload_path}.margin.value")),
        ));
    }
}

fn legacy_member_matches(intent: &str, member: &str) -> bool {
    matches!(
        (intent, member),
        (
            "ACTIVATE_SEGMENTS" | "ACTIVATE_DEALS" | "SUPPRESS_DEALS" | "ADD_CIDS",
            "IDsPayload"
        ) | (
            "ADJUST_DEAL_FLOOR" | "ADJUST_DEAL_MARGIN",
            "AdjustDealPayload"
        ) | ("BID_SHADE", "AdjustBidPayload")
            | ("ADD_METRICS", "AddMetricsPayload")
    )
}

// -------------------------------------------------------------------- apply --

fn apply_segments(bid_request: &mut Value, reading: &MutationReading<'_>) -> bool {
    let Some(ids) = payload_ids(reading) else {
        return false;
    };
    let Some(request) = bid_request.as_object_mut() else {
        return false;
    };

    let user = request
        .entry("user")
        .or_insert_with(|| json!({}))
        .as_object_mut();
    let Some(user) = user else {
        return false;
    };
    let data = user
        .entry("data")
        .or_insert_with(|| json!([]))
        .as_array_mut();
    let Some(data) = data else {
        return false;
    };

    match reading.operation {
        Some("OPERATION_REMOVE") => {
            for entry in data.iter_mut() {
                if let Some(segments) = entry.get_mut("segment").and_then(Value::as_array_mut) {
                    segments.retain(|segment| {
                        segment
                            .get("id")
                            .and_then(Value::as_str)
                            .map_or(true, |id| !ids.iter().any(|proposed| proposed == id))
                    });
                }
            }
            true
        }
        Some("OPERATION_ADD") | Some("OPERATION_REPLACE") => {
            let segments: Vec<Value> = ids.iter().map(|id| json!({ "id": id })).collect();
            if data.is_empty() {
                data.push(json!({ "segment": segments }));
                return true;
            }
            let Some(first) = data[0].as_object_mut() else {
                return false;
            };
            if matches!(reading.operation, Some("OPERATION_REPLACE")) {
                first.insert(String::from("segment"), Value::Array(segments));
                return true;
            }
            let existing = first
                .entry("segment")
                .or_insert_with(|| json!([]))
                .as_array_mut();
            let Some(existing) = existing else {
                return false;
            };
            for segment in segments {
                let id = segment["id"].as_str().unwrap_or_default().to_string();
                let already_there = existing
                    .iter()
                    .any(|entry| entry.get("id").and_then(Value::as_str) == Some(id.as_str()));
                if !already_there {
                    existing.push(segment);
                }
            }
            true
        }
        _ => false,
    }
}

fn apply_deal_ids(bid_request: &mut Value, imp_id: &str, reading: &MutationReading<'_>) -> bool {
    let Some(ids) = payload_ids(reading) else {
        return false;
    };
    let Some(imp) = imp_mut(bid_request, imp_id) else {
        return false;
    };

    let suppressing =
        reading.intent == Some("SUPPRESS_DEALS") || reading.operation == Some("OPERATION_REMOVE");

    if suppressing {
        let Some(deals) = imp
            .get_mut("pmp")
            .and_then(Value::as_object_mut)
            .and_then(|pmp| pmp.get_mut("deals"))
            .and_then(Value::as_array_mut)
        else {
            return false;
        };
        deals.retain(|deal| {
            deal.get("id")
                .and_then(Value::as_str)
                .map_or(true, |id| !ids.iter().any(|proposed| proposed == id))
        });
        return true;
    }

    let pmp = imp
        .entry("pmp")
        .or_insert_with(|| json!({}))
        .as_object_mut();
    let Some(pmp) = pmp else {
        return false;
    };
    let deals = pmp
        .entry("deals")
        .or_insert_with(|| json!([]))
        .as_array_mut();
    let Some(deals) = deals else {
        return false;
    };

    for id in ids {
        let already_there = deals
            .iter()
            .any(|deal| deal.get("id").and_then(Value::as_str) == Some(id.as_str()));
        if !already_there {
            deals.push(json!({ "id": id }));
        }
    }
    true
}

fn apply_deal_floor(
    bid_request: &mut Value,
    imp_id: &str,
    deal_id: &str,
    reading: &MutationReading<'_>,
) -> bool {
    let Some(payload) = reading.payload else {
        return false;
    };
    let Some(bidfloor) = payload.get("bidfloor").and_then(Value::as_f64) else {
        return false;
    };
    let Some(imp) = imp_mut(bid_request, imp_id) else {
        return false;
    };
    let Some(deals) = imp
        .get_mut("pmp")
        .and_then(Value::as_object_mut)
        .and_then(|pmp| pmp.get_mut("deals"))
        .and_then(Value::as_array_mut)
    else {
        return false;
    };

    for deal in deals.iter_mut() {
        if deal.get("id").and_then(Value::as_str) == Some(deal_id) {
            if let Some(deal) = deal.as_object_mut() {
                deal.insert(String::from("bidfloor"), json!(bidfloor));
                return true;
            }
        }
    }
    false
}

fn apply_metrics(bid_request: &mut Value, imp_id: &str, reading: &MutationReading<'_>) -> bool {
    let Some(payload) = reading.payload else {
        return false;
    };
    let proposed = payload
        .get("metric")
        .and_then(Value::as_array)
        .or_else(|| payload.as_array());
    let Some(proposed) = proposed else {
        return false;
    };
    let Some(imp) = imp_mut(bid_request, imp_id) else {
        return false;
    };

    let metrics = imp
        .entry("metric")
        .or_insert_with(|| json!([]))
        .as_array_mut();
    let Some(metrics) = metrics else {
        return false;
    };
    metrics.extend(proposed.iter().cloned());
    true
}

fn apply_bid_price(
    bid_response: &mut Value,
    seat: &str,
    bid_id: &str,
    reading: &MutationReading<'_>,
) -> bool {
    let Some(payload) = reading.payload else {
        return false;
    };
    let Some(price) = payload.get("price").and_then(Value::as_f64) else {
        return false;
    };
    let Some(seatbids) = bid_response
        .get_mut("seatbid")
        .and_then(Value::as_array_mut)
    else {
        return false;
    };

    for seatbid in seatbids.iter_mut() {
        if seatbid.get("seat").and_then(Value::as_str) != Some(seat) {
            continue;
        }
        let Some(bids) = seatbid.get_mut("bid").and_then(Value::as_array_mut) else {
            continue;
        };
        for bid in bids.iter_mut() {
            if bid.get("id").and_then(Value::as_str) == Some(bid_id) {
                if let Some(bid) = bid.as_object_mut() {
                    bid.insert(String::from("price"), json!(price));
                    return true;
                }
            }
        }
    }
    false
}

fn imp_mut<'a>(bid_request: &'a mut Value, imp_id: &str) -> Option<&'a mut Map<String, Value>> {
    bid_request
        .get_mut("imp")?
        .as_array_mut()?
        .iter_mut()
        .find(|imp| imp.get("id").and_then(Value::as_str) == Some(imp_id))?
        .as_object_mut()
}

fn payload_ids(reading: &MutationReading<'_>) -> Option<Vec<String>> {
    let payload = reading.payload?;
    let ids = payload
        .get("id")
        .and_then(Value::as_array)
        .or_else(|| payload.as_array())?;
    let ids: Vec<String> = ids
        .iter()
        .filter_map(Value::as_str)
        .map(String::from)
        .collect();
    (!ids.is_empty()).then_some(ids)
}

// ------------------------------------------------------------------ helpers --

enum EnumRead {
    Missing,
    Known(&'static str),
    /// Rendered as it appeared, quoted when it was a string.
    Unknown(String),
}

/// Reads a protobuf enum from JSON, which carries either the value name or its
/// number.
fn read_enum(value: Option<&Value>, names: &[&'static str]) -> EnumRead {
    match value {
        None | Some(Value::Null) => EnumRead::Missing,
        Some(Value::String(name)) => match names.iter().copied().find(|known| *known == name) {
            Some(known) => EnumRead::Known(known),
            None => EnumRead::Unknown(format!("\"{name}\"")),
        },
        Some(Value::Number(number)) => match number
            .as_u64()
            .and_then(|index| names.get(index as usize).copied())
        {
            Some(known) => EnumRead::Known(known),
            None => EnumRead::Unknown(number.to_string()),
        },
        Some(other) => EnumRead::Unknown(other.to_string()),
    }
}

fn target_label(target: Target<'_>) -> &'static str {
    match target {
        Target::UserSegments => "the user's segments",
        Target::Imp { .. } => "an impression",
        Target::Deal { .. } => "a deal",
        Target::Bid { .. } => "a bid",
        Target::Unrecognized => "nothing ARTF documents",
    }
}

fn example_path_for(intent: &str) -> &'static str {
    match intent {
        "ACTIVATE_SEGMENTS" | "ADD_CIDS" => "/user/data/segment",
        "ACTIVATE_DEALS" | "SUPPRESS_DEALS" | "ADD_METRICS" => "/imp/{imp id}",
        "ADJUST_DEAL_FLOOR" | "ADJUST_DEAL_MARGIN" => "/imp/{imp id}/pmp/deals/{deal id}",
        "BID_SHADE" => "/seatbid/{seat}/bid/{bid id}",
        _ => "a semantic reference into the payload",
    }
}

fn rendered_ids<'a>(ids: impl Iterator<Item = &'a str>) -> String {
    let rendered: Vec<String> = ids.map(|id| format!("\"{id}\"")).collect();
    if rendered.is_empty() {
        String::from("none")
    } else {
        rendered.join(", ")
    }
}

fn pretty(value: &Value) -> String {
    serde_json::to_string_pretty(value).unwrap_or_else(|_| String::from("{}"))
}

fn issue(id: &str, severity: Severity, message: String, path: Option<String>) -> Issue {
    Issue {
        id: String::from(id),
        severity,
        message,
        path,
        section: None,
    }
}
