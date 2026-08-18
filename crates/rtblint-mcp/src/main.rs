//! MCP server for RTBlint.
//!
//! Speaks the Model Context Protocol over stdio (newline-delimited JSON-RPC
//! 2.0) and exposes the rtblint-core OpenRTB validator as callable tools:
//! `validate_bid_request`, `validate_bid_response`, `validate_artf_request`,
//! `validate_artf_response`, `list_openrtb_versions`, and
//! `get_adcp_capabilities` for AdCP protocol discovery.
//!
//! The ARTF tools are the guardrail an agent calls around its own work: check
//! the envelope it was handed, then check the mutation set it is about to
//! propose against the auction it targets, before the orchestrator sees it.

use std::io::{self, BufRead, Write};

use serde_json::{json, Value};

use rtblint_core::{
    validate_artf_mutations_applied, validate_artf_request, validate_artf_response_against_request,
    validate_bid_request_with_profile, validate_bid_response_against_request_with_profile,
    validate_bid_response_with_profile, Dialect, OpenRtbVersion, Profile,
};

const DEFAULT_VERSION: OpenRtbVersion = OpenRtbVersion::V2_6_202606;
const PROTOCOL_VERSION: &str = "2024-11-05";

/// Release-precision AdCP version this server serves. Kept in step with the
/// hosted worker at rtblint.org/mcp: both surfaces must answer discovery
/// identically or an agent gets different answers depending on transport.
const SERVED_ADCP_VERSION: &str = "3.1";
const SERVED_ADCP_MAJOR: u64 = 3;

fn main() {
    let stdin = io::stdin();
    let stdout = io::stdout();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(line) => line,
            Err(_) => break,
        };
        if line.trim().is_empty() {
            continue;
        }

        let message: Value = match serde_json::from_str(&line) {
            Ok(message) => message,
            Err(error) => {
                write_message(
                    &stdout,
                    &error_response(Value::Null, -32700, &format!("Parse error: {error}")),
                );
                continue;
            }
        };

        let id = message.get("id").cloned();
        let method = message.get("method").and_then(Value::as_str).unwrap_or("");
        let params = message.get("params").cloned().unwrap_or(Value::Null);

        let Some(id) = id else {
            // Notifications (initialized, cancelled, ...) need no response.
            continue;
        };

        let response = match method {
            "initialize" => initialize_response(id, &params),
            "ping" => success_response(id, json!({})),
            "tools/list" => success_response(id, json!({ "tools": tool_definitions() })),
            "tools/call" => handle_tool_call(id, &params),
            _ => error_response(id, -32601, &format!("Method not found: {method}")),
        };

        write_message(&stdout, &response);
    }
}

fn initialize_response(id: Value, params: &Value) -> Value {
    let requested = params
        .get("protocolVersion")
        .and_then(Value::as_str)
        .unwrap_or(PROTOCOL_VERSION);

    success_response(
        id,
        json!({
            "protocolVersion": requested,
            "capabilities": { "tools": {} },
            "serverInfo": {
                "name": "rtblint-mcp",
                "version": env!("CARGO_PKG_VERSION"),
            },
        }),
    )
}

