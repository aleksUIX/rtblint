//! MCP server for rtblint.
//!
//! Speaks the Model Context Protocol over stdio (newline-delimited JSON-RPC
//! 2.0) and exposes the rtblint-core OpenRTB validator as callable tools:
//! `validate_bid_request`, `validate_bid_response`, and
//! `list_openrtb_versions`.

use std::io::{self, BufRead, Write};

use serde_json::{json, Value};

use rtblint_core::{
    validate_bid_request_for_version, validate_bid_response_for_version, OpenRtbVersion,
};

const DEFAULT_VERSION: OpenRtbVersion = OpenRtbVersion::V2_6_202606;
const PROTOCOL_VERSION: &str = "2024-11-05";

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
            "description": "Validate an OpenRTB 2.x bid response JSON payload against a tracked spec version. Returns structured issues with rule ids, severities, and JSON paths.",
            "inputSchema": payload_schema("The OpenRTB bid response as a raw JSON string."),
        },
        {
            "name": "list_openrtb_versions",
            "description": "List every OpenRTB version id this build can validate against.",
            "inputSchema": { "type": "object", "properties": {} },
        },
    ])
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

    let result = if payload_type == "response" {
        validate_bid_response_for_version(version, payload)
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
