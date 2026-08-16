//! Fuzz target: validate arbitrary byte sequences as bid responses.
#![no_main]

use libfuzzer_sys::fuzz_target;
use rtblint_core::{validate_bid_response_for_version, OpenRtbVersion};

fuzz_target!(|data: &[u8]| {
    if let Ok(json) = std::str::from_utf8(data) {
        let _ = validate_bid_response_for_version(OpenRtbVersion::V2_6_202606, json);
    }
});