fn tool_definitions() -> Value {
    let version_property = || {
        json!({
            "type": "string",
            "description": format!(
                "OpenRTB version id to validate against (default {}). One of: {}",
                DEFAULT_VERSION.id(),
                version_ids().join(", ")
            ),
        })
    };
    let dialect_property = || {
        json!({
            "type": "string",
            "enum": ["spec-json", "proto-json"],
            "description": "JSON dialect the payload is written in. spec-json (default) types flag fields such as imp.secure, regs.coppa and pmp.private_auction as integers, the way the OpenRTB specification does. proto-json follows the IAB OpenRTB protobuf schema, which declares 28 of those fields bool, so true/false is correct there and an integer is the error. Use proto-json for anything that came off a gRPC bidstream integration.",
        })
    };
    let profile_property = || {
        json!({
            "type": "string",
            "enum": ["spec", "google-ab", "prebid-server", "xandr", "magnite"],
            "description": "Exchange profile applied on top of the spec. spec (default) is the specification only. google-ab is Google Authorized Buyers OpenRTB: at=3 (FIXED_PRICE) is a valid auction type, and each Imp must carry ext.billing_id. prebid-server is Prebid Server /openrtb2/auction: each Imp must name a bidder or a stored request, wseat/bseat are refused, and stored-request objects need id. xandr is Microsoft Monetize outgoing requests: ext.appnexus.seller_member_id and video.ext.appnexus.context. magnite is Magnite xAPI identity fields: imp.ext.rp.zone_id, site/app ext.rp.site_id, publisher.ext.rp.account_id.",
        })
    };
    let payload_schema = |payload_description: &str| {
        json!({
            "type": "object",
            "properties": {
                "payload": {
                    "type": "string",
                    "description": payload_description,
                },
                "version": version_property(),
                "dialect": dialect_property(),
                "profile": profile_property(),
            },
            "required": ["payload"],
        })
    };

    json!([
        {
            "name": "validate_bid_request",
            "description": "Validate an OpenRTB 2.x bid request JSON payload against a tracked spec version. Returns structured issues with rule ids, severities, and JSON paths.",
            "inputSchema": payload_schema("The OpenRTB bid request as a raw JSON string."),
        },
        {
            "name": "validate_bid_response",
            "description": "Validate an OpenRTB 2.x bid response JSON payload against a tracked spec version. Optionally cross-validate it against the originating bid request (impid, mtype, adm markup, dealid, seat, and currency coherence). Returns structured issues with rule ids, severities, and JSON paths.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "payload": {
                        "type": "string",
                        "description": "The OpenRTB bid response as a raw JSON string.",
                    },
                    "bid_request": {
                        "type": "string",
                        "description": "Optional: the originating OpenRTB bid request as a raw JSON string. When supplied, every bid is also cross-checked against the Imp it references.",
                    },
                    "version": version_property(),
                    "dialect": dialect_property(),
                    "profile": profile_property(),
                },
                "required": ["payload"],
            },
        },
        {
            "name": "validate_artf_request",
            "description": "Validate an ARTF (IAB Tech Lab Agentic Real Time Framework) RTBRequest envelope: required members, lifecycle and payload coherence, tmax plausibility, originator and applicable_intents enums, plus full OpenRTB validation of the bid request and bid response it carries. The carried payloads are protobuf JSON, so they are validated in that dialect.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "payload": {
                        "type": "string",
                        "description": "The ARTF RTBRequest envelope as a raw JSON string.",
                    },
                    "version": version_property(),
                },
                "required": ["payload"],
            },
        },
        {
            "name": "validate_artf_response",
            "description": "Validate an ARTF RTBResponse mutation set against the RTBRequest it answers: envelope id echo, declared intent against applicable_intents, operation and payload coherence, and whether each semantic path (/imp/{id}, /imp/{id}/pmp/deals/{id}, /user/data/segment, /seatbid/{seat}/bid/{id}) resolves to something the auction actually carries. With apply=true the mutations are written into the payloads and revalidated, reporting the OpenRTB findings the mutations introduced. Call this before proposing mutations to an orchestrator.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "payload": {
                        "type": "string",
                        "description": "The ARTF RTBResponse as a raw JSON string.",
                    },
                    "rtb_request": {
                        "type": "string",
                        "description": "The ARTF RTBRequest envelope this response answers, as a raw JSON string. Required: a mutation is only meaningful relative to the auction it targets.",
                    },
                    "apply": {
                        "type": "boolean",
                        "description": "Apply the mutations and revalidate the result (default false). Returns the mutated payloads and the findings the mutations introduced, with pre-existing findings filtered out.",
                    },
                    "version": version_property(),
                },
                "required": ["payload", "rtb_request"],
            },
        },
        {
            "name": "list_openrtb_versions",
            "description": "List every OpenRTB version id this build can validate against.",
            "inputSchema": { "type": "object", "properties": {} },
        },
        {
            "name": "get_adcp_capabilities",
            "description": "AdCP protocol discovery. Returns the AdCP releases this agent speaks and the bid-stream conformance metrics it computes. Call this first when wiring rtblint into an agentic buying pipeline: it declares the experimental measurement protocol and the metric ids (openrtb_error_count, openrtb_warning_count, openrtb_conformance_rate) that validate_bid_request and validate_bid_response produce. Part of the Ad Context Protocol (AdCP 3.1) specification.",
            "inputSchema": {
                "$schema": "http://json-schema.org/draft-07/schema#",
                "title": "GetAdcpCapabilitiesInput",
                "type": "object",
                "properties": {
                    "adcp_version": {
                        "type": "string",
                        "description": "Release-precision AdCP version the caller pins (for example \"3.1\"). When the pin is not in supported_versions the call returns a VERSION_UNSUPPORTED error naming the releases that would work. When omitted, the highest supported release is served.",
                    },
                    "adcp_major_version": {
                        "type": "integer",
                        "description": "Deprecated in favour of adcp_version. AdCP major version the caller's payloads conform to. When omitted, assumes the highest supported major.",
                    },
                    "protocols": {
                        "type": "array",
                        "items": { "type": "string" },
                        "description": "Filter which per-protocol capability blocks are returned. Does not narrow supported_protocols, which declares what the agent implements.",
                    },
                    "context": {
                        "type": "object",
                        "description": "Caller-supplied context object. Echoed back unchanged in the response.",
                    },
                    "idempotency_key": {
                        "type": "string",
                        "description": "AdCP 3.1 carries an idempotency key on every task request, reads included. This agent has no mutating surface and no replay store, so the key is accepted and ignored rather than rejected as an unknown field.",
                    },
                    "ext": {
                        "type": "object",
                        "description": "Caller extension object. Accepted and ignored; declared so envelope fields do not trip strict request-wrapper validation.",
                    },
                },
            },
        },
    ])
}

