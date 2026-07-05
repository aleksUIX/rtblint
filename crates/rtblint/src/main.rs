use std::{
    env, fs,
    io::{self, Read},
    process,
};

use serde::Serialize;

use rtblint_core::{Issue, OpenRtbVersion, ValidationResult};

const DEFAULT_VERSION: OpenRtbVersion = OpenRtbVersion::V2_6_202606;

fn main() {
    match run() {
        Ok(exit_code) => process::exit(exit_code),
        Err(message) => {
            eprintln!("{message}");
            process::exit(2);
        }
    }
}

fn run() -> Result<i32, String> {
    let mut args = env::args().skip(1);
    let Some(command) = args.next() else {
        print_usage();
        return Ok(2);
    };

    if matches!(command.as_str(), "-h" | "--help") {
        print_usage();
        return Ok(0);
    }

    if matches!(command.as_str(), "-V" | "--version") {
        println!("rtblint {}", env!("CARGO_PKG_VERSION"));
        return Ok(0);
    }

    if command != "validate" {
        return Err(format!("Unknown command: {command}\n\n{}", usage_text()));
    }

    let validate_args: Vec<String> = args.collect();
    if validate_args
        .iter()
        .any(|arg| matches!(arg.as_str(), "-h" | "--help"))
    {
        print_validate_usage();
        return Ok(0);
    }

    let command = parse_validate_command(validate_args)?;
    let result = match command.payload_type {
        PayloadType::Request => {
            rtblint_core::validate_bid_request_for_version(command.version, &command.input)
        }
        PayloadType::Response => {
            rtblint_core::validate_bid_response_for_version(command.version, &command.input)
        }
    };
    print_result(
        command.version,
        command.payload_type,
        &result,
        command.output_format,
    )?;

    Ok(if result.valid { 0 } else { 1 })
}

fn parse_validate_command(args: Vec<String>) -> Result<ValidateCommand, String> {
    let mut read_stdin = false;
    let mut file_path: Option<String> = None;
    let mut version = DEFAULT_VERSION;
    let mut output_format = OutputFormat::Human;
    let mut payload_type = PayloadType::Request;
    let mut args = args.into_iter();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--stdin" | "-" => {
                if read_stdin || file_path.is_some() {
                    return Err(format!(
                        "Choose exactly one input source.\n\n{}",
                        validate_usage_text()
                    ));
                }
                read_stdin = true;
            }
            "--version" => {
                let Some(value) = args.next() else {
                    return Err(format!(
                        "Missing value for --version.\n\n{}",
                        validate_usage_text()
                    ));
                };

                version = OpenRtbVersion::from_id(&value).ok_or_else(|| {
                    format!(
                        "Unsupported version: {value}\n\nAvailable versions: {}",
                        available_versions_text()
                    )
                })?;
            }
            "--format" => {
                let Some(value) = args.next() else {
                    return Err(format!(
                        "Missing value for --format.\n\n{}",
                        validate_usage_text()
                    ));
                };

                output_format = OutputFormat::from_str(&value)?;
            }
            "--type" => {
                let Some(value) = args.next() else {
                    return Err(format!(
                        "Missing value for --type.\n\n{}",
                        validate_usage_text()
                    ));
                };

                payload_type = PayloadType::from_str(&value)?;
            }
            _ if arg.starts_with('-') => {
                return Err(format!("Unknown flag: {arg}\n\n{}", validate_usage_text()));
            }
            _ => {
                if read_stdin || file_path.is_some() {
                    return Err(format!(
                        "Choose exactly one input source.\n\n{}",
                        validate_usage_text()
                    ));
                }
                file_path = Some(arg);
            }
        }
    }
    let input = if read_stdin {
        read_from_stdin()?
    } else if let Some(path) = file_path {
        fs::read_to_string(&path).map_err(|error| format!("Failed to read {path}: {error}"))?
    } else {
        return Err(format!(
            "Missing input source.\n\n{}",
            validate_usage_text()
        ));
    };

    Ok(ValidateCommand {
        input,
        version,
        output_format,
        payload_type,
    })
}

fn read_from_stdin() -> Result<String, String> {
    let mut input = String::new();
    io::stdin()
        .read_to_string(&mut input)
        .map_err(|error| format!("Failed to read stdin: {error}"))?;

    if input.trim().is_empty() {
        return Err(String::from("Stdin was empty."));
    }

    Ok(input)
}

