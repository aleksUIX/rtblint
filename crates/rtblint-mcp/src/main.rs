//! MCP server for RTBlint.
//!
//! Speaks the Model Context Protocol over stdio (newline-delimited JSON-RPC
//! 2.0) and exposes the rtblint-core OpenRTB validator as callable tools:
//! `validate_bid_request`, `validate_bid_response`, `list_openrtb_versions`,
//! and `get_adcp_capabilities` for AdCP protocol discovery.

use std::io::{self, BufRead, Write};

use serde_json::{json, Value};

use rtblint_core::{
    validate_bid_request_for_version, validate_bid_response_against_request,
    validate_bid_response_for_version, OpenRtbVersion,
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
    let payload_schema = |payload_description: &str| {
        json!({
            "type": "object",
            "properties": {
                "payload": {
                    "type": "string",
                    "description": payload_description,
                },
                "version": {
                    "type": "string",
                    "description": format!(
                        "OpenRTB version id to validate against (default {}). One of: {}",
                        DEFAULT_VERSION.id(),
                        version_ids().join(", ")
                    ),
                },
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
                    "version": {
                        "type": "string",
                        "description": format!(
                            "OpenRTB version id to validate against (default {}). One of: {}",
                            DEFAULT_VERSION.id(),
                            version_ids().join(", ")
                        ),
                    },
                },
                "required": ["payload"],
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

    let version = match arguments.get("version").and_then(Value::as_str) {
        None => DEFAULT_VERSION,
        Some(version_id) => match OpenRtbVersion::from_id(version_id) {
            Some(version) => version,
            None => {
                return success_response(
                    id,
                    tool_result_text(
                        &format!(
                            "Unsupported OpenRTB version: {version_id}. Available versions: {}",
                            version_ids().join(", ")
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
            Some(request) => validate_bid_response_against_request(version, request, payload),
            None => validate_bid_response_for_version(version, payload),
        }
    } else {
        validate_bid_request_for_version(version, payload)
    };

    let report = json!({
        "version": version.id(),
        "payload_type": payload_type,
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

    /// Parse the JSON body out of a tool result envelope.
    fn payload_of(result: &Value) -> Value {
        let text = result["content"][0]["text"].as_str().expect("text content");
        serde_json::from_str(text).expect("tool payload is JSON")
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
