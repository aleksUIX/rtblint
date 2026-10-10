//! Adform's supplier OpenRTB Handler, including its documented DSA envelope.

use super::{
    contract::{self, Field, Kind},
    join_instance_path, value_at,
};
use crate::Issue;
use serde_json::{Map, Value};

const INTS: Kind = Kind::Array(&Kind::Integer);
const UID: &[Field] = &[
    Field {
        name: "id",
        kind: Kind::String,
    },
    Field {
        name: "atype",
        kind: Kind::Integer,
    },
    Field {
        name: "ext",
        kind: Kind::Object(&[]),
    },
];
const EID: &[Field] = &[
    Field {
        name: "source",
        kind: Kind::String,
    },
    Field {
        name: "uids",
        kind: Kind::Array(&Kind::Object(UID)),
    },
    Field {
        name: "ext",
        kind: Kind::Object(&[]),
    },
];
const SUPPLY_CHAIN_NODE: &[Field] = &[
    Field {
        name: "asi",
        kind: Kind::String,
    },
    Field {
        name: "sid",
        kind: Kind::String,
    },
    Field {
        name: "hp",
        kind: Kind::Flag,
    },
    Field {
        name: "rid",
        kind: Kind::String,
    },
    Field {
        name: "name",
        kind: Kind::String,
    },
    Field {
        name: "domain",
        kind: Kind::String,
    },
    Field {
        name: "ext",
        kind: Kind::Object(&[]),
    },
];
const SUPPLY_CHAIN: &[Field] = &[
    Field {
        name: "ver",
        kind: Kind::String,
    },
    Field {
        name: "complete",
        kind: Kind::Flag,
    },
    Field {
        name: "nodes",
        kind: Kind::Array(&Kind::Object(SUPPLY_CHAIN_NODE)),
    },
    Field {
        name: "ext",
        kind: Kind::Object(&[]),
    },
];
const TRANSPARENCY: &[Field] = &[
    Field {
        name: "domain",
        kind: Kind::String,
    },
    Field {
        name: "dsaparams",
        kind: INTS,
    },
    // Adform's own request examples use params rather than dsaparams.
    Field {
        name: "params",
        kind: INTS,
    },
];
const REQUEST_DSA: &[Field] = &[
    Field {
        name: "dsarequired",
        kind: Kind::Integer,
    },
    Field {
        name: "required",
        kind: Kind::Integer,
    },
    Field {
        name: "pubrender",
        kind: Kind::Integer,
    },
    Field {
        name: "datatopub",
        kind: Kind::Integer,
    },
    Field {
        name: "transparency",
        kind: Kind::Array(&Kind::Object(TRANSPARENCY)),
    },
];
const RESPONSE_DSA: &[Field] = &[
    Field {
        name: "behalf",
        kind: Kind::String,
    },
    Field {
        name: "paid",
        kind: Kind::String,
    },
    Field {
        name: "adrender",
        kind: Kind::Flag,
    },
    Field {
        name: "transparency",
        kind: Kind::Array(&Kind::Object(TRANSPARENCY)),
    },
];
const RESPONSE_SOURCE: &str = "https://www.adformhelp.com/hc/en-us/articles/9739105213841-Adform-OpenRTB-Handler-Bid-Response-Specifications";

