use std::{
    env, fs,
    io::{self, BufRead, Read, Write},
    process,
};

use serde::Serialize;

use rtblint_core::{Issue, OpenRtbVersion, Severity, ValidationResult};

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
    if command.batch {
        return run_batch(command.version, command.payload_type, command.output_format);
    }

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
    let mut batch = false;
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
            "--batch" => {
                batch = true;
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
    let input = if batch {
        if file_path.is_some() {
            return Err(format!(
                "--batch reads newline-delimited payloads from stdin; do not pass a file.\n\n{}",
                validate_usage_text()
            ));
        }
        String::new()
    } else if read_stdin {
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
        batch,
    })
}

/// Batch mode: one JSON payload per stdin line, one result per stdout line.
/// The process, spec catalogs, and allocator warm-up are paid once, so
/// per-payload cost approaches pure parse+validate time.
fn run_batch(
    version: OpenRtbVersion,
    payload_type: PayloadType,
    output_format: OutputFormat,
) -> Result<i32, String> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = io::BufWriter::new(stdout.lock());
    let mut all_valid = true;
    let mut count = 0usize;

    for (line_number, line) in stdin.lock().lines().enumerate() {
        let line = line.map_err(|error| format!("Failed to read stdin: {error}"))?;
        if line.trim().is_empty() {
            continue;
        }
        count += 1;

        let result = match payload_type {
            PayloadType::Request => rtblint_core::validate_bid_request_for_version(version, &line),
            PayloadType::Response => {
                rtblint_core::validate_bid_response_for_version(version, &line)
            }
        };
        all_valid &= result.valid;

        match output_format {
            OutputFormat::Json => {
                let report = BatchReport {
                    index: line_number + 1,
                    valid: result.valid,
                    issues: &result.issues,
                };
                let encoded = serde_json::to_string(&report)
                    .map_err(|error| format!("Failed to serialize JSON output: {error}"))?;
                writeln!(out, "{encoded}")
                    .map_err(|error| format!("Failed to write stdout: {error}"))?;
            }
            OutputFormat::Human => {
                let error_count = result
                    .issues
                    .iter()
                    .filter(|issue| issue.severity == Severity::Error)
                    .count();
                let warning_count = result
                    .issues
                    .iter()
                    .filter(|issue| issue.severity == Severity::Warning)
                    .count();
                let verdict = if result.valid { "OK" } else { "FAILED" };
                writeln!(
                    out,
                    "#{} {verdict}: {error_count} error(s), {warning_count} warning(s)",
                    line_number + 1
                )
                .map_err(|error| format!("Failed to write stdout: {error}"))?;
            }
        }
    }

    out.flush()
        .map_err(|error| format!("Failed to flush stdout: {error}"))?;

    if count == 0 {
        return Err(String::from("Stdin was empty."));
    }

    Ok(if all_valid { 0 } else { 1 })
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
        .filter(|issue| issue.severity == Severity::Error)
        .count();
    let warning_count = result
        .issues
        .iter()
        .filter(|issue| issue.severity == Severity::Warning)
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
    let section_suffix = issue
        .section
        .as_deref()
        .map(|section| format!(" · spec {section}"))
        .unwrap_or_default();

    match issue.path.as_deref() {
        Some(path) => println!(
            "- [{}] {}: {} ({}){}",
            issue.severity, path, issue.message, issue.id, section_suffix
        ),
        None => println!(
            "- [{}] {} ({}){}",
            issue.severity, issue.message, issue.id, section_suffix
        ),
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
    "rtblint\n\nUsage:\n  rtblint validate [--type request|response] [--version <openrtb-version>] [--format human|json] <file.json>\n  rtblint validate [--type request|response] [--version <openrtb-version>] [--format human|json] --stdin\n  rtblint validate --batch [--type request|response] [--version <openrtb-version>] [--format human|json]\n  rtblint --version\n  rtblint --help"
}

fn validate_usage_text() -> &'static str {
    "Usage:\n  rtblint validate [--type request|response] [--version <openrtb-version>] [--format human|json] <file.json>\n  rtblint validate [--type request|response] [--version <openrtb-version>] [--format human|json] --stdin\n  rtblint validate --batch [--type request|response] [--version <openrtb-version>] [--format human|json]\n\n--type selects the payload type (default: request). --version selects the OpenRTB spec version (default: latest tracked 2.6). --batch reads one JSON payload per stdin line and emits one result per line; exit code 0 means every payload was valid."
}

struct ValidateCommand {
    input: String,
    version: OpenRtbVersion,
    output_format: OutputFormat,
    payload_type: PayloadType,
    batch: bool,
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
struct BatchReport<'a> {
    index: usize,
    valid: bool,
    issues: &'a [Issue],
}

#[derive(Serialize)]
struct ValidationReport<'a> {
    version: &'a str,
    payload_type: &'a str,
    valid: bool,
    issues: &'a [Issue],
}