/// Metrics published under the experimental `measurement` protocol.
///
/// rtblint is not a transacting agent: it does not plan, negotiate or deliver.
/// It computes quantitative properties of a bid-stream payload, which is what
/// the measurement block is for. Nothing here claims viewability, attention or
/// outcome measurement.
fn measurement_metrics() -> Value {
    const OPENRTB_SPEC: &str =
        "https://github.com/InteractiveAdvertisingBureau/openrtb2.x/blob/main/2.6.md";
    json!([
        {
            "metric_id": "openrtb_error_count",
            "unit": "count",
            "description": "Number of spec errors in a single OpenRTB payload, validated against a pinned release. An error is a construct the specification forbids: a missing required field, a value outside a defined enumeration, or an object at a path the release does not define.",
            "standard_reference": OPENRTB_SPEC,
            "methodology_url": "https://rtblint.org/docs/what-rtblint-checks/",
        },
        {
            "metric_id": "openrtb_warning_count",
            "unit": "count",
            "description": "Number of spec warnings in a single OpenRTB payload: deprecated fields still in use, fields that moved path between releases, and values that are legal but incoherent with the rest of the payload. Warnings do not make a payload invalid.",
            "standard_reference": OPENRTB_SPEC,
            "methodology_url": "https://rtblint.org/docs/rule-reference/",
        },
        {
            "metric_id": "openrtb_conformance_rate",
            "unit": "percent",
            "description": "Share of payloads in a sample that validate with zero errors against a pinned OpenRTB release. Computed over a caller-supplied sample; rtblint does not observe live traffic, so this metric is only as representative as the sample given to it.",
            "standard_reference": OPENRTB_SPEC,
            "methodology_url": "https://rtblint.org/docs/what-rtblint-checks/",
        },
    ])
}

/// `get_adcp_capabilities`.
///
/// `supported_protocols` claims only `measurement`, which is experimental in
/// 3.1 and scoped to capability discovery. Declaring a protocol commits the
/// agent to that protocol's baseline compliance storyboard, and rtblint
/// implements none of the transactional surfaces, so claiming media_buy or
/// creative would be a claim it cannot pass. Experimental surfaces must also be
/// listed in `experimental_features`, hence `measurement.core`.
fn get_adcp_capabilities(arguments: &Value) -> Value {
    let version_unsupported = |message: String| {
        tool_result_text(
            &serde_json::to_string_pretty(&json!({
                "status": "failed",
                "adcp_version": SERVED_ADCP_VERSION,
                "adcp_error": {
                    "code": "VERSION_UNSUPPORTED",
                    "message": message,
                    "details": {
                        "adcp_version": SERVED_ADCP_VERSION,
                        "supported_versions": [SERVED_ADCP_VERSION],
                    },
                },
            }))
            .unwrap_or_default(),
            true,
        )
    };

    if let Some(pin) = arguments.get("adcp_version").and_then(Value::as_str) {
        if pin != SERVED_ADCP_VERSION {
            return version_unsupported(format!("AdCP release {pin} is not served by this agent."));
        }
    }

    if let Some(major) = arguments.get("adcp_major_version").and_then(Value::as_u64) {
        if major != SERVED_ADCP_MAJOR {
            return version_unsupported(format!(
                "AdCP major version {major} is not served by this agent."
            ));
        }
    }

    let requested: Option<Vec<&str>> = arguments
        .get("protocols")
        .and_then(Value::as_array)
        .map(|values| values.iter().filter_map(Value::as_str).collect());
    let include_measurement = requested
        .as_ref()
        .map(|names| names.contains(&"measurement"))
        .unwrap_or(true);

    let mut payload = json!({
        // Required on every task response, including synchronous metadata
        // responses such as this one.
        "status": "completed",
        "adcp_version": SERVED_ADCP_VERSION,
        "adcp": {
            // Deprecated in favour of supported_versions, but servers must keep
            // emitting it through 3.x.
            "major_versions": [SERVED_ADCP_MAJOR],
            "supported_versions": [SERVED_ADCP_VERSION],
            // No mutating task surface, so there is nothing to replay-guard.
            "idempotency": { "supported": false },
        },
        "operator": { "name": "rtblint", "domain": "rtblint.org" },
        "supported_protocols": ["measurement"],
        "experimental_features": ["measurement.core"],
    });

    if include_measurement {
        payload["measurement"] = json!({ "metrics": measurement_metrics() });
    }
    if let Some(context) = arguments.get("context") {
        payload["context"] = context.clone();
    }

    tool_result_text(
        &serde_json::to_string_pretty(&payload).unwrap_or_default(),
        false,
    )
}

