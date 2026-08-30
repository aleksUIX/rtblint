//! Payload-versus-declaration checks on privacy signals.
//!
//! Findings state facts about the document ("regs.coppa is 1 and device.ifa
//! is a non-zero advertising identifier"). They do not state legal
//! conclusions. Consent-string forensics (GVL, CMP lists, entropy) stay out.

use serde_json::{Map, Value};

use crate::privacy::{decode_tcf_core, gpp_section};
use crate::{Issue, Severity};

const SECTION_REGS: &str = "3.2.3";
const SECTION_DEVICE: &str = "3.2.18";
const SECTION_USER: &str = "3.2.20";
const SECTION_TCF: &str = "IAB TCF v2.2";
const SECTION_USP: &str = "US Privacy String v1";
const SECTION_DSA: &str = "DSA Transparency";

/// Cross-field privacy checks on a 2.x BidRequest or an AdCOM Context.
pub(crate) fn validate_privacy_signals(
    root: &Map<String, Value>,
    instance_path: &str,
    issues: &mut Vec<Issue>,
) {
    let regs = root.get("regs").and_then(Value::as_object);
    let device = root.get("device").and_then(Value::as_object);
    let user = root.get("user").and_then(Value::as_object);
    let site = root.get("site").and_then(Value::as_object);

    let coppa = regs.is_some_and(|regs| flag_is_set(regs.get("coppa")));
    let gdpr = regs.is_some_and(|regs| flag_is_set(regs.get("gdpr")));
    let lmt = device.is_some_and(|device| flag_is_set(device.get("lmt")));
    let dnt = device.is_some_and(|device| flag_is_set(device.get("dnt")));

    if coppa {
        if let Some((path, _value)) = device_advertising_id(device, instance_path) {
            push(
                issues,
                "openrtb.privacy.coppa_identifier",
                SECTION_REGS,
                path,
                "regs.coppa is 1 and this device field carries an advertising identifier; \
                 the flag and the identifier contradict each other.",
            );
        }
        if has_eids(user) {
            push(
                issues,
                "openrtb.privacy.coppa_eids",
                SECTION_REGS,
                join(instance_path, "user.eids"),
                "regs.coppa is 1 and user.eids is populated; the flag and the extended \
                 identifiers contradict each other.",
            );
        }
        if let Some(path) = precise_geo_path(device, instance_path) {
            push(
                issues,
                "openrtb.privacy.coppa_geo",
                SECTION_REGS,
                path,
                "regs.coppa is 1 and device.geo carries lat/lon with more than two decimal \
                 places (finer than city-level).",
            );
        }
        if has_segments(user) {
            push(
                issues,
                "openrtb.privacy.coppa_segments",
                SECTION_REGS,
                join(instance_path, "user.data"),
                "regs.coppa is 1 and user.data carries audience segments; the flag and \
                 the segments contradict each other.",
            );
        }
    }

    if lmt {
        if let Some((path, _)) = real_ifa(device, instance_path) {
            push(
                issues,
                "openrtb.privacy.lmt_ifa",
                SECTION_DEVICE,
                path,
                "device.lmt is 1 and device.ifa is a non-zero advertising identifier; \
                 limit-ad-tracking and a live IFA contradict each other.",
            );
        }
    }

    if dnt && has_user_identifiers(user, device) {
        let path = if user.is_some_and(|user| non_empty_str(user.get("id"))) {
            join(instance_path, "user.id")
        } else if user.is_some_and(|user| non_empty_str(user.get("buyeruid"))) {
            join(instance_path, "user.buyeruid")
        } else if has_eids(user) {
            join(instance_path, "user.eids")
        } else {
            join(instance_path, "device.ifa")
        };
        push(
            issues,
            "openrtb.privacy.dnt_identifiers",
            SECTION_DEVICE,
            path,
            "device.dnt is 1 and the request still carries user.id, user.buyeruid, \
             user.eids, or a non-zero device.ifa.",
        );
    }

    if gdpr && !has_consent_string(user, regs) && has_any_identifier(user, device) {
        push(
            issues,
            "openrtb.privacy.gdpr_without_consent",
            SECTION_REGS,
            join(instance_path, "regs.gdpr"),
            "regs.gdpr is 1, no user.consent or GPP TCF section is present, and the \
             request still carries identifiers or user.data segments.",
        );
    }

    if let Some(core) = tcf_core_from(user, regs) {
        if !core.purpose1_consent
            && !core.purpose_one_treatment
            && has_storage_identifiers(user, device)
        {
            let path = if user.is_some_and(|user| non_empty_str(user.get("consent"))) {
                join(instance_path, "user.consent")
            } else {
                join(instance_path, "regs.gpp")
            };
            push(
                issues,
                "openrtb.privacy.tcf_purpose1_identifiers",
                SECTION_TCF,
                path,
                "the TCF 2 core string does not set Purpose 1 consent (and \
                 PurposeOneTreatment is unset) while the request still carries \
                 device-storage identifiers.",
            );
        }
    }

    if us_privacy_opt_out_sale(regs) && has_hashed_eids(user) {
        push(
            issues,
            "openrtb.privacy.us_privacy_sale_eids",
            SECTION_USP,
            join(instance_path, "user.eids"),
            "the US Privacy string (regs.us_privacy or GPP section 6) has Opt-Out Sale \
             Y, and user.eids includes a UID with atype 3 (hashed identifier).",
        );
    }

    scan_pii_email(user, site, instance_path, issues);
    validate_request_dsa(regs, instance_path, issues);
}

