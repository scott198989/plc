#[cfg(any(target_arch = "wasm32", test))]
mod kernel_bridge;
#[cfg(any(target_arch = "wasm32", test))]
mod system_bridge;

const HEALTH_PAYLOAD: &[u8] =
    br#"{"schemaVersion":1,"buildIdentity":"plc-engineering-core@0.2.0","healthState":"HEALTHY"}"#;

#[must_use]
pub const fn health_payload() -> &'static [u8] {
    HEALTH_PAYLOAD
}

#[cfg(target_arch = "wasm32")]
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn foundation_health() -> u32 {
    HEALTH_PAYLOAD.as_ptr() as u32
}

#[cfg(target_arch = "wasm32")]
#[allow(unsafe_code)]
#[unsafe(no_mangle)]
pub extern "C" fn foundation_health_len() -> u32 {
    HEALTH_PAYLOAD.len() as u32
}

#[cfg(test)]
mod tests {
    use super::health_payload;

    #[test]
    fn engineering_health_payload_is_exact_and_deterministic() {
        const EXPECTED: &[u8] = br#"{"schemaVersion":1,"buildIdentity":"plc-engineering-core@0.2.0","healthState":"HEALTHY"}"#;
        assert_eq!(health_payload(), EXPECTED);
        assert_eq!(health_payload(), health_payload());
    }
}
