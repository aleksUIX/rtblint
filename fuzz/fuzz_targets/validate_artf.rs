//! Fuzz target: validate arbitrary byte sequences as ARTF envelopes.
#![no_main]

use libfuzzer_sys::fuzz_target;
use rtblint_core::{validate_artf_request, OpenRtbVersion};

fuzz_target!(|data: &[u8]| {
    if let Ok(json) = std::str::from_utf8(data) {
        let _ = validate_artf_request(OpenRtbVersion::V2_6_202606, json);
    }
});
