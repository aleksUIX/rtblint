//! OpenRTB substitution macros in `nurl`, `burl`, `lurl`, and `adm`.
//!
//! The spec lists a closed set of `${AUCTION_*}` tokens. Unknown names are
//! almost always typos (`${AUCTION_PRCIE}`) that survive until the notice
//! URL 404s. A billing notice without `${AUCTION_PRICE}` cannot settle.

use serde_json::{Map, Value};

use crate::{Issue, Severity};

const SECTION: &str = "OpenRTB 2.6 §4.4";

const MACROS: &[&str] = &[
    "AUCTION_ID",
    "AUCTION_BID_ID",
    "AUCTION_IMP_ID",
    "AUCTION_SEAT_ID",
    "AUCTION_AD_ID",
    "AUCTION_PRICE",
    "AUCTION_CURRENCY",
    "AUCTION_MBR",
    "AUCTION_LOSS",
    "AUCTION_MIN_TO_WIN",
    "AUCTION_MULTIPLIER",
    "AUCTION_IMP_TS",
    "AUCTION_DISCOUNT_PCT",
    "AUCTION_DISCOUNT_CPM",
];

const NOTICE_FIELDS: [&str; 4] = ["nurl", "burl", "lurl", "adm"];

pub(crate) fn validate_bid_macros(
    bid: &Map<String, Value>,
    instance_path: &str,
    issues: &mut Vec<Issue>,
) {
    let mut price_on_billing = false;
    let mut has_billing = false;
    let mut lurl_has_loss = false;
    let mut has_lurl = false;

    for field in NOTICE_FIELDS {
        let Some(text) = bid.get(field).and_then(Value::as_str) else {
            continue;
        };
        let path = join_path(instance_path, field);
        for_each_macro(text, |name| {
            if !MACROS.contains(&name) {
                issues.push(issue(
                    "openrtb.macro.unknown",
                    Severity::Warning,
                    format!(
                        "${{{name}}} is not a documented OpenRTB substitution macro. Known \
                         names are AUCTION_ID, AUCTION_BID_ID, AUCTION_IMP_ID, AUCTION_SEAT_ID, \
                         AUCTION_AD_ID, AUCTION_PRICE, AUCTION_CURRENCY, AUCTION_MBR, \
                         AUCTION_LOSS, AUCTION_MIN_TO_WIN, AUCTION_MULTIPLIER, AUCTION_IMP_TS, \
                         AUCTION_DISCOUNT_PCT, AUCTION_DISCOUNT_CPM."
                    ),
                    path.clone(),
                ));
            }
        });

        match field {
            "nurl" | "burl" => {
                has_billing = true;
                if contains_macro(text, "AUCTION_PRICE") {
                    price_on_billing = true;
                }
            }
            "lurl" => {
                has_lurl = true;
                lurl_has_loss = contains_macro(text, "AUCTION_LOSS");
            }
            _ => {}
        }
    }

    if has_billing && !price_on_billing {
        let path = if bid.get("burl").and_then(Value::as_str).is_some() {
            join_path(instance_path, "burl")
        } else {
            join_path(instance_path, "nurl")
        };
        issues.push(issue(
            "openrtb.bid.price_macro_missing",
            Severity::Warning,
            String::from(
                "nurl or burl is present without ${AUCTION_PRICE}; a billing notice cannot \
                 report the clearing price.",
            ),
            path,
        ));
    }

    if has_lurl && !lurl_has_loss {
        issues.push(issue(
            "openrtb.bid.loss_macro_missing",
            Severity::Warning,
            String::from(
                "lurl is present without ${AUCTION_LOSS}; a loss notice cannot report why the \
                 bid lost.",
            ),
            join_path(instance_path, "lurl"),
        ));
    }
}

fn contains_macro(text: &str, name: &str) -> bool {
    let mut found = false;
    for_each_macro(text, |found_name| {
        if found_name == name {
            found = true;
        }
    });
    found
}

/// Walks `${NAME}` and `${NAME:ENCODING}` tokens. Exchange `%%MACRO%%`
/// spellings are ignored; they are not OpenRTB.
fn for_each_macro(text: &str, mut visit: impl FnMut(&str)) {
    let bytes = text.as_bytes();
    let mut i = 0;
    while i + 1 < bytes.len() {
        if bytes[i] == b'$' && bytes[i + 1] == b'{' {
            let rest = &text[i + 2..];
            if let Some(end) = rest.find('}') {
                let inner = &rest[..end];
                if !inner.is_empty() {
                    let name = inner.split_once(':').map(|(name, _)| name).unwrap_or(inner);
                    if name.bytes().all(|b| b.is_ascii_uppercase() || b == b'_') {
                        visit(name);
                    }
                }
                i += 2 + end + 1;
                continue;
            }
        }
        i += 1;
    }
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
        section: Some(String::from(SECTION)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encoding_suffix_is_still_the_macro() {
        let mut names = Vec::new();
        for_each_macro("https://x/${AUCTION_PRICE:B64}", |name| {
            names.push(name.to_owned())
        });
        assert_eq!(names, vec!["AUCTION_PRICE"]);
    }

    #[test]
    fn google_percent_macros_are_ignored() {
        let mut names = Vec::new();
        for_each_macro("https://x/%%WINNING_PRICE%%", |name| {
            names.push(name.to_owned())
        });
        assert!(names.is_empty());
    }
}
