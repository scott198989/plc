use plc_core::{Sha256Digest, Uuid, sha256};

#[derive(Clone, Debug, Default)]
pub(crate) struct CanonicalEncoder {
    bytes: Vec<u8>,
}

impl CanonicalEncoder {
    pub(crate) fn domain(&mut self, domain: &str) {
        self.text(domain);
    }

    pub(crate) fn tag(&mut self, tag: &str) {
        self.text(tag);
    }

    pub(crate) fn bool(&mut self, value: bool) {
        self.bytes.push(u8::from(value));
    }

    pub(crate) fn u8(&mut self, value: u8) {
        self.bytes.push(value);
    }

    pub(crate) fn u32(&mut self, value: u32) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    pub(crate) fn i32(&mut self, value: i32) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    pub(crate) fn u64(&mut self, value: u64) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    pub(crate) fn i64(&mut self, value: i64) {
        self.bytes.extend_from_slice(&value.to_be_bytes());
    }

    pub(crate) fn usize(&mut self, value: usize) {
        self.u64(u64::try_from(value).expect("canonical collection length fits u64"));
    }

    pub(crate) fn text(&mut self, value: &str) {
        self.usize(value.len());
        self.bytes.extend_from_slice(value.as_bytes());
    }

    pub(crate) fn uuid(&mut self, value: Uuid) {
        self.bytes.extend_from_slice(&value.into_bytes());
    }

    pub(crate) fn digest(&mut self, value: Sha256Digest) {
        self.bytes.extend_from_slice(&value.0);
    }

    pub(crate) fn option<T>(&mut self, value: Option<T>, encode: impl FnOnce(&mut Self, T)) {
        self.bool(value.is_some());
        if let Some(value) = value {
            encode(self, value);
        }
    }

    pub(crate) fn finish(self) -> Vec<u8> {
        self.bytes
    }

    pub(crate) fn fingerprint(self) -> Sha256Digest {
        sha256(&self.bytes)
    }
}
