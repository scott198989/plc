use plc_runtime::{Hash32, Sha256};

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

    pub(crate) fn u8(&mut self, value: u8) {
        self.inner.update(&[value]);
    }

    pub(crate) fn bool(&mut self, value: bool) {
        self.u8(u8::from(value));
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
}

pub(crate) fn id_from_hash(hash: Hash32) -> u128 {
    let mut id = [0_u8; 16];
    id.copy_from_slice(&hash.as_bytes()[..16]);
    u128::from_be_bytes(id)
}

pub(crate) fn encode_value(value: plc_runtime::CanonicalValue, hasher: &mut CanonicalHasher) {
    match value {
        plc_runtime::CanonicalValue::Bool(value) => {
            hasher.u8(1);
            hasher.bool(value);
        }
        plc_runtime::CanonicalValue::I32(value) => {
            hasher.u8(2);
            hasher.i32(value);
        }
        plc_runtime::CanonicalValue::I64(value) => {
            hasher.u8(3);
            hasher.bytes(&value.to_be_bytes());
        }
        plc_runtime::CanonicalValue::U32(value) => {
            hasher.u8(4);
            hasher.u32(value);
        }
        plc_runtime::CanonicalValue::TimeMs(value) => {
            hasher.u8(5);
            hasher.bytes(&value.to_be_bytes());
        }
        plc_runtime::CanonicalValue::I8(value) => {
            hasher.u8(6);
            hasher.bytes(&value.to_be_bytes());
        }
        plc_runtime::CanonicalValue::I16(value) => {
            hasher.u8(7);
            hasher.bytes(&value.to_be_bytes());
        }
        plc_runtime::CanonicalValue::U8(value) => {
            hasher.u8(8);
            hasher.u8(value);
        }
        plc_runtime::CanonicalValue::U16(value) => {
            hasher.u8(9);
            hasher.bytes(&value.to_be_bytes());
        }
        plc_runtime::CanonicalValue::U64(value) => {
            hasher.u8(10);
            hasher.u64(value);
        }
        plc_runtime::CanonicalValue::Bits8(value) => {
            hasher.u8(11);
            hasher.u8(value);
        }
        plc_runtime::CanonicalValue::Bits16(value) => {
            hasher.u8(12);
            hasher.bytes(&value.to_be_bytes());
        }
        plc_runtime::CanonicalValue::Bits32(value) => {
            hasher.u8(13);
            hasher.u32(value);
        }
        plc_runtime::CanonicalValue::Bits64(value) => {
            hasher.u8(14);
            hasher.u64(value);
        }
        plc_runtime::CanonicalValue::F32(value) => {
            hasher.u8(15);
            hasher.u32(value.bits());
        }
        plc_runtime::CanonicalValue::F64(value) => {
            hasher.u8(16);
            hasher.u64(value.bits());
        }
        plc_runtime::CanonicalValue::Char(value) => {
            hasher.u8(17);
            hasher.u8(value);
        }
    }
}
