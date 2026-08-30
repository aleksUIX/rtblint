//! GPP and TCF strings that OpenRTB carries as opaque `regs.gpp` / `user.consent`.
//!
//! Presence of `gpp` vs `gpp_sid` is checked in `validator`. This module looks
//! at the bytes: GPP header type/version/section IDs (Fibonacci range),
//! section count vs `gpp_sid`, and the TCF 2 core-string shape.

use serde_json::{Map, Value};

use crate::{Issue, Severity};

const SECTION_GPP: &str = "GPP Consent String Specification";
const SECTION_TCF: &str = "IAB TCF v2.2";

/// IAB GPP section IDs that are not the header (3). Sid 3 is the header
/// itself and must not appear in `gpp_sid`.
const KNOWN_GPP_SIDS: &[i64] = &[
    1, 2, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27,
    28, 29, 30, 31, 32,
];

const TCF_MIN_LEN: usize = 19;

pub(crate) fn validate_gpp(
    gpp: &str,
    gpp_sid: Option<&[Value]>,
    instance_path: &str,
    issues: &mut Vec<Issue>,
) {
    if gpp.is_empty() {
        return;
    }

    let Some((header, sections)) = split_gpp(gpp) else {
        issues.push(issue(
            "openrtb.regs.gpp_malformed",
            Severity::Warning,
            String::from(
                "GPP string has no '~' separator; a GPP string is a header followed by one \
                 section per declared id.",
            ),
            join_path(instance_path, "gpp"),
        ));
        return;
    };

    if !header_alphabet_ok(header) {
        issues.push(issue(
            "openrtb.regs.gpp_malformed",
            Severity::Warning,
            String::from(
                "GPP header contains characters outside the IAB 6-bit alphabet (A-Z, a-z, 0-9, \
                 +, /, -, _).",
            ),
            join_path(instance_path, "gpp"),
        ));
        return;
    }

    let decoded = decode_header(header);
    match decoded {
        None => issues.push(issue(
            "openrtb.regs.gpp_malformed",
            Severity::Warning,
            String::from(
                "GPP header does not decode as type 3 version 1 with a Fibonacci section range; \
                 a well-formed header starts with \"DB\".",
            ),
            join_path(instance_path, "gpp"),
        )),
        Some(ids) => {
            if ids.len() != sections.len() {
                issues.push(issue(
                    "openrtb.regs.gpp_section_mismatch",
                    Severity::Warning,
                    format!(
                        "GPP header declares {} section(s) but the string carries {} payload(s) \
                         after '~'.",
                        ids.len(),
                        sections.len()
                    ),
                    join_path(instance_path, "gpp"),
                ));
            }
            if let Some(sids) = gpp_sid {
                let declared: Vec<i64> = sids.iter().filter_map(json_int).collect();
                if !declared.is_empty() && declared != ids {
                    issues.push(issue(
                        "openrtb.regs.gpp_section_mismatch",
                        Severity::Warning,
                        format!(
                            "gpp_sid {:?} does not match the section ids encoded in the GPP \
                             header {:?}.",
                            declared, ids
                        ),
                        join_path(instance_path, "gpp_sid"),
                    ));
                }
                for (index, sid) in declared.iter().enumerate() {
                    if *sid == 3 {
                        issues.push(issue(
                            "openrtb.regs.gpp_malformed",
                            Severity::Warning,
                            String::from(
                                "gpp_sid value 3 is the GPP header itself, not a discrete \
                                 section; omit it from the array.",
                            ),
                            format!("{}.gpp_sid[{index}]", instance_path),
                        ));
                    } else if !KNOWN_GPP_SIDS.contains(sid) {
                        issues.push(issue(
                            "openrtb.regs.gpp_malformed",
                            Severity::Warning,
                            format!(
                                "gpp_sid value {sid} is not a documented GPP section id (see IAB \
                                 GPP Section Information)."
                            ),
                            format!("{}.gpp_sid[{index}]", instance_path),
                        ));
                    }
                }
                for (index, sid) in declared.iter().enumerate() {
                    let Some(payload) = sections.get(index) else {
                        continue;
                    };
                    match *sid {
                        2 | 5 => {
                            if let Some(message) = tcf_shape_error(payload) {
                                issues.push(issue(
                                    "openrtb.regs.tcf_malformed",
                                    Severity::Warning,
                                    format!(
                                        "GPP section {sid} should be a TCF 2 string; {message}"
                                    ),
                                    join_path(instance_path, "gpp"),
                                ));
                            }
                        }
                        6 if !payload.is_empty() && !us_privacy_shape(payload) => {
                            issues.push(issue(
                                "openrtb.regs.us_privacy_malformed",
                                Severity::Warning,
                                format!(
                                    "GPP section 6 should be a US Privacy string (1Y/N/-); got \
                                     \"{payload}\"."
                                ),
                                join_path(instance_path, "gpp"),
                            ));
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}

pub(crate) fn validate_user_consent(
    object: &Map<String, Value>,
    instance_path: &str,
    issues: &mut Vec<Issue>,
) {
    let Some(consent) = object.get("consent").and_then(Value::as_str) else {
        return;
    };
    if consent.is_empty() {
        return;
    }
    if let Some(message) = tcf_shape_error(consent) {
        issues.push(issue(
            "openrtb.regs.tcf_malformed",
            Severity::Warning,
            format!("user.consent {message}"),
            join_path(instance_path, "consent"),
        ));
    }
}

/// TCF 2 core bits that contradiction checks need. `None` if the string is
/// not a readable TCF 2 core (shape errors stay on `validate_user_consent`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TcfCore {
    pub purpose1_consent: bool,
    pub purpose_one_treatment: bool,
}

pub(crate) fn decode_tcf_core(value: &str) -> Option<TcfCore> {
    let core = value.split('.').next().unwrap_or(value);
    let mut reader = BitReader::from_alphabet(core)?;
    let version = reader.read_bits(6)?;
    if version != 2 {
        return None;
    }
    let _created = reader.read_bits(36)?;
    let _last_updated = reader.read_bits(36)?;
    let _cmp_id = reader.read_bits(12)?;
    let _cmp_version = reader.read_bits(12)?;
    let _consent_screen = reader.read_bits(6)?;
    let _consent_language = reader.read_bits(12)?;
    let _vendor_list_version = reader.read_bits(12)?;
    let _policy_version = reader.read_bits(6)?;
    let _is_service_specific = reader.read_bits(1)?;
    let _use_non_standard = reader.read_bits(1)?;
    let _special_features = reader.read_bits(12)?;
    let purposes_consent = reader.read_bits(24)?;
    let _purposes_li = reader.read_bits(24)?;
    let purpose_one_treatment = reader.read_bits(1)?;
    let _publisher_cc = reader.read_bits(12)?;
    Some(TcfCore {
        purpose1_consent: (purposes_consent >> 23) & 1 == 1,
        purpose_one_treatment: purpose_one_treatment == 1,
    })
}

#[cfg(test)]
pub(crate) fn encode_tcf_core(purpose1_consent: bool, purpose_one_treatment: bool) -> String {
    let mut writer = BitWriter::default();
    writer.write_bits(2, 6);
    writer.write_bits(0, 36);
    writer.write_bits(0, 36);
    writer.write_bits(1, 12);
    writer.write_bits(1, 12);
    writer.write_bits(1, 6);
    writer.write_bits((b'E' - b'A') as i64, 6);
    writer.write_bits((b'N' - b'A') as i64, 6);
    writer.write_bits(1, 12);
    writer.write_bits(2, 6);
    writer.write_bits(1, 1);
    writer.write_bits(0, 1);
    writer.write_bits(0, 12);
    let purposes = if purpose1_consent { 1_i64 << 23 } else { 0 };
    writer.write_bits(purposes, 24);
    writer.write_bits(0, 24);
    writer.write_bits(i64::from(purpose_one_treatment), 1);
    writer.write_bits(0, 12);
    writer.finish()
}

/// Payload of GPP section `want`, if the header decodes and that id is present.
pub(crate) fn gpp_section(gpp: &str, want: i64) -> Option<&str> {
    let (header, sections) = split_gpp(gpp)?;
    let ids = decode_header(header)?;
    let index = ids.iter().position(|id| *id == want)?;
    sections.get(index).copied()
}

fn split_gpp(gpp: &str) -> Option<(&str, Vec<&str>)> {
    let mut parts = gpp.split('~');
    let header = parts.next()?;
    if header.is_empty() {
        return None;
    }
    let sections: Vec<&str> = parts.collect();
    if sections.is_empty() {
        return None;
    }
    Some((header, sections))
}

fn header_alphabet_ok(header: &str) -> bool {
    header
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '+' | '/' | '-' | '_'))
}

/// Decodes a GPP header into ordered section IDs. `None` if type/version
/// are wrong or the range cannot be read.
fn decode_header(header: &str) -> Option<Vec<i64>> {
    let mut reader = BitReader::from_alphabet(header)?;
    let header_type = reader.read_bits(6)?;
    let version = reader.read_bits(6)?;
    if header_type != 3 || version != 1 {
        return None;
    }
    let count = reader.read_bits(12)? as usize;
    if count == 0 || count > 64 {
        return None;
    }
    let mut ids = Vec::with_capacity(count);
    let mut last_id: i64 = 0;
    for _ in 0..count {
        let is_group = reader.read_bits(1)? == 1;
        let delta = reader.read_fibonacci()?;
        last_id = last_id.checked_add(delta)?;
        if is_group {
            let offset = reader.read_fibonacci()?;
            let end = last_id.checked_add(offset)?;
            for id in last_id..=end {
                ids.push(id);
            }
            last_id = end;
        } else {
            ids.push(last_id);
        }
    }
    Some(ids)
}

fn tcf_shape_error(value: &str) -> Option<String> {
    let core = value.split('.').next().unwrap_or(value);
    if core.is_empty() {
        return Some(String::from("it is empty."));
    }
    if core.starts_with('B') {
        return Some(String::from(
            "it starts with \"B\" (TCF 1.1). TCF 2.x core strings start with \"C\".",
        ));
    }
    if !core.starts_with('C') {
        return Some(format!(
            "it starts with \"{}\" rather than \"C\" (TCF 2 core version bits).",
            core.chars().next().unwrap_or('?')
        ));
    }
    if core.len() < TCF_MIN_LEN {
        return Some(format!(
            "the core segment is {} characters; a TCF 2 core string is much longer.",
            core.len()
        ));
    }
    if !value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'))
    {
        return Some(String::from(
            "it contains characters outside the TCF alphabet (A-Z, a-z, 0-9, -, _, .).",
        ));
    }
    None
}

fn us_privacy_shape(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 4
        && bytes[0] == b'1'
        && bytes[1..]
            .iter()
            .all(|byte| matches!(byte, b'Y' | b'N' | b'-'))
}

fn json_int(value: &Value) -> Option<i64> {
    value
        .as_i64()
        .or_else(|| value.as_u64().and_then(|number| i64::try_from(number).ok()))
}

fn join_path(base: &str, segment: &str) -> String {
    if base.is_empty() {
        String::from(segment)
    } else {
        format!("{base}.{segment}")
    }
}

fn issue(id: &'static str, severity: Severity, message: String, path: String) -> Issue {
    Issue {
        id: String::from(id),
        severity,
        message,
        path: Some(path),
        section: Some(String::from(if id.contains("tcf") {
            SECTION_TCF
        } else if id.contains("us_privacy") {
            "US Privacy String v1"
        } else {
            SECTION_GPP
        })),
    }
}

struct BitReader {
    bits: Vec<bool>,
    pos: usize,
}

impl BitReader {
    fn from_alphabet(encoded: &str) -> Option<Self> {
        let mut bits = Vec::with_capacity(encoded.len() * 6);
        for ch in encoded.chars() {
            let value = six_bit_value(ch)?;
            for shift in (0..6).rev() {
                bits.push((value >> shift) & 1 == 1);
            }
        }
        Some(Self { bits, pos: 0 })
    }

    fn read_bits(&mut self, n: usize) -> Option<i64> {
        if n == 0 || self.pos + n > self.bits.len() {
            return None;
        }
        let mut value: i64 = 0;
        for _ in 0..n {
            value = (value << 1) | i64::from(self.bits[self.pos]);
            self.pos += 1;
        }
        Some(value)
    }

    fn read_fibonacci(&mut self) -> Option<i64> {
        // Weights are F(2), F(3), F(4), ... = 1, 2, 3, 5, 8. A terminating
        // 1 is not part of the value (11 encodes 1, not 2).
        let mut prev_bit = false;
        let mut weight: i64 = 1;
        let mut next_weight: i64 = 2;
        let mut total: i64 = 0;
        loop {
            if self.pos >= self.bits.len() {
                return None;
            }
            let bit = self.bits[self.pos];
            self.pos += 1;
            if prev_bit && bit {
                return Some(total);
            }
            if bit {
                total = total.checked_add(weight)?;
            }
            prev_bit = bit;
            let following = weight.checked_add(next_weight)?;
            weight = next_weight;
            next_weight = following;
            if weight > 1_000_000 {
                return None;
            }
        }
    }
}

fn six_bit_value(ch: char) -> Option<u8> {
    Some(match ch {
        'A'..='Z' => ch as u8 - b'A',
        'a'..='z' => 26 + (ch as u8 - b'a'),
        '0'..='9' => 52 + (ch as u8 - b'0'),
        '+' | '-' => 62,
        '/' | '_' => 63,
        _ => return None,
    })
}

#[cfg(test)]
#[derive(Default)]
struct BitWriter {
    bits: Vec<bool>,
}

#[cfg(test)]
impl BitWriter {
    fn write_bits(&mut self, value: i64, n: usize) {
        for shift in (0..n).rev() {
            self.bits.push((value >> shift) & 1 == 1);
        }
    }

    fn finish(self) -> String {
        let mut chars = String::new();
        let mut i = 0;
        while i < self.bits.len() {
            let mut value = 0_u8;
            for _ in 0..6 {
                value <<= 1;
                if i < self.bits.len() && self.bits[i] {
                    value |= 1;
                }
                i += 1;
            }
            chars.push(six_bit_char(value));
        }
        chars
    }
}

#[cfg(test)]
fn six_bit_char(value: u8) -> char {
    match value {
        0..=25 => (b'A' + value) as char,
        26..=51 => (b'a' + (value - 26)) as char,
        52..=61 => (b'0' + (value - 52)) as char,
        62 => '-',
        _ => '_',
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_dbabm_is_tcf_eu() {
        assert_eq!(decode_header("DBABM"), Some(vec![2]));
    }

    #[test]
    fn header_dbacny_is_tcf_and_us_privacy() {
        assert_eq!(decode_header("DBACNY"), Some(vec![2, 6]));
    }

    #[test]
    fn header_dbabjw_is_canada_and_us_privacy_group() {
        assert_eq!(decode_header("DBABjw"), Some(vec![5, 6]));
    }

    #[test]
    fn header_dbabla_is_usnat() {
        assert_eq!(decode_header("DBABLA"), Some(vec![7]));
    }

    #[test]
    fn tcf_core_accepts_documented_shape() {
        assert!(tcf_shape_error("COwK9wAOwK9wAABABBENAPCgAAAAAAAAAAAYgAAAAAAAA").is_none());
    }

    #[test]
    fn tcf_rejects_v1_prefix() {
        assert!(tcf_shape_error("BAAAAAAAAAA").unwrap().contains("TCF 1.1"));
    }

    #[test]
    fn tcf_core_roundtrip_purpose1() {
        let with = encode_tcf_core(true, false);
        let without = encode_tcf_core(false, false);
        let treated = encode_tcf_core(false, true);
        assert_eq!(
            decode_tcf_core(&with),
            Some(TcfCore {
                purpose1_consent: true,
                purpose_one_treatment: false,
            })
        );
        assert_eq!(
            decode_tcf_core(&without),
            Some(TcfCore {
                purpose1_consent: false,
                purpose_one_treatment: false,
            })
        );
        assert_eq!(
            decode_tcf_core(&treated),
            Some(TcfCore {
                purpose1_consent: false,
                purpose_one_treatment: true,
            })
        );
        assert!(with.starts_with('C'));
        assert!(tcf_shape_error(&with).is_none());
    }
}
