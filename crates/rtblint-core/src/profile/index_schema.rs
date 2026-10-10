// Field descriptors transcribed from Index Exchange public tables reviewed 2026-10-09.
// Requiredness and documented contradictions are handled in index_exchange.rs.
const INDEX_SELLER_BIDREQUEST: &[Field] = &[Field {
    name: "ssl",
    kind: Kind::Integer,
}];
const INDEX_SELLER_SOURCE: &[Field] = &[
    Field {
        name: "omidpn",
        kind: Kind::String,
    },
    Field {
        name: "omidpv",
        kind: Kind::String,
    },
];
const INDEX_SELLER_REGS: &[Field] = &[
    Field {
        name: "dsa",
        kind: Kind::Object(INDEX_SELLER_DSA),
    },
    Field {
        name: "gdpr",
        kind: Kind::Integer,
    },
];
const INDEX_SELLER_DSA: &[Field] = &[
    Field {
        name: "datatopub",
        kind: Kind::Integer,
    },
    Field {
        name: "pubrender",
        kind: Kind::Integer,
    },
    Field {
        name: "dsarequired",
        kind: Kind::Integer,
    },
    Field {
        name: "transparency",
        kind: Kind::Array(&Kind::Object(INDEX_SELLER_TRANSPARENCY)),
    },
];
const INDEX_SELLER_TRANSPARENCY: &[Field] = &[
    Field {
        name: "domain",
        kind: Kind::String,
    },
    Field {
        name: "dsaparams",
        kind: Kind::Array(&Kind::Integer),
    },
];
const INDEX_SELLER_IMP: &[Field] = &[
    Field {
        name: "ae",
        kind: Kind::Integer,
    },
    Field {
        name: "dfp_ad_unit_code",
        kind: Kind::String,
    },
    Field {
        name: "data",
        kind: Kind::Object(&[]),
    },
    Field {
        name: "gpid",
        kind: Kind::String,
    },
    Field {
        name: "skadn",
        kind: Kind::Object(INDEX_SELLER_SKAD),
    },
    Field {
        name: "sid",
        kind: Kind::String,
    },
    Field {
        name: "tid",
        kind: Kind::String,
    },
];
const INDEX_SELLER_SKAD: &[Field] = &[
    Field {
        name: "version",
        kind: Kind::String,
    },
    Field {
        name: "versions",
        kind: Kind::Array(&Kind::String),
    },
    Field {
        name: "sourceapp",
        kind: Kind::String,
    },
    Field {
        name: "skadnetids",
        kind: Kind::Array(&Kind::String),
    },
    Field {
        name: "skadnetlist",
        kind: Kind::Object(INDEX_SELLER_SKADLIST),
    },
];
const INDEX_SELLER_SKADLIST: &[Field] = &[
    Field {
        name: "max",
        kind: Kind::Integer,
    },
    Field {
        name: "excl",
        kind: Kind::Array(&Kind::Integer),
    },
    Field {
        name: "addl",
        kind: Kind::Array(&Kind::String),
    },
];
const INDEX_SELLER_BANNER: &[Field] = &[Field {
    name: "bidfloor",
    kind: Kind::Number,
}];
const INDEX_SELLER_VIDEO: &[Field] = &[
    Field {
        name: "rewarded",
        kind: Kind::Integer,
    },
    Field {
        name: "bidfloor",
        kind: Kind::Number,
    },
];
const INDEX_SELLER_NATIVE: &[Field] = &[Field {
    name: "bidfloor",
    kind: Kind::Number,
}];
const INDEX_SELLER_DEAL: &[Field] = &[Field {
    name: "guaranteed",
    kind: Kind::Integer,
}];
const INDEX_SELLER_SITE: &[Field] = &[
    Field {
        name: "data",
        kind: Kind::Object(&[]),
    },
    Field {
        name: "inventorypartnerdomain",
        kind: Kind::String,
    },
];
const INDEX_SELLER_APP: &[Field] = &[
    Field {
        name: "inventorypartnerdomain",
        kind: Kind::String,
    },
    Field {
        name: "data",
        kind: Kind::Object(&[]),
    },
];
const INDEX_SELLER_CONTENT: &[Field] = &[
    Field {
        name: "channel",
        kind: Kind::String,
    },
    Field {
        name: "content_channel",
        kind: Kind::String,
    },
    Field {
        name: "network",
        kind: Kind::String,
    },
    Field {
        name: "content_network",
        kind: Kind::String,
    },
    Field {
        name: "distrib_name",
        kind: Kind::String,
    },
];
const INDEX_SELLER_DEVICE: &[Field] = &[
    Field {
        name: "atts",
        kind: Kind::Integer,
    },
    Field {
        name: "cdep",
        kind: Kind::String,
    },
    Field {
        name: "ifv",
        kind: Kind::String,
    },
    Field {
        name: "ifa_type",
        kind: Kind::String,
    },
];
const INDEX_SELLER_USER: &[Field] = &[
    Field {
        name: "consent",
        kind: Kind::String,
    },
    Field {
        name: "data",
        kind: Kind::Object(&[]),
    },
];
const INDEX_SELLER_EID: &[Field] = &[Field {
    name: "rtipartner",
    kind: Kind::String,
}];
const INDEX_SELLER_UID: &[Field] = &[Field {
    name: "rtipartner",
    kind: Kind::String,
}];
const INDEX_SELLER_DATA: &[Field] = &[Field {
    name: "segtax",
    kind: Kind::Integer,
}];
fn index_seller_schema(object: &str) -> &'static [Field] {
    match object {
        "BidRequest" => INDEX_SELLER_BIDREQUEST,
        "Source" => INDEX_SELLER_SOURCE,
        "Regs" => INDEX_SELLER_REGS,
        "Imp" => INDEX_SELLER_IMP,
        "Banner" => INDEX_SELLER_BANNER,
        "Video" => INDEX_SELLER_VIDEO,
        "Native" => INDEX_SELLER_NATIVE,
        "Deal" => INDEX_SELLER_DEAL,
        "Site" => INDEX_SELLER_SITE,
        "App" => INDEX_SELLER_APP,
        "Content" => INDEX_SELLER_CONTENT,
        "Device" => INDEX_SELLER_DEVICE,
        "User" => INDEX_SELLER_USER,
        "EID" => INDEX_SELLER_EID,
        "UID" => INDEX_SELLER_UID,
        "Data" => INDEX_SELLER_DATA,
        _ => &[],
    }
}
const INDEX_DSP_BIDREQUEST: &[Field] = &[
    Field {
        name: "placement",
        kind: Kind::Object(INDEX_DSP_PLACEMENT),
    },
    Field {
        name: "tz",
        kind: Kind::Integer,
    },
];
const INDEX_DSP_PLACEMENT: &[Field] = &[
    Field {
        name: "name",
        kind: Kind::String,
    },
    Field {
        name: "private",
        kind: Kind::CompatibleFlag,
    },
];
const INDEX_DSP_SOURCE: &[Field] = &[
    Field {
        name: "sourceType",
        kind: Kind::Integer,
    },
    Field {
        name: "sourceOrigin",
        kind: Kind::Integer,
    },
    Field {
        name: "omidpn",
        kind: Kind::String,
    },
    Field {
        name: "omidpv",
        kind: Kind::String,
    },
];
const INDEX_DSP_REGS: &[Field] = &[
    Field {
        name: "dsa",
        kind: Kind::Object(INDEX_DSP_DSA),
    },
    Field {
        name: "gdpr",
        kind: Kind::Integer,
    },
    Field {
        name: "us_privacy",
        kind: Kind::String,
    },
];
const INDEX_DSP_DSA: &[Field] = &[
    Field {
        name: "datatopub",
        kind: Kind::Integer,
    },
    Field {
        name: "pubrender",
        kind: Kind::Integer,
    },
    Field {
        name: "transparency",
        kind: Kind::Array(&Kind::Object(INDEX_DSP_TRANSPARENCY)),
    },
    Field {
        name: "dsarequired",
        kind: Kind::Integer,
    },
];
const INDEX_DSP_TRANSPARENCY: &[Field] = &[
    Field {
        name: "domain",
        kind: Kind::String,
    },
    Field {
        name: "dsaparams",
        kind: Kind::Array(&Kind::Integer),
    },
];
const INDEX_DSP_IMP: &[Field] = &[
    Field {
        name: "bcrid",
        kind: Kind::Array(&Kind::String),
    },
    Field {
        name: "floor",
        kind: Kind::Array(&Kind::Integer),
    },
    Field {
        name: "hb",
        kind: Kind::Integer,
    },
    Field {
        name: "wopv",
        kind: Kind::String,
    },
    Field {
        name: "skadn",
        kind: Kind::Object(INDEX_DSP_SKAD),
    },
    Field {
        name: "gpid",
        kind: Kind::String,
    },
    Field {
        name: "tid",
        kind: Kind::String,
    },
    Field {
        name: "data",
        kind: Kind::Object(&[]),
    },
    Field {
        name: "ae",
        kind: Kind::Integer,
    },
    Field {
        name: "sid",
        kind: Kind::String,
    },
];
const INDEX_DSP_BANNER: &[Field] = &[Field {
    name: "bidfloor",
    kind: Kind::Number,
}];
const INDEX_DSP_VIDEO: &[Field] = &[
    Field {
        name: "bidfloor",
        kind: Kind::Number,
    },
    Field {
        name: "playertype",
        kind: Kind::Integer,
    },
    Field {
        name: "renderedBy",
        kind: Kind::Integer,
    },
    Field {
        name: "rewarded",
        kind: Kind::Integer,
    },
];
const INDEX_DSP_NATIVE: &[Field] = &[
    Field {
        name: "bidfloor",
        kind: Kind::Number,
    },
    Field {
        name: "renderedBy",
        kind: Kind::Integer,
    },
];
const INDEX_DSP_PMP: &[Field] = &[Field {
    name: "name",
    kind: Kind::String,
}];
const INDEX_DSP_DEAL: &[Field] = &[
    Field {
        name: "disc",
        kind: Kind::Number,
    },
    Field {
        name: "guaranteed",
        kind: Kind::Integer,
    },
];
const INDEX_DSP_SKAD: &[Field] = &[
    Field {
        name: "version",
        kind: Kind::String,
    },
    Field {
        name: "versions",
        kind: Kind::Array(&Kind::String),
    },
    Field {
        name: "sourceapp",
        kind: Kind::String,
    },
    Field {
        name: "skadnetids",
        kind: Kind::Array(&Kind::String),
    },
    Field {
        name: "skadnetlist",
        kind: Kind::Object(INDEX_DSP_SKADLIST),
    },
];
const INDEX_DSP_SKADLIST: &[Field] = &[
    Field {
        name: "max",
        kind: Kind::Integer,
    },
    Field {
        name: "excl",
        kind: Kind::Array(&Kind::Integer),
    },
    Field {
        name: "addl",
        kind: Kind::Array(&Kind::String),
    },
];
const INDEX_DSP_SITE: &[Field] = &[
    Field {
        name: "data",
        kind: Kind::Object(&[]),
    },
    Field {
        name: "inventorypartnerdomain",
        kind: Kind::String,
    },
];
const INDEX_DSP_APP: &[Field] = &[
    Field {
        name: "inventorypartnerdomain",
        kind: Kind::String,
    },
    Field {
        name: "data",
        kind: Kind::Object(&[]),
    },
];
const INDEX_DSP_CONTENT: &[Field] = &[
    Field {
        name: "channel",
        kind: Kind::String,
    },
    Field {
        name: "content_channel",
        kind: Kind::String,
    },
    Field {
        name: "network",
        kind: Kind::String,
    },
    Field {
        name: "content_network",
        kind: Kind::String,
    },
    Field {
        name: "distrib_name",
        kind: Kind::String,
    },
];
const INDEX_DSP_DEVICE: &[Field] = &[
    Field {
        name: "atts",
        kind: Kind::Integer,
    },
    Field {
        name: "cdep",
        kind: Kind::String,
    },
    Field {
        name: "ifv",
        kind: Kind::String,
    },
    Field {
        name: "ifa_type",
        kind: Kind::String,
    },
];
const INDEX_DSP_USER: &[Field] = &[
    Field {
        name: "consent",
        kind: Kind::String,
    },
    Field {
        name: "data",
        kind: Kind::Object(&[]),
    },
];
const INDEX_DSP_EID: &[Field] = &[Field {
    name: "rtipartner",
    kind: Kind::String,
}];
const INDEX_DSP_UID: &[Field] = &[Field {
    name: "rtipartner",
    kind: Kind::String,
}];
const INDEX_DSP_DATA: &[Field] = &[
    Field {
        name: "segclass",
        kind: Kind::String,
    },
    Field {
        name: "segtax",
        kind: Kind::Integer,
    },
];
fn index_dsp_schema(object: &str) -> &'static [Field] {
    match object {
        "BidRequest" => INDEX_DSP_BIDREQUEST,
        "Source" => INDEX_DSP_SOURCE,
        "Regs" => INDEX_DSP_REGS,
        "Imp" => INDEX_DSP_IMP,
        "Banner" => INDEX_DSP_BANNER,
        "Video" => INDEX_DSP_VIDEO,
        "Native" => INDEX_DSP_NATIVE,
        "Pmp" => INDEX_DSP_PMP,
        "Deal" => INDEX_DSP_DEAL,
        "Site" => INDEX_DSP_SITE,
        "App" => INDEX_DSP_APP,
        "Content" => INDEX_DSP_CONTENT,
        "Device" => INDEX_DSP_DEVICE,
        "User" => INDEX_DSP_USER,
        "EID" => INDEX_DSP_EID,
        "UID" => INDEX_DSP_UID,
        "Data" => INDEX_DSP_DATA,
        _ => &[],
    }
}
const INDEX_RESPONSE_BID: &[Field] = &[
    Field {
        name: "dsa",
        kind: Kind::Object(INDEX_RESPONSE_DSA),
    },
    Field {
        name: "skadn",
        kind: Kind::Object(INDEX_RESPONSE_SKAD),
    },
];
const INDEX_RESPONSE_DSA: &[Field] = &[
    Field {
        name: "adrender",
        kind: Kind::Integer,
    },
    Field {
        name: "behalf",
        kind: Kind::String,
    },
    Field {
        name: "paid",
        kind: Kind::String,
    },
    Field {
        name: "transparency",
        kind: Kind::Array(&Kind::Object(INDEX_RESPONSE_TRANSPARENCY)),
    },
];
const INDEX_RESPONSE_TRANSPARENCY: &[Field] = &[
    Field {
        name: "domain",
        kind: Kind::String,
    },
    Field {
        name: "dsaparams",
        kind: Kind::Array(&Kind::Integer),
    },
];
const INDEX_RESPONSE_SKAD: &[Field] = &[
    Field {
        name: "version",
        kind: Kind::String,
    },
    Field {
        name: "network",
        kind: Kind::String,
    },
    Field {
        name: "campaign",
        kind: Kind::String,
    },
    Field {
        name: "itunesitem",
        kind: Kind::String,
    },
    Field {
        name: "fidelities",
        kind: Kind::Array(&Kind::Object(INDEX_RESPONSE_FIDELITY)),
    },
    Field {
        name: "nonce",
        kind: Kind::String,
    },
    Field {
        name: "sourceapp",
        kind: Kind::String,
    },
    Field {
        name: "timestamp",
        kind: Kind::String,
    },
    Field {
        name: "signature",
        kind: Kind::String,
    },
];
const INDEX_RESPONSE_FIDELITY: &[Field] = &[
    Field {
        name: "fidelity",
        kind: Kind::Integer,
    },
    Field {
        name: "nonce",
        kind: Kind::String,
    },
    Field {
        name: "timestamp",
        kind: Kind::String,
    },
    Field {
        name: "signature",
        kind: Kind::String,
    },
    Field {
        name: "ext",
        kind: Kind::Object(&[]),
    },
];
const INDEX_RESPONSE_BIDRESPONSE: &[Field] = &[Field {
    name: "igbid",
    kind: Kind::Array(&Kind::Object(&[])),
}];
fn index_response_schema(object: &str) -> &'static [Field] {
    match object {
        "Bid" => INDEX_RESPONSE_BID,
        "BidResponse" => INDEX_RESPONSE_BIDRESPONSE,
        _ => &[],
    }
}