fn handle_tool_call(id: Value, params: &Value) -> Value {
    let tool_name = params.get("name").and_then(Value::as_str).unwrap_or("");
    let arguments = params.get("arguments").cloned().unwrap_or(json!({}));

    match tool_name {
        "validate_bid_request" => run_validation(id, &arguments, "request"),
        "validate_bid_response" => run_validation(id, &arguments, "response"),
        "validate_artf_request" => run_artf_request(id, &arguments),
        "validate_artf_response" => run_artf_response(id, &arguments),
        "list_openrtb_versions" => success_response(
            id,
            tool_result_text(
                &serde_json::to_string_pretty(&json!({ "versions": version_ids() }))
                    .unwrap_or_default(),
                false,
            ),
        ),
        "get_adcp_capabilities" => success_response(id, get_adcp_capabilities(&arguments)),
        _ => error_response(id, -32602, &format!("Unknown tool: {tool_name}")),
    }
}

fn run_validation(id: Value, arguments: &Value, payload_type: &str) -> Value {
    let Some(payload) = arguments.get("payload").and_then(Value::as_str) else {
        return success_response(
            id,
            tool_result_text("Missing required argument: payload (string).", true),
        );
    };

    let version = match resolve_version(arguments) {
        Ok(version) => version,
        Err(message) => return success_response(id, tool_result_text(&message, true)),
    };

    let dialect = match arguments.get("dialect").and_then(Value::as_str) {
        None => Dialect::SpecJson,
        Some(dialect_id) => match Dialect::from_id(dialect_id) {
            Some(dialect) => dialect,
            None => {
                return success_response(
                    id,
                    tool_result_text(
                        &format!(
                            "Unsupported dialect: {dialect_id}. Use one of: spec-json, proto-json"
                        ),
                        true,
                    ),
                );
            }
        },
    };

    let profile = match arguments.get("profile").and_then(Value::as_str) {
        None => Profile::Spec,
        Some(profile_id) => match Profile::from_id(profile_id) {
            Some(profile) => profile,
            None => {
                return success_response(
                    id,
                    tool_result_text(
                        &format!(
                            "Unsupported profile: {profile_id}. Use one of: {}",
                            Profile::ids().join(", ")
                        ),
                        true,
                    ),
                );
            }
        },
    };

    let bid_request = arguments.get("bid_request").and_then(Value::as_str);
    let result = if payload_type == "response" {
        match bid_request {
            Some(request) => validate_bid_response_against_request_with_profile(
                version, dialect, profile, request, payload,
            ),
            None => validate_bid_response_with_profile(version, dialect, profile, payload),
        }
    } else {
        validate_bid_request_with_profile(version, dialect, profile, payload)
    };

    let report = json!({
        "version": version.id(),
        "payload_type": payload_type,
        "dialect": dialect.as_str(),
        "profile": profile.as_str(),
        "valid": result.valid,
        "issues": result.issues,
    });

    success_response(
        id,
        tool_result_text(
            &serde_json::to_string_pretty(&report).unwrap_or_default(),
            false,
        ),
    )
}

