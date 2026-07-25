//! Cross-payload validation of a bid response against its originating bid
//! request.
//!
//! This is the two-pass validator the single-payload walk cannot be: pass
//! one indexes the request (imp ids, offered media subtypes, deal ids, seat
//! constraints, allowed currencies), pass two runs the normal response
//! validation and then checks every bid against the imp it references.

use std::collections::HashMap;

use serde_json::{Map, Value};

use crate::{
    canonical_object,
    validator::{classify_adm, finalize_result, integer_value, validate_bid_response, AdmMarkup},
    Issue, OpenRtbVersion, Severity,
};

pub(crate) fn validate_bid_response_against_request(
    version: OpenRtbVersion,
    request_input: &str,
    response_input: &str,
) -> crate::ValidationResult {
    let mut result = validate_bid_response(version, response_input);

    let request_value = match serde_json::from_str::<Value>(request_input) {
        Ok(value) => value,
        Err(error) => {
            result.issues.push(request_unusable(format!(
                "The bid request supplied for cross-validation is not valid JSON: {error}"
            )));
            return finalize_result(result.issues);
        }
    };
    let Some(request) = request_value.as_object() else {
        result.issues.push(request_unusable(String::from(
            "The bid request supplied for cross-validation is not a JSON object.",
        )));
        return finalize_result(result.issues);
    };

    // The response walk already reported unparseable or non-object
    // responses; cross-checks simply have nothing to add then.
    let Ok(response_value) = serde_json::from_str::<Value>(response_input) else {
        return result;
    };
    let Some(response) = response_value.as_object() else {
        return result;
    };

    let context = RequestContext::index(request);
    cross_validate(version, &context, response, &mut result.issues);
    finalize_result(result.issues)
}

fn request_unusable(message: String) -> Issue {
    Issue {
        id: String::from("openrtb.pair.request_unusable"),
        severity: Severity::Error,
        message,
        path: None,
        section: None,
    }
}

/// Pass one: everything the response side needs to know about the request.
struct RequestContext<'a> {
    id: Option<&'a str>,
    /// `None` when the request does not restrict currencies.
    currencies: Option<Vec<&'a str>>,
    wseat: Option<Vec<&'a str>>,
    bseat: Option<Vec<&'a str>>,
    imps: HashMap<&'a str, ImpContext<'a>>,
}

struct ImpContext<'a> {
    banner: bool,
    video: bool,
    audio: bool,
    native: bool,
    /// `Some` when the imp carries a pmp object; the vec holds its deal ids.
    deal_ids: Option<Vec<&'a str>>,
}

impl<'a> RequestContext<'a> {
    fn index(request: &'a Map<String, Value>) -> Self {
        let mut imps = HashMap::new();
        for imp in request
            .get("imp")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let Some(imp) = imp.as_object() else {
                continue;
            };
            let Some(imp_id) = imp.get("id").and_then(Value::as_str) else {
                continue;
            };

            let deal_ids = imp.get("pmp").and_then(Value::as_object).map(|pmp| {
                pmp.get("deals")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .filter_map(|deal| deal.get("id").and_then(Value::as_str))
                    .collect::<Vec<_>>()
            });

            imps.insert(
                imp_id,
                ImpContext {
                    banner: imp.contains_key("banner"),
                    video: imp.contains_key("video"),
                    audio: imp.contains_key("audio"),
                    native: imp.contains_key("native"),
                    deal_ids,
                },
            );
        }

        Self {
            id: request.get("id").and_then(Value::as_str),
            currencies: string_list(request.get("cur")),
            wseat: string_list(request.get("wseat")),
            bseat: string_list(request.get("bseat")),
            imps,
        }
    }
}

fn string_list(value: Option<&Value>) -> Option<Vec<&str>> {
    value
        .and_then(Value::as_array)
        .map(|items| items.iter().filter_map(Value::as_str).collect())
}

/// Pass two: cross-checks over the already-validated response.
fn cross_validate(
    version: OpenRtbVersion,
    context: &RequestContext<'_>,
    response: &Map<String, Value>,
    issues: &mut Vec<Issue>,
) {
    let response_section = object_section(version, "BidResponse");
    let seatbid_section = object_section(version, "SeatBid");
    let bid_section = object_section(version, "Bid");

    if let (Some(request_id), Some(response_id)) =
        (context.id, response.get("id").and_then(Value::as_str))
    {
        if request_id != response_id {
            issues.push(Issue {
                id: String::from("openrtb.response.request_id_mismatch"),
                severity: Severity::Error,
                message: format!(
                    "Response id \"{response_id}\" does not echo the bid request id \
                     \"{request_id}\"; BidResponse.id must be the id of the request it answers."
                ),
                path: Some(String::from("id")),
                section: response_section.clone(),
            });
        }
    }

    if let (Some(allowed), Some(currency)) = (
        context.currencies.as_deref(),
        response.get("cur").and_then(Value::as_str),
    ) {
        if !allowed.is_empty() && !allowed.contains(&currency) {
            issues.push(Issue {
                id: String::from("openrtb.response.cur_not_allowed"),
                severity: Severity::Error,
                message: format!(
                    "Response currency \"{currency}\" is not among the currencies the request \
                     allows ({}).",
                    allowed.join(", ")
                ),
                path: Some(String::from("cur")),
                section: response_section.clone(),
            });
        }
    }

    for (seatbid_index, seatbid) in response
        .get("seatbid")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .enumerate()
    {
        let Some(seatbid) = seatbid.as_object() else {
            continue;
        };

        if let Some(seat) = seatbid.get("seat").and_then(Value::as_str) {
            let whitelisted = match context.wseat.as_deref() {
                Some(wseat) => wseat.is_empty() || wseat.contains(&seat),
                None => true,
            };
            let blocked = context
                .bseat
                .as_deref()
                .is_some_and(|bseat| bseat.contains(&seat));

            if !whitelisted || blocked {
                let constraint = if blocked {
                    "is on the request's blocked seat list (bseat)"
                } else {
                    "is not on the request's allowed seat list (wseat)"
                };
                issues.push(Issue {
                    id: String::from("openrtb.seatbid.seat_not_allowed"),
                    severity: Severity::Error,
                    message: format!("Seat \"{seat}\" {constraint}."),
                    path: Some(format!("seatbid[{seatbid_index}].seat")),
                    section: seatbid_section.clone(),
                });
            }
        }

        for (bid_index, bid) in seatbid
            .get("bid")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .enumerate()
        {
            let Some(bid) = bid.as_object() else {
                continue;
            };
            let bid_path = format!("seatbid[{seatbid_index}].bid[{bid_index}]");
            cross_validate_bid(context, bid, &bid_path, bid_section.as_deref(), issues);
        }
    }
}