fn print_result(
    version: OpenRtbVersion,
    payload_type: PayloadType,
    result: &ValidationResult,
    output_format: OutputFormat,
) -> Result<(), String> {
    match output_format {
        OutputFormat::Human => {
            print_human_result(version, payload_type, result);
            Ok(())
        }
        OutputFormat::Json => print_json_result(version, payload_type, result),
    }
}

fn print_human_result(
    version: OpenRtbVersion,
    payload_type: PayloadType,
    result: &ValidationResult,
) {
    let error_count = result
        .issues
        .iter()
        .filter(|issue| issue.severity == "error")
        .count();
    let warning_count = result
        .issues
        .iter()
        .filter(|issue| issue.severity == "warning")
        .count();

    if error_count == 0 && warning_count == 0 {
        println!(
            "OK (OpenRTB {} {}): no issues found.",
            version.id(),
            payload_type.label()
        );
        return;
    }

    if error_count == 0 {
        println!(
            "OK with warnings (OpenRTB {} {}): {warning_count} warning(s).",
            version.id(),
            payload_type.label()
        );
    } else {
        println!(
            "FAILED (OpenRTB {} {}): {error_count} error(s), {warning_count} warning(s).",
            version.id(),
            payload_type.label()
        );
    }

    for issue in &result.issues {
        print_issue(issue);
    }
}

fn print_json_result(
    version: OpenRtbVersion,
    payload_type: PayloadType,
    result: &ValidationResult,
) -> Result<(), String> {
    let report = ValidationReport {
        version: version.id(),
        payload_type: payload_type.id(),
        valid: result.valid,
        issues: &result.issues,
    };

    let output = serde_json::to_string_pretty(&report)
        .map_err(|error| format!("Failed to serialize JSON output: {error}"))?;
    println!("{output}");
    Ok(())
}

fn print_issue(issue: &Issue) {
    match issue.path.as_deref() {
        Some(path) => println!(
            "- [{}] {}: {} ({})",
            issue.severity, path, issue.message, issue.id
        ),
        None => println!("- [{}] {} ({})", issue.severity, issue.message, issue.id),
    }
}

fn available_versions_text() -> String {
    OpenRtbVersion::all()
        .iter()
        .map(|version| version.id())
        .collect::<Vec<_>>()
        .join(", ")
}

fn print_usage() {
    eprintln!("{}", usage_text());
}

fn print_validate_usage() {
    eprintln!("{}", validate_usage_text());
}

fn usage_text() -> &'static str {
    "rtblint\n\nUsage:\n  rtblint validate [--type request|response] [--version <openrtb-version>] [--format human|json] <file.json>\n  rtblint validate [--type request|response] [--version <openrtb-version>] [--format human|json] --stdin\n  rtblint --version\n  rtblint --help"
}

fn validate_usage_text() -> &'static str {
    "Usage:\n  rtblint validate [--type request|response] [--version <openrtb-version>] [--format human|json] <file.json>\n  rtblint validate [--type request|response] [--version <openrtb-version>] [--format human|json] --stdin\n\n--type selects the payload type (default: request). --version selects the OpenRTB spec version (default: latest tracked 2.6)."
}

struct ValidateCommand {
    input: String,
    version: OpenRtbVersion,
    output_format: OutputFormat,
    payload_type: PayloadType,
}

#[derive(Clone, Copy)]
enum PayloadType {
    Request,
    Response,
}

impl PayloadType {
    fn from_str(value: &str) -> Result<Self, String> {
        match value {
            "request" => Ok(Self::Request),
            "response" => Ok(Self::Response),
            _ => Err(format!(
                "Unsupported type: {value}\n\nUse one of: request, response"
            )),
        }
    }

    fn id(self) -> &'static str {
        match self {
            Self::Request => "request",
            Self::Response => "response",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Request => "bid request",
            Self::Response => "bid response",
        }
    }
}

#[derive(Clone, Copy)]
enum OutputFormat {
    Human,
    Json,
}

impl OutputFormat {
    fn from_str(value: &str) -> Result<Self, String> {
        match value {
            "human" => Ok(Self::Human),
            "json" => Ok(Self::Json),
            _ => Err(format!(
                "Unsupported format: {value}\n\nUse one of: human, json"
            )),
        }
    }
}

#[derive(Serialize)]
struct ValidationReport<'a> {
    version: &'a str,
    payload_type: &'a str,
    valid: bool,
    issues: &'a [Issue],
}
