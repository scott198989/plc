use plc_runtime::{Hash32, Sha256};

/// Length-delimited canonical encoding with a distinct domain prefix.
pub(crate) struct CanonicalHasher {
    inner: Sha256,
}

impl CanonicalHasher {
    pub(crate) fn new(domain: &str) -> Self {
        let mut value = Self {
            inner: Sha256::new(),
        };
        value.string(domain);
        value
    }

    pub(crate) fn finish(self) -> Hash32 {
        self.inner.finalize()
    }

    pub(crate) fn bool(&mut self, value: bool) {
        self.u8(u8::from(value));
    }

    pub(crate) fn u8(&mut self, value: u8) {
        self.inner.update(&[value]);
    }

    pub(crate) fn u16(&mut self, value: u16) {
        self.inner.update(&value.to_be_bytes());
    }

    pub(crate) fn u32(&mut self, value: u32) {
        self.inner.update(&value.to_be_bytes());
    }

    pub(crate) fn i32(&mut self, value: i32) {
        self.inner.update(&value.to_be_bytes());
    }

    pub(crate) fn u64(&mut self, value: u64) {
        self.inner.update(&value.to_be_bytes());
    }

    pub(crate) fn i64(&mut self, value: i64) {
        self.inner.update(&value.to_be_bytes());
    }

    pub(crate) fn u128(&mut self, value: u128) {
        self.inner.update(&value.to_be_bytes());
    }

    pub(crate) fn bytes(&mut self, value: &[u8]) {
        self.u64(value.len() as u64);
        self.inner.update(value);
    }

    pub(crate) fn string(&mut self, value: &str) {
        self.bytes(value.as_bytes());
    }

    pub(crate) fn hash(&mut self, value: Hash32) {
        self.inner.update(value.as_bytes());
    }
}

pub(crate) fn hash_bytes(domain: &str, value: &[u8]) -> Hash32 {
    let mut hasher = CanonicalHasher::new(domain);
    hasher.bytes(value);
    hasher.finish()
}