/// `bid.ext.dsa` presence: the IAB DSA Transparency fields `behalf` and `paid`.
pub(crate) fn validate_bid_dsa(
    bid: &Map<String, Value>,
    instance_path: &str,
    issues: &mut Vec<Issue>,
) {
    let Some(dsa) = bid
        .get("ext")
        .and_then(Value::as_object)
        .and_then(|ext| ext.get("dsa"))
        .and_then(Value::as_object)
    else {
        return;
    };
    for field in ["behalf", "paid"] {
        if !non_empty_str(dsa.get(field)) {
            push(
                issues,
                "openrtb.bid.dsa.field_required",
                SECTION_DSA,
                join(instance_path, &format!("ext.dsa.{field}")),
                format!(
                    "bid.ext.dsa is present but {field} is missing or empty; the DSA \
                     Transparency extension names {field} as a required string."
                ),
            );
        }
    }
}

fn validate_request_dsa(
    regs: Option<&Map<String, Value>>,
    instance_path: &str,
    issues: &mut Vec<Issue>,
) {
    let Some(dsa) = regs
        .and_then(|regs| regs.get("ext"))
        .and_then(Value::as_object)
        .and_then(|ext| ext.get("dsa"))
        .and_then(Value::as_object)
    else {
        return;
    };

    if let Some(required) = dsa.get("dsarequired") {
        let allowed = match required {
            Value::Number(number) => number
                .as_i64()
                .or_else(|| number.as_u64().and_then(|n| i64::try_from(n).ok()))
                .is_some_and(|n| (0..=3).contains(&n)),
            _ => false,
        };
        if !allowed {
            push(
                issues,
                "openrtb.privacy.dsa_dsarequired_invalid",
                SECTION_DSA,
                join(instance_path, "regs.ext.dsa.dsarequired"),
                "regs.ext.dsa.dsarequired is not 0, 1, 2, or 3 (the DSA Transparency \
                 extension's documented values).",
            );
        }
    }

    if let Some(entries) = dsa.get("transparency").and_then(Value::as_array) {
        for (index, entry) in entries.iter().enumerate() {
            let Some(object) = entry.as_object() else {
                continue;
            };
            if !non_empty_str(object.get("domain")) {
                push(
                    issues,
                    "openrtb.privacy.dsa_transparency_domain",
                    SECTION_DSA,
                    join(
                        instance_path,
                        &format!("regs.ext.dsa.transparency[{index}].domain"),
                    ),
                    "a DSA transparency entry has no domain.",
                );
            }
        }
    }
}

fn scan_pii_email(
    user: Option<&Map<String, Value>>,
    site: Option<&Map<String, Value>>,
    instance_path: &str,
    issues: &mut Vec<Issue>,
) {
    if let Some(user) = user {
        for field in ["id", "buyeruid"] {
            if let Some(value) = user.get(field).and_then(Value::as_str) {
                if looks_like_email(value) {
                    push(
                        issues,
                        "openrtb.privacy.pii_email",
                        SECTION_USER,
                        join(instance_path, &format!("user.{field}")),
                        format!(
                            "user.{field} looks like an email address rather than an opaque \
                             identifier."
                        ),
                    );
                }
            }
        }
    }
    if let Some(page) = site
        .and_then(|site| site.get("page"))
        .and_then(Value::as_str)
    {
        if page_carries_email(page) {
            push(
                issues,
                "openrtb.privacy.pii_email",
                SECTION_USER,
                join(instance_path, "site.page"),
                "site.page carries an email address in the path or query string.",
            );
        }
    }
}