fn cross_validate_bid(
    context: &RequestContext<'_>,
    bid: &Map<String, Value>,
    bid_path: &str,
    bid_section: Option<&str>,
    issues: &mut Vec<Issue>,
) {
    let Some(impid) = bid.get("impid").and_then(Value::as_str) else {
        // A missing impid is the single-payload walk's finding, not ours.
        return;
    };

    let Some(imp) = context.imps.get(impid) else {
        issues.push(Issue {
            id: String::from("openrtb.bid.impid_unknown"),
            severity: Severity::Error,
            message: format!(
                "impid \"{impid}\" does not match the id of any Imp in the bid request."
            ),
            path: Some(format!("{bid_path}.impid")),
            section: bid_section.map(String::from),
        });
        return;
    };

    if let Some(mtype) = bid.get("mtype").and_then(integer_value) {
        let (offered, media_label) = match mtype {
            1 => (imp.banner, "banner"),
            2 => (imp.video, "video"),
            3 => (imp.audio, "audio"),
            4 => (imp.native, "native"),
            // Out-of-range mtype is the value-set check's finding.
            _ => (true, ""),
        };
        if !offered {
            issues.push(Issue {
                id: String::from("openrtb.bid.mtype_not_offered"),
                severity: Severity::Error,
                message: format!(
                    "mtype {mtype} declares {media_label} markup, but imp \"{impid}\" does not \
                     offer a {media_label} subtype."
                ),
                path: Some(format!("{bid_path}.mtype")),
                section: bid_section.map(String::from),
            });
        }
    }

    if let Some(adm) = bid.get("adm").and_then(Value::as_str) {
        let adm_path = format!("{bid_path}.adm");
        match classify_adm(adm) {
            AdmMarkup::NativeJson if !imp.native => issues.push(Issue {
                id: String::from("openrtb.bid.adm.media_type_mismatch"),
                severity: Severity::Error,
                message: format!(
                    "adm is a native JSON payload, but imp \"{impid}\" does not offer a native \
                     subtype."
                ),
                path: Some(adm_path),
                section: bid_section.map(String::from),
            }),
            AdmMarkup::Vast if !imp.video && !imp.audio => issues.push(Issue {
                id: String::from("openrtb.bid.adm.media_type_mismatch"),
                severity: Severity::Error,
                message: format!(
                    "adm is VAST markup, but imp \"{impid}\" offers neither a video nor an \
                     audio subtype."
                ),
                path: Some(adm_path),
                section: bid_section.map(String::from),
            }),
            AdmMarkup::OtherMarkup | AdmMarkup::Other
                if imp.native && !imp.banner && !imp.video && !imp.audio =>
            {
                issues.push(Issue {
                    id: String::from("openrtb.bid.adm.media_type_mismatch"),
                    severity: Severity::Error,
                    message: format!(
                        "imp \"{impid}\" offers only a native subtype, but adm is not a JSON \
                         Native Markup Response."
                    ),
                    path: Some(adm_path),
                    section: bid_section.map(String::from),
                });
            }
            _ => {}
        }
    }

    if let Some(dealid) = bid.get("dealid").and_then(Value::as_str) {
        // Warning, not error: deals arranged out of band and applied at the
        // exchange do legitimately reference ids absent from pmp.deals.
        let known = match imp.deal_ids.as_deref() {
            Some(deal_ids) => deal_ids.contains(&dealid),
            None => false,
        };
        if !known {
            let detail = if imp.deal_ids.is_some() {
                "is not among the deals its pmp object enumerates"
            } else {
                "carries no pmp object at all"
            };
            issues.push(Issue {
                id: String::from("openrtb.bid.dealid_unknown"),
                severity: Severity::Warning,
                message: format!(
                    "dealid \"{dealid}\" references a deal, but imp \"{impid}\" {detail}; \
                     verify the deal was arranged out of band."
                ),
                path: Some(format!("{bid_path}.dealid")),
                section: bid_section.map(String::from),
            });
        }
    }
}

fn object_section(version: OpenRtbVersion, object_name: &str) -> Option<String> {
    canonical_object(version, object_name).map(|object| String::from(object.section))
}