fn run_artf_request(id: Value, arguments: &Value) -> Value {
    let Some(payload) = arguments.get("payload").and_then(Value::as_str) else {
        return success_response(
            id,
            tool_result_text("Missing required argument: payload (string).", true),
        );
    };
    let version = match resolve_version(arguments) {
        Ok(version) => version,
        Err(message) => return success_response(id, tool_result_text(&message, true)),
    };

    let result = validate_artf_request(version, payload);
    let report = json!({
        "version": version.id(),
        "payload_type": "artf-request",
        "valid": result.valid,
        "issues": result.issues,
    });

    success_response(
        id,
        tool_result_text(
            &serde_json::to_string_pretty(&report).unwrap_or_default(),
            false,
        ),
    )
}

fn run_artf_response(id: Value, arguments: &Value) -> Value {
    let Some(payload) = arguments.get("payload").and_then(Value::as_str) else {
        return success_response(
            id,
            tool_result_text("Missing required argument: payload (string).", true),
        );
    };
    let Some(rtb_request) = arguments.get("rtb_request").and_then(Value::as_str) else {
        return success_response(
            id,
            tool_result_text(
                "Missing required argument: rtb_request (string). A mutation set can only be \
                 checked against the RTBRequest envelope it answers.",
                true,
            ),
        );
    };
    let version = match resolve_version(arguments) {
        Ok(version) => version,
        Err(message) => return success_response(id, tool_result_text(&message, true)),
    };

    let apply = arguments
        .get("apply")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    let report = if apply {
        let outcome = validate_artf_mutations_applied(version, rtb_request, payload);
        json!({
            "version": version.id(),
            "payload_type": "artf-response",
            "applied": true,
            "valid": outcome.result.valid,
            "issues": outcome.result.issues,
            "application": outcome.application,
        })
    } else {
        let result = validate_artf_response_against_request(version, rtb_request, payload);
        json!({
            "version": version.id(),
            "payload_type": "artf-response",
            "applied": false,
            "valid": result.valid,
            "issues": result.issues,
        })
    };

    success_response(
        id,
        tool_result_text(
            &serde_json::to_string_pretty(&report).unwrap_or_default(),
            false,
        ),
    )
}

fn resolve_version(arguments: &Value) -> Result<OpenRtbVersion, String> {
    match arguments.get("version").and_then(Value::as_str) {
        None => Ok(DEFAULT_VERSION),
        Some(version_id) => OpenRtbVersion::from_id(version_id).ok_or_else(|| {
            format!(
                "Unsupported OpenRTB version: {version_id}. Available versions: {}",
                version_ids().join(", ")
            )
        }),
    }
}

fn tool_result_text(text: &str, is_error: bool) -> Value {
    json!({
        "content": [{ "type": "text", "text": text }],
        "isError": is_error,
    })
}

fn version_ids() -> Vec<&'static str> {
    OpenRtbVersion::all()
        .iter()
        .map(|version| version.id())
        .collect()
}

fn success_response(id: Value, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

fn error_response(id: Value, code: i64, message: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": { "code": code, "message": message },
    })
}