fn tcf_core_from(
    user: Option<&Map<String, Value>>,
    regs: Option<&Map<String, Value>>,
) -> Option<crate::privacy::TcfCore> {
    if let Some(consent) = user
        .and_then(|user| user.get("consent"))
        .and_then(Value::as_str)
    {
        if let Some(core) = decode_tcf_core(consent) {
            return Some(core);
        }
    }
    let gpp = regs
        .and_then(|regs| regs.get("gpp"))
        .and_then(Value::as_str)?;
    for sid in [2_i64, 5] {
        if let Some(section) = gpp_section(gpp, sid) {
            if let Some(core) = decode_tcf_core(section) {
                return Some(core);
            }
        }
    }
    None
}

fn has_consent_string(
    user: Option<&Map<String, Value>>,
    regs: Option<&Map<String, Value>>,
) -> bool {
    if user.is_some_and(|user| non_empty_str(user.get("consent"))) {
        return true;
    }
    let Some(gpp) = regs
        .and_then(|regs| regs.get("gpp"))
        .and_then(Value::as_str)
    else {
        return false;
    };
    gpp_section(gpp, 2).is_some() || gpp_section(gpp, 5).is_some()
}

fn us_privacy_opt_out_sale(regs: Option<&Map<String, Value>>) -> bool {
    let Some(regs) = regs else {
        return false;
    };
    if let Some(usp) = regs.get("us_privacy").and_then(Value::as_str) {
        if usp_opt_out_sale(usp) {
            return true;
        }
    }
    if let Some(gpp) = regs.get("gpp").and_then(Value::as_str) {
        if let Some(section) = gpp_section(gpp, 6) {
            return usp_opt_out_sale(section);
        }
    }
    false
}

fn usp_opt_out_sale(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 4 && bytes[0] == b'1' && bytes[2] == b'Y'
}

fn device_advertising_id<'a>(
    device: Option<&'a Map<String, Value>>,
    instance_path: &str,
) -> Option<(String, &'a str)> {
    let device = device?;
    if let Some(found) = real_ifa(Some(device), instance_path) {
        return Some(found);
    }
    for field in [
        "didsha1", "didmd5", "dpidsha1", "dpidmd5", "macsha1", "macmd5",
    ] {
        if let Some(value) = device.get(field).and_then(Value::as_str) {
            if ifa_is_real(value) {
                return Some((join(instance_path, &format!("device.{field}")), value));
            }
        }
    }
    None
}

fn real_ifa<'a>(
    device: Option<&'a Map<String, Value>>,
    instance_path: &str,
) -> Option<(String, &'a str)> {
    let ifa = device?.get("ifa").and_then(Value::as_str)?;
    if ifa_is_real(ifa) {
        Some((join(instance_path, "device.ifa"), ifa))
    } else {
        None
    }
}

fn ifa_is_real(ifa: &str) -> bool {
    let compact: String = ifa
        .chars()
        .filter(|ch| *ch != '-' && !ch.is_whitespace())
        .collect();
    if compact.is_empty() {
        return false;
    }
    if compact.chars().all(|ch| ch == '0') {
        return false;
    }
    if compact.chars().all(|ch| ch == 'f' || ch == 'F') {
        return false;
    }
    true
}

fn has_eids(user: Option<&Map<String, Value>>) -> bool {
    user.and_then(|user| user.get("eids"))
        .and_then(Value::as_array)
        .is_some_and(|eids| {
            eids.iter().any(|eid| {
                eid.get("uids")
                    .and_then(Value::as_array)
                    .is_some_and(|uids| uids.iter().any(|uid| non_empty_str(uid.get("id"))))
            })
        })
}

fn has_hashed_eids(user: Option<&Map<String, Value>>) -> bool {
    user.and_then(|user| user.get("eids"))
        .and_then(Value::as_array)
        .is_some_and(|eids| {
            eids.iter().any(|eid| {
                eid.get("uids")
                    .and_then(Value::as_array)
                    .is_some_and(|uids| {
                        uids.iter().any(|uid| {
                            non_empty_str(uid.get("id"))
                                && uid.get("atype").and_then(json_int) == Some(3)
                        })
                    })
            })
        })
}

