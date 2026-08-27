#![cfg_attr(target_arch = "wasm32", no_std)]

const HEALTH_PAYLOAD: &[u8] =
    br#"{"schemaVersion":1,"buildIdentity":"foundation-core@0.1.0","healthState":"HEALTHY"}"#;

/// Returns the immutable health payload used by both native tests and the WASM exports.
#[must_use]
pub const fn health_payload() -> &'static [u8] {
    HEALTH_PAYLOAD
}

/// Returns a pointer into the module's linear memory for the deterministic health payload.
#[cfg(target_arch = "wasm32")]
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn foundation_health() -> u32 {
    HEALTH_PAYLOAD.as_ptr() as u32
}

/// Returns the byte length of the deterministic health payload.
#[cfg(target_arch = "wasm32")]
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn foundation_health_len() -> u32 {
    HEALTH_PAYLOAD.len() as u32
}

#[cfg(target_arch = "wasm32")]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo<'_>) -> ! {
    loop {
        core::hint::spin_loop();
    }
}

#[cfg(test)]
mod tests {
    use super::health_payload;

    #[test]
    fn health_payload_is_exact_and_deterministic() {
        const EXPECTED: &[u8] = br#"{"schemaVersion":1,"buildIdentity":"foundation-core@0.1.0","healthState":"HEALTHY"}"#;
        assert_eq!(health_payload(), EXPECTED);
        assert_eq!(health_payload(), health_payload());
    }

    #[test]
    fn health_payload_contains_no_capability_descriptor() {
        let payload = core::str::from_utf8(health_payload()).expect("payload is UTF-8");
        for forbidden in ["endpoint", "transport", "socket", "device", "protocol"] {
            assert!(!payload.contains(forbidden));
        }
    }
}