fn write_message(stdout: &io::Stdout, message: &Value) {
    let mut handle = stdout.lock();
    if let Ok(serialized) = serde_json::to_string(message) {
        let _ = handle.write_all(serialized.as_bytes());
        let _ = handle.write_all(b"\n");
        let _ = handle.flush();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The tool result inside a JSON-RPC success response.
    fn tool_result(response: &Value) -> &Value {
        &response["result"]
    }

    /// Parse the JSON body out of a tool result envelope.
    fn payload_of(result: &Value) -> Value {
        let text = result["content"][0]["text"].as_str().expect("text content");
        serde_json::from_str(text).expect("tool payload is JSON")
    }

    const ARTF_REQUEST: &str = r#"{
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

    #[test]
    fn every_declared_tool_has_a_handler() {
        for tool in tool_definitions().as_array().expect("tool array") {
            let name = tool["name"].as_str().expect("tool name");
            let response = handle_tool_call(json!(1), &json!({ "name": name }));
            assert!(
                response.get("error").is_none(),
                "{name} is declared but not routed: {response}"
            );
        }
    }

    #[test]
    fn dialect_argument_switches_the_flag_encoding() {
        let payload = r#"{
            "id": "req-1",
            "imp": [{ "id": "imp-1", "secure": true, "banner": { "w": 300, "h": 250 } }],
            "site": { "id": "s-1" }
        }"#;

        let spec = run_validation(json!(1), &json!({ "payload": payload }), "request");
        let spec = payload_of(tool_result(&spec));
        assert_eq!(spec["valid"], json!(false));
        assert_eq!(spec["dialect"], "spec-json");
        assert_eq!(spec["profile"], "spec");

        let proto = run_validation(
            json!(2),
            &json!({ "payload": payload, "dialect": "proto-json" }),
            "request",
        );
        let proto = payload_of(tool_result(&proto));
        assert_eq!(proto["valid"], json!(true));
        assert_eq!(proto["dialect"], "proto-json");
    }

    #[test]
    fn unknown_dialect_is_reported_as_a_tool_error() {
        let result = run_validation(
            json!(1),
            &json!({ "payload": "{}", "dialect": "yaml" }),
            "request",
        );
        assert_eq!(tool_result(&result)["isError"], json!(true));
    }

    #[test]
    fn profile_argument_allows_google_fixed_price() {
        let payload = r#"{
            "id": "req-1",
            "at": 3,
            "imp": [{ "id": "1", "banner": { "w": 300, "h": 250 }, "ext": { "billing_id": ["1"] } }]
        }"#;

        let spec = run_validation(json!(1), &json!({ "payload": payload }), "request");
        let spec = payload_of(tool_result(&spec));
        assert_eq!(spec["valid"], json!(false));

        let google = run_validation(
            json!(2),
            &json!({ "payload": payload, "profile": "google-ab" }),
            "request",
        );
        let google = payload_of(tool_result(&google));
        assert_eq!(google["valid"], json!(true));
        assert_eq!(google["profile"], "google-ab");
    }

    #[test]
    fn profile_argument_requires_a_prebid_bidder() {
        let payload = r#"{
            "id": "req-1",
            "imp": [{ "id": "1", "banner": { "w": 300, "h": 250 } }]
        }"#;

        let spec = run_validation(json!(1), &json!({ "payload": payload }), "request");
        let spec = payload_of(tool_result(&spec));
        assert_eq!(spec["valid"], json!(true));

        let prebid = run_validation(
            json!(2),
            &json!({ "payload": payload, "profile": "prebid" }),
            "request",
        );
        let prebid = payload_of(tool_result(&prebid));
        assert_eq!(prebid["valid"], json!(false));
        assert_eq!(prebid["profile"], "prebid-server");
    }

    #[test]
    fn unknown_profile_is_reported_as_a_tool_error() {
        let result = run_validation(
            json!(1),
            &json!({ "payload": "{}", "profile": "amazon-tam" }),
            "request",
        );
        assert_eq!(tool_result(&result)["isError"], json!(true));
    }

    #[test]
    fn artf_request_tool_validates_the_envelope() {
        let result = run_artf_request(json!(1), &json!({ "payload": ARTF_REQUEST }));
        let payload = payload_of(tool_result(&result));

        assert_eq!(payload["payload_type"], "artf-request");
        assert_eq!(
            payload["valid"],
            json!(true),
            "issues: {}",
            payload["issues"]
        );
    }

    #[test]
    fn artf_response_tool_requires_the_request_it_answers() {
        let result = run_artf_response(json!(1), &json!({ "payload": "{}" }));
        assert_eq!(tool_result(&result)["isError"], json!(true));
    }

    #[test]
    fn artf_response_tool_resolves_mutation_paths() {
        let mutations = r#"{
            "id": "ep-1",
            "mutations": [
                {
                    "intent": "ACTIVATE_DEALS",
                    "op": "OPERATION_ADD",
                    "path": "/imp/imp-404",
                    "ids": { "id": ["deal-2"] }
                }
            ],
            "metadata": { "api_version": "1.0.0", "model_version": "m" }
        }"#;

        let result = run_artf_response(
            json!(1),
            &json!({ "payload": mutations, "rtb_request": ARTF_REQUEST }),
        );
        let payload = payload_of(tool_result(&result));

        assert_eq!(payload["valid"], json!(false));
        assert_eq!(payload["applied"], json!(false));
        let ids: Vec<&str> = payload["issues"]
            .as_array()
            .expect("issues")
            .iter()
            .map(|issue| issue["id"].as_str().expect("id"))
            .collect();
        assert!(ids.contains(&"artf.mutation.imp_unknown"), "{ids:?}");
    }

    #[test]
    fn artf_response_tool_returns_the_mutated_payload_when_applying() {
        let mutations = r#"{
            "id": "ep-1",
            "mutations": [
                {
                    "intent": "ACTIVATE_DEALS",
                    "op": "OPERATION_ADD",
                    "path": "/imp/imp-1",
                    "ids": { "id": ["deal-2"] }
                }
            ],
            "metadata": { "api_version": "1.0.0", "model_version": "m" }
        }"#;

        let result = run_artf_response(
            json!(1),
            &json!({ "payload": mutations, "rtb_request": ARTF_REQUEST, "apply": true }),
        );
        let payload = payload_of(tool_result(&result));

        assert_eq!(payload["applied"], json!(true));
        assert_eq!(
            payload["valid"],
            json!(true),
            "issues: {}",
            payload["issues"]
        );
        assert_eq!(payload["application"]["applied"], json!([0]));
        let mutated = payload["application"]["bid_request"]
            .as_str()
            .expect("mutated bid request");
        assert!(mutated.contains("deal-2"), "{mutated}");
    }

    #[test]
    fn capabilities_declare_only_what_the_agent_implements() {
        let result = get_adcp_capabilities(&json!({}));
        assert_eq!(result["isError"], json!(false));
        let payload = payload_of(&result);

        // status is required on every task response, including synchronous
        // metadata responses. Omitting it is non-conformant regardless of
        // whether the body validates.
        assert_eq!(payload["status"], "completed");
        assert_eq!(payload["adcp_version"], SERVED_ADCP_VERSION);
        assert_eq!(
            payload["adcp"]["supported_versions"],
            json!([SERVED_ADCP_VERSION])
        );

        // Claiming a protocol commits the agent to its compliance storyboard,
        // so the list stays at the one surface rtblint actually serves.
        assert_eq!(payload["supported_protocols"], json!(["measurement"]));
        assert_eq!(
            payload["experimental_features"],
            json!(["measurement.core"])
        );

        let metrics = payload["measurement"]["metrics"]
            .as_array()
            .expect("metrics array");
        assert_eq!(metrics.len(), 3);
        for metric in metrics {
            let id = metric["metric_id"].as_str().expect("metric_id");
            // vendor-metric-id: lowercase, digits and underscores, max 64.
            assert!(id.len() <= 64, "{id} exceeds the 64 char limit");
            assert!(
                id.chars().next().is_some_and(|c| c.is_ascii_lowercase()),
                "{id} must start with a lowercase letter"
            );
            assert!(
                id.chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_'),
                "{id} has characters outside the vendor-metric-id pattern"
            );
        }
    }

    #[test]
    fn unsupported_release_pin_returns_typed_error() {
        let result = get_adcp_capabilities(&json!({ "adcp_version": "3.0" }));
        assert_eq!(result["isError"], json!(true));
        let payload = payload_of(&result);

        assert_eq!(payload["status"], "failed");
        assert_eq!(payload["adcp_error"]["code"], "VERSION_UNSUPPORTED");
        // The buyer re-pins from this list rather than making a second
        // capabilities call.
        assert_eq!(
            payload["adcp_error"]["details"]["supported_versions"],
            json!([SERVED_ADCP_VERSION])
        );
    }

    #[test]
    fn unsupported_major_pin_returns_typed_error() {
        let result = get_adcp_capabilities(&json!({ "adcp_major_version": 2 }));
        assert_eq!(result["isError"], json!(true));
        assert_eq!(
            payload_of(&result)["adcp_error"]["code"],
            "VERSION_UNSUPPORTED"
        );
    }

    #[test]
    fn protocol_filter_scopes_blocks_without_narrowing_the_claim() {
        let result = get_adcp_capabilities(&json!({ "protocols": ["governance"] }));
        let payload = payload_of(&result);

        // The filter drops the block it did not ask for...
        assert!(payload.get("measurement").is_none());
        // ...but supported_protocols declares what the agent implements and is
        // constant across calls.
        assert_eq!(payload["supported_protocols"], json!(["measurement"]));
    }

    #[test]
    fn context_is_echoed_unchanged() {
        let context = json!({ "trace": "abc", "nested": { "n": 1 } });
        let result = get_adcp_capabilities(&json!({ "context": context.clone() }));
        assert_eq!(payload_of(&result)["context"], context);
    }
}
