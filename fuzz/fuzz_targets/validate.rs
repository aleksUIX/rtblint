//! Fuzz target: validate arbitrary byte sequences as bid requests.
//!
//! The validator must never panic regardless of input. It should always
//! return a structured result.
#![no_main]

use libfuzzer_sys::fuzz_target;
use rtblint_core::validate;

fuzz_target!(|data: &[u8]| {
    if let Ok(json) = std::str::from_utf8(data) {
        let _ = validate(json);
    }
});