fn has_segments(user: Option<&Map<String, Value>>) -> bool {
    user.and_then(|user| user.get("data"))
        .and_then(Value::as_array)
        .is_some_and(|data| {
            data.iter().any(|node| {
                node.get("segment")
                    .and_then(Value::as_array)
                    .is_some_and(|segments| !segments.is_empty())
            })
        })
}

fn has_user_identifiers(
    user: Option<&Map<String, Value>>,
    device: Option<&Map<String, Value>>,
) -> bool {
    user.is_some_and(|user| non_empty_str(user.get("id")))
        || user.is_some_and(|user| non_empty_str(user.get("buyeruid")))
        || has_eids(user)
        || real_ifa(device, "").is_some()
}

fn has_any_identifier(
    user: Option<&Map<String, Value>>,
    device: Option<&Map<String, Value>>,
) -> bool {
    has_user_identifiers(user, device)
        || has_segments(user)
        || device_advertising_id(device, "").is_some()
}

fn has_storage_identifiers(
    user: Option<&Map<String, Value>>,
    device: Option<&Map<String, Value>>,
) -> bool {
    real_ifa(device, "").is_some()
        || device_advertising_id(device, "").is_some()
        || user.is_some_and(|user| non_empty_str(user.get("id")))
        || user.is_some_and(|user| non_empty_str(user.get("buyeruid")))
        || has_eids(user)
}

fn precise_geo_path(device: Option<&Map<String, Value>>, instance_path: &str) -> Option<String> {
    let geo = device?.get("geo").and_then(Value::as_object)?;
    for field in ["lat", "lon"] {
        if let Some(value) = geo.get(field) {
            if more_than_two_decimals(value) {
                return Some(join(instance_path, &format!("device.geo.{field}")));
            }
        }
    }
    None
}

fn more_than_two_decimals(value: &Value) -> bool {
    let Value::Number(number) = value else {
        return false;
    };
    let rendered = number.to_string();
    let Some((_, frac)) = rendered.split_once('.') else {
        return false;
    };
    let digits = frac.trim_end_matches('0');
    digits.len() > 2
}

fn looks_like_email(value: &str) -> bool {
    let value = value.trim();
    if value.len() < 6 || value.len() > 254 || value.contains(' ') {
        return false;
    }
    let Some((local, domain)) = value.split_once('@') else {
        return false;
    };
    if local.is_empty() || domain.len() < 3 || !domain.contains('.') {
        return false;
    }
    local
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '%' | '+' | '-'))
        && domain
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-'))
        && domain
            .rsplit('.')
            .next()
            .is_some_and(|tld| tld.len() >= 2 && tld.chars().all(|ch| ch.is_ascii_alphabetic()))
}

fn page_carries_email(page: &str) -> bool {
    if looks_like_email(page) {
        return true;
    }
    for chunk in page.split(['/', '?', '&', '=', '#']) {
        let decoded = chunk.replace("%40", "@");
        if looks_like_email(&decoded) {
            return true;
        }
    }
    false
}

fn flag_is_set(value: Option<&Value>) -> bool {
    match value {
        Some(Value::Bool(true)) => true,
        Some(Value::Number(number)) => {
            number.as_i64() == Some(1)
                || number.as_u64().and_then(|n| i64::try_from(n).ok()) == Some(1)
        }
        _ => false,
    }
}

fn non_empty_str(value: Option<&Value>) -> bool {
    value
        .and_then(Value::as_str)
        .is_some_and(|text| !text.is_empty())
}

fn json_int(value: &Value) -> Option<i64> {
    value
        .as_i64()
        .or_else(|| value.as_u64().and_then(|number| i64::try_from(number).ok()))
}

fn join(base: &str, segment: &str) -> String {
    if base.is_empty() {
        String::from(segment)
    } else {
        format!("{base}.{segment}")
    }
}

