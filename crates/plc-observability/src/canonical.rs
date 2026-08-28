use plc_runtime::{CanonicalValue, Hash32, Sha256};

pub(crate) struct CanonicalHasher {
    inner: Sha256,
}

impl CanonicalHasher {
    pub(crate) fn new(domain: &str) -> Self {
        let mut value = Self {
            inner: Sha256::new(),
        };
        value.bytes(domain.as_bytes());
        value
    }

    pub(crate) fn finish(self) -> Hash32 {
        self.inner.finalize()
    }

    pub(crate) fn u8(&mut self, value: u8) {
        self.inner.update(&[value]);
    }

    pub(crate) fn bool(&mut self, value: bool) {
        self.u8(u8::from(value));
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

    pub(crate) fn hash(&mut self, value: Hash32) {
        self.inner.update(value.as_bytes());
    }

    pub(crate) fn bytes(&mut self, value: &[u8]) {
        self.u64(value.len() as u64);
        self.inner.update(value);
    }

    pub(crate) fn string(&mut self, value: &str) {
        self.bytes(value.as_bytes());
    }

    pub(crate) fn value(&mut self, value: CanonicalValue) {
        self.u8(value.value_type() as u8);
        match value {
            CanonicalValue::Bool(value) => self.bool(value),
            CanonicalValue::I32(value) => self.i32(value),
            CanonicalValue::I64(value) => self.i64(value),
            CanonicalValue::U32(value) => self.u32(value),
            CanonicalValue::TimeMs(value) => self.u64(value),
        }
    }
}

pub(crate) fn id128(hash: Hash32) -> u128 {
    u128::from_be_bytes(
        hash.as_bytes()[..16]
            .try_into()
            .expect("a SHA-256 digest always contains sixteen identity bytes"),
    )
}
