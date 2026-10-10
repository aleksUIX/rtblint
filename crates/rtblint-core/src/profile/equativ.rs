//! Public Equativ supplier API and shared bidder field contracts.
//! The bidder field referral does not transfer supplier account routing rules.
use serde_json::{Map, Value};

use super::contract::{bids, error, fields, integer_enum, present, required, warning, Field, Kind};
use super::{join_instance_path, value_at};
use crate::Issue;

pub(super) fn validate(
    object_name: &str,
    object: &Map<String, Value>,
    path: &str,
    issues: &mut Vec<Issue>,
    supplier: bool,
) {
    let schema: &[Field] = match object_name {
        "BidRequest" if supplier => REQUEST,
        "BidRequest" => REQUEST_SHARED,
        "Imp" => IMP,
        "Banner" | "Audio" | "Native" => FLOOR,
        "Video" => VIDEO,
        "Site" => SITE,
        "Device" => DEVICE,
        "User" => USER,
        "Regs" => REGS,
        "Source" => SOURCE,
        "BidResponse" => RESPONSE,
        "Bid" => BID,
        _ => &[],
    };
    fields(object, schema, path, issues);
    if object_name == "Publisher" {
        fields(object, PUBLISHER, path, issues);
    }
    if supplier {
        let required_fields: &[&str] = match object_name {
            "BidRequest" => &["device", "user"],
            "Site" => &["domain", "page"],
            "DOOH" => &["domain"],
            "Format" => &["w", "h"],
            "Video" | "Audio" => &["mimes"],
            "User" => &["buyeruid"],
            _ => &[],
        };
        for field in required_fields {
            required(object, field, path, "Equativ supplier API", issues);
        }
        if object_name == "App" && !present(object, "bundle") {
            warning(
                "openrtb.profile.equativ.app_identity_advisory",
                "The Equativ App table marks bundle required, but its parent description strongly recommends App ID or Bundle ID. Missing bundle is advisory while these source requirements conflict.",
                join_instance_path(path, "bundle"),
                issues,
            );
        }
        if object_name == "BidRequest" {
            validate_supplier_request(object, path, issues);
        }
        if object_name == "Video" && !present(object, "plcmt") && !present(object, "placement") {
            error("openrtb.profile.equativ.video_placement", "Equativ requires video placement information. Modern plcmt or documented legacy placement supplies it.", join_instance_path(path, "plcmt"), issues);
        }
        if matches!(object_name, "Banner" | "Video" | "Audio" | "Format") {
            for field in ["w", "h"] {
                if object.get(field).and_then(Value::as_i64) == Some(0) {
                    warning(
                        "openrtb.profile.equativ.zero_dimension",
                        "Equativ ignores requests with an explicitly zero width or height.",
                        join_instance_path(path, field),
                        issues,
                    );
                }
            }
        }
        if object_name == "BidResponse" && bids(object).count() > 10 {
            error(
                "openrtb.profile.equativ.bid_limit",
                "Equativ supplier responses contain at most ten bid objects.",
                join_instance_path(path, "seatbid"),
                issues,
            );
        }
    }
    match object_name {
        "Video" => {
            integer_enum(object, "ext.orientation", &[0, 1, 2], path, issues);
            integer_enum(object, "plcmt", &[1, 2, 3, 4, 5, 6, 7, 8, 9], path, issues);
            if present(object, "ext.rewarded") {
                warning("openrtb.profile.equativ.obsolete_rewarded", "The Equativ guide marks video.ext.rewarded obsolete and recommends imp.rwdd. Current 2.6 acceptance is not promised.", join_instance_path(path, "ext.rewarded"), issues);
            }
        }
        "Site" => integer_enum(object, "ext.amp", &[0, 1], path, issues),
        "Device" => integer_enum(object, "ext.atts", &[0, 1, 2, 3], path, issues),
        "User" => priority_warning(object, "consent", path, issues),
        "Regs" => {
            priority_warning(object, "gdpr", path, issues);
            integer_enum(object, "ext.gdpr", &[0, 1], path, issues);
            integer_enum(object, "ext.dsa.dsarequired", &[0, 1, 2, 3], path, issues);
            integer_enum(object, "ext.dsa.pubrender", &[0, 1, 2], path, issues);
            integer_enum(object, "ext.dsa.datatopub", &[0, 1, 2], path, issues);
        }
        "Source" => {
            priority_warning(object, "schain", path, issues);
            if !present(object, "schain") {
                if let Some(chain) = value_at(object, "ext.schain").and_then(Value::as_object) {
                    let chain_path = join_instance_path(path, "ext.schain");
                    for field in ["complete", "nodes", "ver"] {
                        required(
                            chain,
                            field,
                            &chain_path,
                            "Equativ supplied supply chain",
                            issues,
                        );
                    }
                    integer_enum(chain, "complete", &[0, 1], &chain_path, issues);
                    if let Some(nodes) = chain.get("nodes").and_then(Value::as_array) {
                        for (index, node) in nodes.iter().enumerate() {
                            if let Some(node) = node.as_object() {
                                let node_path = format!("{chain_path}.nodes[{index}]");
                                for field in ["asi", "sid"] {
                                    required(
                                        node,
                                        field,
                                        &node_path,
                                        "Equativ supply chain node",
                                        issues,
                                    );
                                }
                                integer_enum(node, "hp", &[0, 1], &node_path, issues);
                            }
                        }
                    }
                }
            }
        }
        "BidResponse" => {
            if let Some(deal_type) = value_at(object, "ext.dealtype").and_then(Value::as_str) {
                if ![
                    "GuaranteedDeal",
                    "AuctionPackage",
                    "DirectDeal",
                    "PrivateAuction",
                ]
                .contains(&deal_type)
                {
                    error(
                        "openrtb.profile.equativ.deal_type",
                        "Equativ dealtype must be one of its four documented strings.",
                        join_instance_path(path, "ext.dealtype"),
                        issues,
                    );
                }
            }
            // Prose says an array of pixels; the type column says STRING.
            // Accept both documented shapes until the vendor resolves the discrepancy.
            if let Some(value) =
                value_at(object, "ext.impression_tracking_url").filter(|v| !v.is_null())
            {
                let valid = value.is_string()
                    || value
                        .as_array()
                        .is_some_and(|v| v.iter().all(Value::is_string));
                if !valid {
                    error(
                        "openrtb.profile.field_type",
                        "The documented tracking URL is a string or an array of strings.",
                        join_instance_path(path, "ext.impression_tracking_url"),
                        issues,
                    );
                }
            }
        }
        "Bid" => {
            integer_enum(object, "ext.dsa.adrender", &[0, 1], path, issues);
            for field in ["ext.dsa.behalf", "ext.dsa.paid"] {
                if value_at(object, field)
                    .and_then(Value::as_str)
                    .is_some_and(|v| v.chars().count() > 100)
                {
                    error("openrtb.profile.equativ.dsa_name_length", "Equativ DSA advertiser and payer names have a maximum of 100 Unicode characters.", join_instance_path(path, field), issues);
                }
            }
        }
        _ => {}
    }
}