fn push(
    issues: &mut Vec<Issue>,
    id: &'static str,
    section: &'static str,
    path: String,
    message: impl Into<String>,
) {
    issues.push(Issue {
        id: String::from(id),
        severity: Severity::Warning,
        message: message.into(),
        path: Some(path),
        section: Some(String::from(section)),
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn root(value: Value) -> Map<String, Value> {
        value.as_object().cloned().expect("object")
    }

    #[test]
    fn coppa_and_ifa_contradict() {
        let object = root(json!({
            "regs": { "coppa": 1 },
            "device": { "ifa": "aebe4057-cebe-4301-8cc6-5e6e41c0bf47" }
        }));
        let mut issues = Vec::new();
        validate_privacy_signals(&object, "", &mut issues);
        assert!(issues
            .iter()
            .any(|issue| issue.id == "openrtb.privacy.coppa_identifier"));
    }

    #[test]
    fn zero_ifa_is_not_an_identifier() {
        let object = root(json!({
            "regs": { "coppa": 1 },
            "device": { "ifa": "00000000-0000-0000-0000-000000000000" }
        }));
        let mut issues = Vec::new();
        validate_privacy_signals(&object, "", &mut issues);
        assert!(!issues
            .iter()
            .any(|issue| issue.id == "openrtb.privacy.coppa_identifier"));
    }

    #[test]
    fn proto_bool_coppa_counts() {
        let object = root(json!({
            "regs": { "coppa": true },
            "device": { "ifa": "abc" }
        }));
        let mut issues = Vec::new();
        validate_privacy_signals(&object, "", &mut issues);
        assert!(issues
            .iter()
            .any(|issue| issue.id == "openrtb.privacy.coppa_identifier"));
    }

    #[test]
    fn email_in_user_id() {
        let object = root(json!({ "user": { "id": "buyer@example.com" } }));
        let mut issues = Vec::new();
        validate_privacy_signals(&object, "", &mut issues);
        assert!(issues
            .iter()
            .any(|issue| issue.id == "openrtb.privacy.pii_email"));
    }

    #[test]
    fn us_privacy_sale_opt_out_and_hashed_eid() {
        let object = root(json!({
            "regs": { "us_privacy": "1YYY" },
            "user": { "eids": [{ "source": "liveramp.com", "uids": [{ "id": "abc", "atype": 3 }] }] }
        }));
        let mut issues = Vec::new();
        validate_privacy_signals(&object, "", &mut issues);
        assert!(issues
            .iter()
            .any(|issue| issue.id == "openrtb.privacy.us_privacy_sale_eids"));
    }

    #[test]
    fn dsa_transparency_needs_domain() {
        let object = root(json!({
            "regs": { "ext": { "dsa": { "dsarequired": 2, "transparency": [{ "dsaparams": [1] }] } } }
        }));
        let mut issues = Vec::new();
        validate_privacy_signals(&object, "", &mut issues);
        assert!(issues
            .iter()
            .any(|issue| issue.id == "openrtb.privacy.dsa_transparency_domain"));
    }

    #[test]
    fn bid_dsa_needs_behalf() {
        let bid = root(json!({ "ext": { "dsa": { "paid": "Advertiser" } } }));
        let mut issues = Vec::new();
        validate_bid_dsa(&bid, "seatbid[0].bid[0]", &mut issues);
        assert!(issues
            .iter()
            .any(|issue| issue.id == "openrtb.bid.dsa.field_required"
                && issue.path.as_deref() == Some("seatbid[0].bid[0].ext.dsa.behalf")));
    }

    #[test]
    fn tcf_purpose1_off_with_ifa() {
        let consent = crate::privacy::encode_tcf_core(false, false);
        let object = root(json!({
            "user": { "consent": consent },
            "device": { "ifa": "aebe4057-cebe-4301-8cc6-5e6e41c0bf47" }
        }));
        let mut issues = Vec::new();
        validate_privacy_signals(&object, "", &mut issues);
        assert!(issues
            .iter()
            .any(|issue| issue.id == "openrtb.privacy.tcf_purpose1_identifiers"));
    }

    #[test]
    fn tcf_purpose1_on_with_ifa_is_quiet() {
        let consent = crate::privacy::encode_tcf_core(true, false);
        let object = root(json!({
            "user": { "consent": consent },
            "device": { "ifa": "aebe4057-cebe-4301-8cc6-5e6e41c0bf47" }
        }));
        let mut issues = Vec::new();
        validate_privacy_signals(&object, "", &mut issues);
        assert!(!issues
            .iter()
            .any(|issue| issue.id == "openrtb.privacy.tcf_purpose1_identifiers"));
    }
}
