use std::{
    env, fs,
    io::{self, BufRead, Read, Write},
    process,
};

use serde::Serialize;

use rtblint_core::{ArtfApplication, Dialect, Issue, OpenRtbVersion, Severity, ValidationResult};
use rtblint_resolve::Cache;

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
        return run_batch(&command);
    }

    let (mut result, application) = validate_one(&command, &command.input);
    apply_resolution(&command, &command.input, &mut result);
    print_result(&command, &result, application.as_ref())?;

    Ok(if result.valid { 0 } else { 1 })
}

/// Validates a single payload, returning the findings and, for an applied ARTF
/// run, what applying the mutations produced.
fn validate_one(
    command: &ValidateCommand,
    input: &str,
) -> (ValidationResult, Option<ArtfApplication>) {
    let version = command.version;
    let dialect = command.dialect;
    let request = command.request_context.as_deref();

    match (command.payload_type, request) {
        (PayloadType::Request, _) => (
            rtblint_core::validate_bid_request_with_dialect(version, dialect, input),
            None,
        ),
        (PayloadType::Response, Some(request)) => (
            rtblint_core::validate_bid_response_against_request_with_dialect(
                version, dialect, request, input,
            ),
            None,
        ),
        (PayloadType::Response, None) => (
            rtblint_core::validate_bid_response_with_dialect(version, dialect, input),
            None,
        ),
        (PayloadType::ArtfRequest, _) => {
            (rtblint_core::validate_artf_request(version, input), None)
        }
        (PayloadType::ArtfResponse, Some(request)) => {
            if command.apply {
                let outcome =
                    rtblint_core::validate_artf_mutations_applied(version, request, input);
                (outcome.result, Some(outcome.application))
            } else {
                (
                    rtblint_core::validate_artf_response_against_request(version, request, input),
                    None,
                )
            }
        }
        // parse_validate_command rejects this combination.
        (PayloadType::ArtfResponse, None) => unreachable!("artf-response requires --request"),
    }
}

fn apply_resolution(command: &ValidateCommand, input: &str, result: &mut ValidationResult) {
    let Some(cache) = command.cache.as_ref() else {
        return;
    };
    rtblint_resolve::merge_into(result, rtblint_resolve::resolve_bid_request(input, cache));
}