fn priority_warning(object: &Map<String, Value>, field: &str, path: &str, issues: &mut Vec<Issue>) {
    let legacy = format!("ext.{field}");
    if present(object, field) && present(object, &legacy) {
        warning("openrtb.profile.equativ.mainline_precedence", "Equativ prioritizes the mainline field when both mainline and legacy values are present.", join_instance_path(path, &legacy), issues);
    }
}

fn validate_supplier_request(request: &Map<String, Value>, path: &str, issues: &mut Vec<Issue>) {
    let network = present(request, "ext.network_id")
        || ["site.publisher.id", "app.publisher.id", "dooh.publisher.id"]
            .iter()
            .any(|field| present(request, field));
    if !network {
        error("openrtb.profile.equativ.network_id", "Supplier requests must provide the Equativ network identifier through publisher.id or the configured ext.network_id method.", join_instance_path(path, "ext.network_id"), issues);
    }
    if let Some(imps) = request.get("imp").and_then(Value::as_array) {
        if imps.len() > 10 {
            error(
                "openrtb.profile.equativ.impression_limit",
                "Equativ accepts at most ten impression objects in a supplier request.",
                join_instance_path(path, "imp"),
                issues,
            );
        }
        let non_instream = imps.iter().any(|imp| {
            let Some(imp) = imp.as_object() else {
                return false;
            };
            value_at(imp, "video.plcmt").and_then(Value::as_i64) != Some(1)
                && value_at(imp, "video.placement").and_then(Value::as_i64) != Some(1)
        });
        if imps.len() > 1 && non_instream {
            let first = imps
                .first()
                .and_then(|imp| imp.get("tagid"))
                .and_then(Value::as_str)
                .filter(|id| !id.is_empty());
            if first.is_none()
                || imps
                    .iter()
                    .any(|imp| imp.get("tagid").and_then(Value::as_str) != first)
            {
                warning("openrtb.profile.equativ.mio_first_only", "Equativ processes only the first non-instream impression when placement tag IDs are missing or differ. Unified requests also require account activation.", join_instance_path(path, "imp"), issues);
            }
        }
    }
    let mut directpay_count = 0;
    for context in ["site", "app", "dooh"] {
        if value_at(request, &format!("{context}.publisher.ext.directpay"))
            .is_some_and(|v| !v.is_null())
        {
            directpay_count += 1;
        }
    }
    if directpay_count > 1 {
        error(
            "openrtb.profile.equativ.directpay_once",
            "Publisher directpay can be sent only once in a request.",
            path.to_owned(),
            issues,
        );
    }
}

