#![no_main]

use libfuzzer_sys::fuzz_target;

#[path = "../../tests/support/mod.rs"]
mod support;

fuzz_target!(|data: &[u8]| {
    let operations: Vec<_> = data
        .chunks_exact(4)
        .take(256)
        .map(|bytes| (bytes[0], bytes[1], i16::from_le_bytes([bytes[2], bytes[3]])))
        .collect();
    support::check_operations(&operations);
});