pub(super) fn validate(
    object_name: &str,
    object: &Map<String, Value>,
    path: &str,
    issues: &mut Vec<Issue>,
) {
    match object_name {
        "BidRequest" => contract::fields(
            object,
            &[Field {
                name: "ext",
                kind: Kind::Object(&[Field {
                    name: "pt",
                    kind: Kind::String,
                }]),
            }],
            path,
            issues,
        ),
        "Regs" => {
            contract::fields(
                object,
                &[
                    Field {
                        name: "gpp",
                        kind: Kind::String,
                    },
                    // The source's description specifies section IDs as an array.
                    Field {
                        name: "gpp_sid",
                        kind: INTS,
                    },
                    Field {
                        name: "ext",
                        kind: Kind::Object(&[
                            Field {
                                name: "gpc",
                                kind: Kind::String,
                            },
                            Field {
                                name: "dsa",
                                kind: Kind::Object(REQUEST_DSA),
                            },
                        ]),
                    },
                ],
                path,
                issues,
            );
            contract::integer_enum(object, "ext.gdpr", &[0, 1], path, issues);
            if let Some(dsa) = value_at(object, "ext.dsa").and_then(Value::as_object) {
                let base = join_instance_path(path, "ext.dsa");
                for field in ["dsarequired", "required"] {
                    contract::integer_enum(dsa, field, &[0, 1, 2, 3], &base, issues);
                }
                for field in ["pubrender", "datatopub"] {
                    contract::integer_enum(dsa, field, &[0, 1, 2], &base, issues);
                }
                parameters(dsa, &base, issues);
            }
        }
        "Native" => native_dimensions(object, path, issues),
        "User" => contract::fields(
            object,
            &[
                Field {
                    name: "eids",
                    kind: Kind::Array(&Kind::Object(EID)),
                },
                Field {
                    name: "ext",
                    kind: Kind::Object(&[
                        Field {
                            name: "consent",
                            kind: Kind::String,
                        },
                        Field {
                            name: "eids",
                            kind: Kind::Array(&Kind::Object(EID)),
                        },
                        Field {
                            name: "digitrust",
                            kind: Kind::Object(&[Field {
                                name: "id",
                                kind: Kind::String,
                            }]),
                        },
                    ]),
                },
            ],
            path,
            issues,
        ),
        "Source" => {
            contract::fields(
                object,
                &[Field {
                    name: "ext",
                    kind: Kind::Object(&[Field {
                        name: "schain",
                        kind: Kind::Object(SUPPLY_CHAIN),
                    }]),
                }],
                path,
                issues,
            );
            if let Some(chain) = value_at(object, "ext.schain").and_then(Value::as_object) {
                let base = join_instance_path(path, "ext.schain");
                for field in ["ver", "complete", "nodes"] {
                    contract::required(chain, field, &base, "Adform supply chain", issues);
                }
                if let Some(nodes) = chain.get("nodes").and_then(Value::as_array) {
                    if nodes.is_empty() {
                        error(
                            "schain_nodes",
                            "Adform's declared supply chain requires at least one node.",
                            join_instance_path(&base, "nodes"),
                            issues,
                        );
                    }
                    for (index, node) in nodes.iter().enumerate() {
                        if let Some(node) = node.as_object() {
                            for field in ["asi", "sid"] {
                                contract::required(
                                    node,
                                    field,
                                    &format!("{base}.nodes[{index}]"),
                                    "Adform supply-chain node",
                                    issues,
                                );
                            }
                        }
                    }
                }
            }
        }
        "Bid" => {
            let start = issues.len();
            contract::fields(
                object,
                &[
                    Field {
                        name: "dur",
                        kind: Kind::Integer,
                    },
                    Field {
                        name: "ext",
                        kind: Kind::Object(&[Field {
                            name: "dsa",
                            kind: Kind::Object(RESPONSE_DSA),
                        }]),
                    },
                ],
                path,
                issues,
            );
            if let Some(dsa) = value_at(object, "ext.dsa").and_then(Value::as_object) {
                let base = join_instance_path(path, "ext.dsa");
                contract::required(dsa, "paid", &base, "Adform DSA response", issues);
                for field in ["behalf", "paid"] {
                    if dsa
                        .get(field)
                        .and_then(Value::as_str)
                        .is_some_and(|value| value.chars().count() > 100)
                    {
                        error("dsa_name_length", "Adform DSA advertiser names must contain at most 100 Unicode characters.", join_instance_path(&base, field), issues);
                    }
                }
                parameters(dsa, &base, issues);
            }
            for issue in &mut issues[start..] {
                issue.section = Some(RESPONSE_SOURCE.into());
            }
        }
        _ => {}
    }
}

pub(super) fn validate_pair(
    request: &Map<String, Value>,
    response: &Map<String, Value>,
    issues: &mut Vec<Issue>,
) {
    let required = value_at(request, "regs.ext.dsa")
        .and_then(Value::as_object)
        .is_some_and(|dsa| {
            ["dsarequired", "required"]
                .iter()
                .any(|field| matches!(dsa.get(*field).and_then(Value::as_i64), Some(2 | 3)))
        });
    if !required {
        return;
    }
    for (path, bid) in contract::bids(response) {
        if contract::matching_imp(request, bid).is_none() {
            continue;
        }
        if !value_at(bid, "ext.dsa").is_some_and(Value::is_object) {
            error("dsa_required", "Adform requires DSA response information when the Handler request's DSA requirement is 2 or 3.", join_instance_path(&path, "ext.dsa"), issues);
        }
    }
}

fn parameters(dsa: &Map<String, Value>, path: &str, issues: &mut Vec<Issue>) {
    let Some(entries) = dsa.get("transparency").and_then(Value::as_array) else {
        return;
    };
    for (index, entry) in entries.iter().enumerate() {
        for field in ["dsaparams", "params"] {
            let Some(params) = entry.get(field).and_then(Value::as_array) else {
                continue;
            };
            for (parameter_index, parameter) in params.iter().enumerate() {
                if !parameter
                    .as_i64()
                    .is_some_and(|value| matches!(value, 1..=3))
                {
                    error(
                        "dsa_parameter",
                        "Adform DSA targeting parameter must be 1, 2, or 3.",
                        format!("{path}.transparency[{index}].{field}[{parameter_index}]"),
                        issues,
                    );
                }
            }
        }
    }
}

fn native_dimensions(object: &Map<String, Value>, path: &str, issues: &mut Vec<Issue>) {
    let Some(encoded) = object.get("request").and_then(Value::as_str) else {
        return;
    };
    let Ok(native) = serde_json::from_str::<Value>(encoded) else {
        return;
    };
    let native = native.get("native").unwrap_or(&native);
    let Some(assets) = native.get("assets").and_then(Value::as_array) else {
        return;
    };
    for (index, asset) in assets.iter().enumerate() {
        let Some(image) = asset.get("img").and_then(Value::as_object) else {
            continue;
        };
        if ["w", "h", "wmin", "hmin"].iter().all(|field| {
            !image
                .get(*field)
                .and_then(Value::as_i64)
                .is_some_and(|value| value > 0)
        }) {
            let start = issues.len();
            error("native_image_size", "Adform Native image requests need image dimensions; unbounded images do not generate impressions.", format!("{path}.request.assets[{index}].img"), issues);
            issues[start].section = Some("https://www.adformhelp.com/hc/en-us/articles/9739088948113-Adform-OpenRTB-Handler-Sample-Bid-Requests".into());
        }
    }
}

fn error(suffix: &str, message: &str, path: String, issues: &mut Vec<Issue>) {
    contract::error(
        &format!("openrtb.profile.adform.{suffix}"),
        message,
        path,
        issues,
    );
}