pub(super) fn validate_pair(
    request: &Map<String, Value>,
    response: &Map<String, Value>,
    issues: &mut Vec<Issue>,
    _supplier: bool,
) {
    if matches!(
        value_at(request, "regs.ext.dsa.dsarequired").and_then(Value::as_i64),
        Some(2 | 3)
    ) {
        for (path, bid) in bids(response) {
            required(
                bid,
                "ext.dsa",
                &path,
                "Equativ required DSA response",
                issues,
            );
        }
    }
}

const STR: Kind = Kind::String;
const INT: Kind = Kind::Integer;
const FLOAT: Kind = Kind::Number;
const INTEGERS: Kind = Kind::Array(&INT);
const TRANSPARENCY: &[Field] = &[
    Field {
        name: "domain",
        kind: STR,
    },
    Field {
        name: "dsaparams",
        kind: INTEGERS,
    },
];
const DSA: &[Field] = &[
    Field {
        name: "behalf",
        kind: STR,
    },
    Field {
        name: "paid",
        kind: STR,
    },
    Field {
        name: "adrender",
        kind: INT,
    },
    Field {
        name: "transparency",
        kind: Kind::Array(&Kind::Object(TRANSPARENCY)),
    },
];
const REGS_DSA: &[Field] = &[
    Field {
        name: "dsarequired",
        kind: INT,
    },
    Field {
        name: "pubrender",
        kind: INT,
    },
    Field {
        name: "datatopub",
        kind: INT,
    },
    Field {
        name: "transparency",
        kind: Kind::Array(&Kind::Object(TRANSPARENCY)),
    },
];
const FEEDBACK: &[Field] = &[
    Field {
        name: "feedback_token",
        kind: STR,
    },
    Field {
        name: "loss",
        kind: INT,
    },
    Field {
        name: "price",
        kind: FLOAT,
    },
];
const REQUEST: &[Field] = &[Field {
    name: "ext",
    kind: Kind::Object(&[
        Field {
            name: "network_id",
            kind: INT,
        },
        Field {
            name: "bid_feedback",
            kind: Kind::Array(&Kind::Object(FEEDBACK)),
        },
    ]),
}];
const REQUEST_SHARED: &[Field] = &[Field {
    name: "ext",
    kind: Kind::Object(&[Field {
        name: "bid_feedback",
        kind: Kind::Array(&Kind::Object(FEEDBACK)),
    }]),
}];
const IMP: &[Field] = &[Field {
    name: "ext",
    kind: Kind::Object(&[
        Field {
            name: "bidder",
            kind: Kind::Object(&[
                Field {
                    name: "plcmtuuid",
                    kind: STR,
                },
                Field {
                    name: "siteId",
                    kind: INT,
                },
                Field {
                    name: "pageId",
                    kind: INT,
                },
                Field {
                    name: "formatId",
                    kind: INT,
                },
            ]),
        },
        Field {
            name: "gpid",
            kind: STR,
        },
        Field {
            name: "dfp_ad_unit_code",
            kind: STR,
        },
        Field {
            name: "bid_feedback",
            kind: Kind::Array(&Kind::Object(FEEDBACK)),
        },
    ]),
}];
const FLOOR: &[Field] = &[Field {
    name: "ext",
    kind: Kind::Object(&[Field {
        name: "bidfloor",
        kind: FLOAT,
    }]),
}];
const VIDEO: &[Field] = &[Field {
    name: "ext",
    kind: Kind::Object(&[
        Field {
            name: "bidfloor",
            kind: FLOAT,
        },
        Field {
            name: "orientation",
            kind: INT,
        },
        Field {
            name: "rewarded",
            kind: INT,
        },
    ]),
}];
const SITE: &[Field] = &[Field {
    name: "ext",
    kind: Kind::Object(&[Field {
        name: "amp",
        kind: INT,
    }]),
}];
const DEVICE: &[Field] = &[Field {
    name: "ext",
    kind: Kind::Object(&[
        Field {
            name: "atts",
            kind: INT,
        },
        Field {
            name: "ifa_type",
            kind: STR,
        },
    ]),
}];
const USER: &[Field] = &[Field {
    name: "ext",
    kind: Kind::Object(&[
        Field {
            name: "consent",
            kind: STR,
        },
        Field {
            name: "eids",
            kind: Kind::Array(&Kind::Object(&[
                Field {
                    name: "source",
                    kind: STR,
                },
                Field {
                    name: "inserter",
                    kind: STR,
                },
                Field {
                    name: "matcher",
                    kind: STR,
                },
                Field {
                    name: "mm",
                    kind: INT,
                },
                Field {
                    name: "ext",
                    kind: Kind::Object(&[]),
                },
                Field {
                    name: "uids",
                    kind: Kind::Array(&Kind::Object(&[
                        Field {
                            name: "id",
                            kind: STR,
                        },
                        Field {
                            name: "atype",
                            kind: INT,
                        },
                        Field {
                            name: "ext",
                            kind: Kind::Object(&[]),
                        },
                    ])),
                },
            ])),
        },
    ]),
}];
const REGS: &[Field] = &[Field {
    name: "ext",
    kind: Kind::Object(&[
        Field {
            name: "gdpr",
            kind: INT,
        },
        Field {
            name: "dsa",
            kind: Kind::Object(REGS_DSA),
        },
    ]),
}];
const SOURCE: &[Field] = &[Field {
    name: "ext",
    kind: Kind::Object(&[
        Field {
            name: "omidpn",
            kind: STR,
        },
        Field {
            name: "omidpv",
            kind: STR,
        },
        Field {
            name: "schain",
            kind: Kind::Object(&[
                Field {
                    name: "complete",
                    kind: INT,
                },
                Field {
                    name: "ver",
                    kind: STR,
                },
                Field {
                    name: "nodes",
                    kind: Kind::Array(&Kind::Object(&[
                        Field {
                            name: "asi",
                            kind: STR,
                        },
                        Field {
                            name: "sid",
                            kind: STR,
                        },
                        Field {
                            name: "hp",
                            kind: INT,
                        },
                        Field {
                            name: "rid",
                            kind: STR,
                        },
                        Field {
                            name: "name",
                            kind: STR,
                        },
                        Field {
                            name: "domain",
                            kind: STR,
                        },
                    ])),
                },
            ]),
        },
    ]),
}];
const PUBLISHER: &[Field] = &[Field {
    name: "ext",
    kind: Kind::Object(&[Field {
        name: "directpay",
        kind: Kind::CompatibleFlag,
    }]),
}];
const RESPONSE: &[Field] = &[Field {
    name: "ext",
    kind: Kind::Object(&[
        Field {
            name: "dealtype",
            kind: STR,
        },
        Field {
            name: "feedback_token",
            kind: STR,
        },
    ]),
}];
const BID: &[Field] = &[Field {
    name: "ext",
    kind: Kind::Object(&[Field {
        name: "dsa",
        kind: Kind::Object(DSA),
    }]),
}];
