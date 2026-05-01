/// Validates an OpenRTB JSON payload (stub — full implementation coming in 0.1.0).
pub fn validate(_input: &str) -> ValidationResult {
    ValidationResult {
        valid: true,
        issues: vec![],
    }
}

/// Result of a validation run.
pub struct ValidationResult {
    pub valid: bool,
    pub issues: Vec<Issue>,
}

/// A single validation issue.
pub struct Issue {
    pub id: String,
    pub severity: String,
    pub message: String,
    pub path: Option<String>,
}