fn parse_validate_command(args: Vec<String>) -> Result<ValidateCommand, String> {
    let mut read_stdin = false;
    let mut file_path: Option<String> = None;
    let mut version = DEFAULT_VERSION;
    let mut output_format = OutputFormat::Human;
    let mut payload_type = PayloadType::Request;
    let mut batch = false;
    let mut request_path: Option<String> = None;
    let mut dialect = Dialect::SpecJson;
    let mut apply = false;
    let mut resolve = false;
    let mut cache_dir: Option<String> = None;
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
            "--request" => {
                let Some(value) = args.next() else {
                    return Err(format!(
                        "Missing value for --request.\n\n{}",
                        validate_usage_text()
                    ));
                };

                request_path = Some(value);
            }
            "--dialect" => {
                let Some(value) = args.next() else {
                    return Err(format!(
                        "Missing value for --dialect.\n\n{}",
                        validate_usage_text()
                    ));
                };

                dialect = Dialect::from_id(&value).ok_or_else(|| {
                    format!("Unsupported dialect: {value}\n\nUse one of: spec-json, proto-json")
                })?;
            }
            "--apply" => {
                apply = true;
            }
            "--resolve" => {
                resolve = true;
            }
            "--cache" => {
                let Some(value) = args.next() else {
                    return Err(format!(
                        "Missing value for --cache.\n\n{}",
                        validate_usage_text()
                    ));
                };
                cache_dir = Some(value);
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

    let request_context = match request_path {
        Some(path) => {
            if !matches!(
                payload_type,
                PayloadType::Response | PayloadType::ArtfResponse
            ) {
                return Err(format!(
                    "--request supplies the payload a response is cross-validated against; it \
                     requires --type response or --type artf-response.\n\n{}",
                    validate_usage_text()
                ));
            }
            Some(
                fs::read_to_string(&path)
                    .map_err(|error| format!("Failed to read {path}: {error}"))?,
            )
        }
        None => None,
    };

    if matches!(payload_type, PayloadType::ArtfResponse) && request_context.is_none() {
        return Err(format!(
            "--type artf-response needs the RTBRequest envelope it answers: pass --request \
             <rtb-request.json>. Mutations only mean anything relative to the auction they \
             target.\n\n{}",
            validate_usage_text()
        ));
    }

    if apply && !matches!(payload_type, PayloadType::ArtfResponse) {
        return Err(format!(
            "--apply writes an ARTF mutation set into the payloads it targets and revalidates \
             them; it requires --type artf-response.\n\n{}",
            validate_usage_text()
        ));
    }

    if cache_dir.is_some() && !resolve {
        return Err(format!(
            "--cache is the sellers.json / ads.txt directory for --resolve; pass --resolve as \
             well.\n\n{}",
            validate_usage_text()
        ));
    }

    if resolve && !matches!(payload_type, PayloadType::Request) {
        return Err(format!(
            "--resolve checks SupplyChain hops and the publisher's ads.txt / app-ads.txt on a \
             bid request; it does not apply to --type {}.\n\n{}",
            payload_type.id(),
            validate_usage_text()
        ));
    }

    let cache = if resolve {
        let Some(dir) = cache_dir else {
            return Err(format!(
                "--resolve needs a local cache directory: pass --cache <dir> containing \
                 sellers/<asi>/sellers.json, ads/<domain>/ads.txt, and \
                 app-ads/<bundle>/app-ads.txt.\n\n{}",
                validate_usage_text()
            ));
        };
        Some(Cache::open(dir)?)
    } else {
        None
    };

    // ARTF carries its OpenRTB payloads as protobuf messages, so their JSON is
    // protojson by construction and the dialect is not the caller's choice.
    if matches!(
        payload_type,
        PayloadType::ArtfRequest | PayloadType::ArtfResponse
    ) && dialect != Dialect::SpecJson
    {
        return Err(format!(
            "--dialect does not apply to ARTF payloads: they are protobuf JSON by \
             definition.\n\n{}",
            validate_usage_text()
        ));
    }

    Ok(ValidateCommand {
        input,
        version,
        output_format,
        payload_type,
        batch,
        request_context,
        dialect,
        apply,
        cache,
    })
}

/// Batch mode: one JSON payload per stdin line, one result per stdout line.
/// The process, spec catalogs, and allocator warm-up are paid once, so
/// per-payload cost approaches pure parse+validate time.
fn run_batch(command: &ValidateCommand) -> Result<i32, String> {
    let output_format = command.output_format;
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

        let (mut result, _) = validate_one(command, &line);
        apply_resolution(command, &line, &mut result);
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
    command: &ValidateCommand,
    result: &ValidationResult,
    application: Option<&ArtfApplication>,
) -> Result<(), String> {
    match command.output_format {
        OutputFormat::Human => {
            print_human_result(command.version, command.payload_type, result);
            if let Some(application) = application {
                print_application_summary(application);
            }
            Ok(())
        }
        OutputFormat::Json => {
            print_json_result(command.version, command.payload_type, result, application)
        }
    }
}

/// After an `--apply` run, say what was written in and what was left alone;
/// a mutation rtblint cannot apply is not the same as one it accepted.
fn print_application_summary(application: &ArtfApplication) {
    let total = application.applied.len() + application.skipped.len();
    println!(
        "Applied {} of {total} mutation(s){}.",
        application.applied.len(),
        if application.skipped.is_empty() {
            String::new()
        } else {
            format!(
                "; not applied: {}",
                application
                    .skipped
                    .iter()
                    .map(|index| format!("mutations[{index}]"))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        }
    );
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
    application: Option<&ArtfApplication>,
) -> Result<(), String> {
    let report = ValidationReport {
        version: version.id(),
        payload_type: payload_type.id(),
        valid: result.valid,
        issues: &result.issues,
        application,
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

const USAGE_LINES: &str = "  rtblint validate [--type request|response|artf-request|artf-response] [--version <openrtb-version>] [--dialect spec-json|proto-json] [--format human|json] [--request <request.json>] [--apply] [--resolve --cache <dir>] <file.json>\n  rtblint validate [...] --stdin\n  rtblint validate --batch [...]";

fn usage_text() -> String {
    format!("rtblint\n\nUsage:\n{USAGE_LINES}\n  rtblint --version\n  rtblint --help")
}

fn validate_usage_text() -> String {
    format!(
        "Usage:\n{USAGE_LINES}\n\n\
         --type selects the payload type (default: request). request and response are OpenRTB \
         payloads; artf-request is an ARTF RTBRequest envelope and artf-response is an ARTF \
         RTBResponse mutation set.\n\
         --version selects the OpenRTB spec version (default: latest tracked 2.6).\n\
         --dialect selects the JSON dialect the payload is written in: spec-json (default) types \
         flag fields as integers, proto-json follows the IAB OpenRTB protobuf schema, where 28 of \
         those fields are bool. ARTF payloads are always protobuf JSON, so the flag is rejected \
         there.\n\
         --request supplies the payload a response is cross-validated against: the originating \
         bid request for --type response (impid, mtype, adm markup, dealid, seat, and currency \
         coherence), or the RTBRequest envelope for --type artf-response (intent eligibility and \
         semantic path resolution). Required for artf-response.\n\
         --apply writes an ARTF mutation set into the payloads it targets, revalidates them, and \
         reports only the OpenRTB findings the mutations introduced; requires --type \
         artf-response.\n\
         --resolve checks each SupplyChain node against that domain's sellers.json and the \
         publisher's ads.txt or app-ads.txt. Requires --cache <dir> laid out as \
         sellers/<asi>/sellers.json, ads/<domain>/ads.txt, app-ads/<bundle>/app-ads.txt. The \
         core stays offline; this pass only reads the local cache.\n\
         --batch reads one JSON payload per stdin line and emits one result per line; exit code 0 \
         means every payload was valid."
    )
}

struct ValidateCommand {
    input: String,
    version: OpenRtbVersion,
    output_format: OutputFormat,
    payload_type: PayloadType,
    batch: bool,
    /// The payload a response is cross-validated against: a bid request for
    /// `--type response`, an ARTF RTBRequest envelope for `--type
    /// artf-response`.
    request_context: Option<String>,
    dialect: Dialect,
    /// Apply an ARTF mutation set and revalidate the result.
    apply: bool,
    /// Local sellers.json / ads.txt cache. Present only with `--resolve`.
    cache: Option<Cache>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PayloadType {
    Request,
    Response,
    ArtfRequest,
    ArtfResponse,
}

impl PayloadType {
    fn from_str(value: &str) -> Result<Self, String> {
        match value {
            "request" => Ok(Self::Request),
            "response" => Ok(Self::Response),
            "artf-request" => Ok(Self::ArtfRequest),
            "artf-response" => Ok(Self::ArtfResponse),
            _ => Err(format!(
                "Unsupported type: {value}\n\nUse one of: request, response, artf-request, \
                 artf-response"
            )),
        }
    }

    fn id(self) -> &'static str {
        match self {
            Self::Request => "request",
            Self::Response => "response",
            Self::ArtfRequest => "artf-request",
            Self::ArtfResponse => "artf-response",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Request => "bid request",
            Self::Response => "bid response",
            Self::ArtfRequest => "ARTF RTBRequest",
            Self::ArtfResponse => "ARTF mutation set",
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
    /// The payloads an `--apply` run produced, and which mutations went in.
    #[serde(skip_serializing_if = "Option::is_none")]
    application: Option<&'a ArtfApplication>,
}
