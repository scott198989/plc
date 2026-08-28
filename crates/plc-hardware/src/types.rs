#![allow(clippy::missing_errors_doc)]

use std::collections::BTreeSet;
use std::fmt;

use plc_core::{Sha256Digest, Uuid};

use crate::canonical::CanonicalEncoder;
use crate::ids::TypeDeclarationId;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FiniteF64(u64);

impl FiniteF64 {
    pub fn new(value: f64) -> Result<Self, TypeError> {
        if !value.is_finite() {
            return Err(TypeError::NonFinite);
        }
        Ok(Self(value.to_bits()))
    }

    #[must_use]
    pub fn get(self) -> f64 {
        f64::from_bits(self.0)
    }

    #[must_use]
    pub const fn bits(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CanonicalF32(u32);

impl CanonicalF32 {
    pub const QUIET_NAN_BITS: u32 = 0x7fc0_0000;

    #[must_use]
    pub fn new(value: f32) -> Self {
        if value.is_nan() {
            Self(Self::QUIET_NAN_BITS)
        } else {
            Self(value.to_bits())
        }
    }

    #[must_use]
    pub const fn bits(self) -> u32 {
        self.0
    }

    #[must_use]
    pub fn get(self) -> f32 {
        f32::from_bits(self.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CanonicalF64(u64);

impl CanonicalF64 {
    pub const QUIET_NAN_BITS: u64 = 0x7ff8_0000_0000_0000;

    #[must_use]
    pub fn new(value: f64) -> Self {
        if value.is_nan() {
            Self(Self::QUIET_NAN_BITS)
        } else {
            Self(value.to_bits())
        }
    }

    #[must_use]
    pub const fn bits(self) -> u64 {
        self.0
    }

    #[must_use]
    pub fn get(self) -> f64 {
        f64::from_bits(self.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PrimitiveType {
    Bool,
    Sint,
    Int,
    Dint,
    Lint,
    Usint,
    Uint,
    Udint,
    Ulint,
    Byte,
    Word,
    Dword,
    Lword,
    Real,
    Lreal,
    Char,
    String(u8),
    Time,
}

impl PrimitiveType {
    #[must_use]
    pub const fn stable_id(self) -> &'static str {
        match self {
            Self::Bool => "BOOL",
            Self::Sint => "SINT",
            Self::Int => "INT",
            Self::Dint => "DINT",
            Self::Lint => "LINT",
            Self::Usint => "USINT",
            Self::Uint => "UINT",
            Self::Udint => "UDINT",
            Self::Ulint => "ULINT",
            Self::Byte => "BYTE",
            Self::Word => "WORD",
            Self::Dword => "DWORD",
            Self::Lword => "LWORD",
            Self::Real => "REAL",
            Self::Lreal => "LREAL",
            Self::Char => "CHAR",
            Self::String(_) => "STRING",
            Self::Time => "TIME",
        }
    }

    #[must_use]
    pub const fn storage_width_bytes(self) -> Option<u8> {
        match self {
            Self::Bool | Self::String(_) => None,
            Self::Sint | Self::Usint | Self::Byte | Self::Char => Some(1),
            Self::Int | Self::Uint | Self::Word => Some(2),
            Self::Dint | Self::Udint | Self::Dword | Self::Real => Some(4),
            Self::Lint | Self::Ulint | Self::Lword | Self::Lreal | Self::Time => Some(8),
        }
    }

    #[must_use]
    pub const fn is_bit_string(self) -> bool {
        matches!(self, Self::Byte | Self::Word | Self::Dword | Self::Lword)
    }

    #[must_use]
    pub const fn is_signed_integer(self) -> bool {
        matches!(self, Self::Sint | Self::Int | Self::Dint | Self::Lint)
    }

    #[must_use]
    pub const fn is_unsigned_integer(self) -> bool {
        matches!(self, Self::Usint | Self::Uint | Self::Udint | Self::Ulint)
    }

    fn encode(self, encoder: &mut CanonicalEncoder) {
        encoder.text(self.stable_id());
        if let Self::String(capacity) = self {
            encoder.u8(capacity);
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum InstructionStateKind {
    Edge,
    Timer,
    Counter,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ArrayBound {
    pub lower: i32,
    pub upper: i32,
}

impl ArrayBound {
    pub fn element_count(self) -> Result<u64, TypeError> {
        if self.lower > self.upper {
            return Err(TypeError::InvalidArrayBound);
        }
        let count = i64::from(self.upper) - i64::from(self.lower) + 1;
        u64::try_from(count).map_err(|_| TypeError::InvalidArrayBound)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StructMember {
    pub id: Uuid,
    pub name: String,
    pub declared_order: u32,
    pub ty: CanonicalType,
    pub reusable_default: Option<PlcValue>,
    pub comment: String,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CanonicalType {
    Primitive(PrimitiveType),
    Array {
        dimensions: Vec<ArrayBound>,
        element_type: Box<Self>,
    },
    AnonymousStruct(Vec<StructMember>),
    Named {
        id: TypeDeclarationId,
        members: Vec<StructMember>,
    },
    InstructionState(InstructionStateKind),
}

impl CanonicalType {
    pub fn validate(
        &self,
        max_depth: u8,
        max_members: u32,
        max_dimensions: u8,
        max_elements: u64,
    ) -> Result<(), TypeError> {
        self.validate_at(1, max_depth, max_members, max_dimensions, max_elements)
    }

    fn validate_at(
        &self,
        depth: u8,
        max_depth: u8,
        max_members: u32,
        max_dimensions: u8,
        max_elements: u64,
    ) -> Result<(), TypeError> {
        if depth > max_depth {
            return Err(TypeError::NestingLimit);
        }
        match self {
            Self::Primitive(PrimitiveType::String(255)) => Err(TypeError::StringCapacity),
            Self::Primitive(_) | Self::InstructionState(_) => Ok(()),
            Self::Array {
                dimensions,
                element_type,
            } => {
                if dimensions.is_empty() || dimensions.len() > usize::from(max_dimensions) {
                    return Err(TypeError::InvalidDimensionCount);
                }
                let mut count = 1_u64;
                for bound in dimensions {
                    count = count
                        .checked_mul(bound.element_count()?)
                        .ok_or(TypeError::ElementLimit)?;
                    if count > max_elements {
                        return Err(TypeError::ElementLimit);
                    }
                }
                element_type.validate_at(
                    depth.saturating_add(1),
                    max_depth,
                    max_members,
                    max_dimensions,
                    max_elements,
                )
            }
            Self::AnonymousStruct(members) | Self::Named { members, .. } => {
                if let Self::Named { id, .. } = self
                    && !id.uuid().is_rfc9562_v4()
                {
                    return Err(TypeError::InvalidIdentity);
                }
                if u32::try_from(members.len()).map_or(true, |count| count > max_members) {
                    return Err(TypeError::MemberLimit);
                }
                let mut names = BTreeSet::new();
                let mut ids = BTreeSet::new();
                let mut orders = BTreeSet::new();
                for member in members {
                    if !member.id.is_rfc9562_v4() {
                        return Err(TypeError::InvalidIdentity);
                    }
                    validate_member_name(&member.name)?;
                    if !names.insert(member.name.to_ascii_lowercase()) {
                        return Err(TypeError::DuplicateMemberName);
                    }
                    if !ids.insert(member.id) || !orders.insert(member.declared_order) {
                        return Err(TypeError::DuplicateMemberIdentityOrOrder);
                    }
                    member.ty.validate_at(
                        depth.saturating_add(1),
                        max_depth,
                        max_members,
                        max_dimensions,
                        max_elements,
                    )?;
                    if let Some(value) = &member.reusable_default {
                        member.ty.validate_value(value)?;
                    }
                    if matches!(member.ty, Self::InstructionState(_))
                        && member.reusable_default.is_some()
                    {
                        return Err(TypeError::StateDefaultIsFixed);
                    }
                }
                Ok(())
            }
        }
    }

    #[must_use]
    pub fn fingerprint(&self) -> Sha256Digest {
        let mut encoder = CanonicalEncoder::default();
        encoder.domain("EDU21-TYPE-V1");
        self.encode(&mut encoder, false);
        encoder.fingerprint()
    }

    #[must_use]
    pub fn anonymous_signature(&self) -> Option<Sha256Digest> {
        if !matches!(self, Self::AnonymousStruct(_)) {
            return None;
        }
        let mut encoder = CanonicalEncoder::default();
        encoder.domain("EDU21-ANON-STRUCT-SIGNATURE-V1");
        self.encode(&mut encoder, true);
        Some(encoder.fingerprint())
    }

    pub fn validate_value(&self, value: &PlcValue) -> Result<(), TypeError> {
        match (self, value) {
            (Self::Primitive(primitive), value) => validate_primitive_value(*primitive, value),
            (
                Self::Array {
                    dimensions,
                    element_type,
                },
                PlcValue::Array(values),
            ) => {
                let expected = dimensions.iter().try_fold(1_u64, |count, bound| {
                    count
                        .checked_mul(bound.element_count()?)
                        .ok_or(TypeError::ElementLimit)
                })?;
                if u64::try_from(values.len()).ok() != Some(expected) {
                    return Err(TypeError::ValueShapeMismatch);
                }
                values
                    .iter()
                    .try_for_each(|value| element_type.validate_value(value))
            }
            (
                Self::AnonymousStruct(members) | Self::Named { members, .. },
                PlcValue::Struct(values),
            ) => {
                if members.len() != values.len() {
                    return Err(TypeError::ValueShapeMismatch);
                }
                members
                    .iter()
                    .zip(values)
                    .try_for_each(|(member, (id, value))| {
                        if member.id != *id {
                            return Err(TypeError::ValueShapeMismatch);
                        }
                        member.ty.validate_value(value)
                    })
            }
            (Self::InstructionState(expected), PlcValue::InstructionState(actual))
                if expected == actual =>
            {
                Ok(())
            }
            _ => Err(TypeError::ValueTypeMismatch),
        }
    }

    #[must_use]
    pub fn canonical_default(&self) -> PlcValue {
        match self {
            Self::Primitive(primitive) => primitive_default(*primitive),
            Self::Array {
                dimensions,
                element_type,
            } => {
                let count = dimensions.iter().fold(1_usize, |count, bound| {
                    let dimension =
                        usize::try_from(bound.element_count().unwrap_or(0)).unwrap_or(0);
                    count.saturating_mul(dimension)
                });
                PlcValue::Array(vec![element_type.canonical_default(); count])
            }
            Self::AnonymousStruct(members) | Self::Named { members, .. } => PlcValue::Struct(
                members
                    .iter()
                    .map(|member| {
                        (
                            member.id,
                            member
                                .reusable_default
                                .clone()
                                .unwrap_or_else(|| member.ty.canonical_default()),
                        )
                    })
                    .collect(),
            ),
            Self::InstructionState(kind) => PlcValue::InstructionState(*kind),
        }
    }

    pub(crate) fn encode(&self, encoder: &mut CanonicalEncoder, signature_only: bool) {
        match self {
            Self::Primitive(primitive) => {
                encoder.tag("primitive");
                primitive.encode(encoder);
            }
            Self::Array {
                dimensions,
                element_type,
            } => {
                encoder.tag("array");
                encoder.usize(dimensions.len());
                for dimension in dimensions {
                    encoder.i32(dimension.lower);
                    encoder.i32(dimension.upper);
                }
                element_type.encode(encoder, signature_only);
            }
            Self::AnonymousStruct(members) => {
                encoder.tag("anonymous-struct");
                encode_members(members, encoder, signature_only);
            }
            Self::Named { id, members } => {
                encoder.tag("named");
                encoder.uuid(id.uuid());
                encode_members(members, encoder, signature_only);
            }
            Self::InstructionState(kind) => {
                encoder.tag(match kind {
                    InstructionStateKind::Edge => "state-edge",
                    InstructionStateKind::Timer => "state-timer",
                    InstructionStateKind::Counter => "state-counter",
                });
            }
        }
    }
}

fn encode_members(members: &[StructMember], encoder: &mut CanonicalEncoder, signature_only: bool) {
    encoder.usize(members.len());
    let mut ordered: Vec<_> = members.iter().collect();
    ordered.sort_by_key(|member| (member.declared_order, member.id));
    for member in ordered {
        if !signature_only {
            encoder.uuid(member.id);
        }
        encoder.text(&member.name.to_ascii_lowercase());
        member.ty.encode(encoder, signature_only);
    }
}

fn validate_member_name(name: &str) -> Result<(), TypeError> {
    if name.is_empty()
        || name.len() > 128
        || !name.is_ascii()
        || !name
            .bytes()
            .next()
            .is_some_and(|byte| byte.is_ascii_alphabetic() || byte == b'_')
        || !name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
    {
        return Err(TypeError::InvalidMemberName);
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PlcValue {
    Bool(bool),
    Signed(i64),
    Unsigned(u64),
    BitString(u64),
    Real(CanonicalF32),
    Lreal(CanonicalF64),
    Char(u8),
    String(Vec<u8>),
    Time(i64),
    Array(Vec<Self>),
    Struct(Vec<(Uuid, Self)>),
    InstructionState(InstructionStateKind),
}

impl PlcValue {
    pub(crate) fn encode(&self, encoder: &mut CanonicalEncoder) {
        match self {
            Self::Bool(value) => {
                encoder.tag("bool");
                encoder.bool(*value);
            }
            Self::Signed(value) => {
                encoder.tag("signed");
                encoder.i64(*value);
            }
            Self::Unsigned(value) => {
                encoder.tag("unsigned");
                encoder.u64(*value);
            }
            Self::BitString(value) => {
                encoder.tag("bits");
                encoder.u64(*value);
            }
            Self::Real(value) => {
                encoder.tag("real");
                encoder.u32(value.bits());
            }
            Self::Lreal(value) => {
                encoder.tag("lreal");
                encoder.u64(value.bits());
            }
            Self::Char(value) => {
                encoder.tag("char");
                encoder.u8(*value);
            }
            Self::String(value) => {
                encoder.tag("string");
                encoder.usize(value.len());
                for byte in value {
                    encoder.u8(*byte);
                }
            }
            Self::Time(value) => {
                encoder.tag("time");
                encoder.i64(*value);
            }
            Self::Array(values) => {
                encoder.tag("array-value");
                encoder.usize(values.len());
                for value in values {
                    value.encode(encoder);
                }
            }
            Self::Struct(values) => {
                encoder.tag("struct-value");
                encoder.usize(values.len());
                for (id, value) in values {
                    encoder.uuid(*id);
                    value.encode(encoder);
                }
            }
            Self::InstructionState(kind) => {
                encoder.tag(match kind {
                    InstructionStateKind::Edge => "state-edge-default",
                    InstructionStateKind::Timer => "state-timer-default",
                    InstructionStateKind::Counter => "state-counter-default",
                });
            }
        }
    }
}

fn validate_primitive_value(primitive: PrimitiveType, value: &PlcValue) -> Result<(), TypeError> {
    let valid = match (primitive, value) {
        (PrimitiveType::Bool, PlcValue::Bool(_))
        | (PrimitiveType::Real, PlcValue::Real(_))
        | (PrimitiveType::Lreal, PlcValue::Lreal(_))
        | (PrimitiveType::Char, PlcValue::Char(_))
        | (PrimitiveType::Time, PlcValue::Time(_))
        | (PrimitiveType::Lint, PlcValue::Signed(_))
        | (PrimitiveType::Ulint, PlcValue::Unsigned(_))
        | (PrimitiveType::Lword, PlcValue::BitString(_)) => true,
        (PrimitiveType::Sint, PlcValue::Signed(value)) => i8::try_from(*value).is_ok(),
        (PrimitiveType::Int, PlcValue::Signed(value)) => i16::try_from(*value).is_ok(),
        (PrimitiveType::Dint, PlcValue::Signed(value)) => i32::try_from(*value).is_ok(),
        (PrimitiveType::Usint, PlcValue::Unsigned(value))
        | (PrimitiveType::Byte, PlcValue::BitString(value)) => u8::try_from(*value).is_ok(),
        (PrimitiveType::Uint, PlcValue::Unsigned(value))
        | (PrimitiveType::Word, PlcValue::BitString(value)) => u16::try_from(*value).is_ok(),
        (PrimitiveType::Udint, PlcValue::Unsigned(value))
        | (PrimitiveType::Dword, PlcValue::BitString(value)) => u32::try_from(*value).is_ok(),
        (PrimitiveType::String(capacity), PlcValue::String(value)) => {
            value.len() <= usize::from(capacity)
        }
        _ => false,
    };
    if valid {
        Ok(())
    } else {
        Err(TypeError::ValueTypeMismatch)
    }
}

fn primitive_default(primitive: PrimitiveType) -> PlcValue {
    match primitive {
        PrimitiveType::Bool => PlcValue::Bool(false),
        PrimitiveType::Sint | PrimitiveType::Int | PrimitiveType::Dint | PrimitiveType::Lint => {
            PlcValue::Signed(0)
        }
        PrimitiveType::Usint
        | PrimitiveType::Uint
        | PrimitiveType::Udint
        | PrimitiveType::Ulint => PlcValue::Unsigned(0),
        PrimitiveType::Byte | PrimitiveType::Word | PrimitiveType::Dword | PrimitiveType::Lword => {
            PlcValue::BitString(0)
        }
        PrimitiveType::Real => PlcValue::Real(CanonicalF32::new(0.0)),
        PrimitiveType::Lreal => PlcValue::Lreal(CanonicalF64::new(0.0)),
        PrimitiveType::Char => PlcValue::Char(0),
        PrimitiveType::String(_) => PlcValue::String(Vec::new()),
        PrimitiveType::Time => PlcValue::Time(0),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RetainPolicy {
    NonRetentive,
    Retentive,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TypeError {
    NonFinite,
    InvalidIdentity,
    StringCapacity,
    InvalidArrayBound,
    InvalidDimensionCount,
    NestingLimit,
    MemberLimit,
    ElementLimit,
    InvalidMemberName,
    DuplicateMemberName,
    DuplicateMemberIdentityOrOrder,
    StateDefaultIsFixed,
    ValueTypeMismatch,
    ValueShapeMismatch,
}

impl fmt::Display for TypeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for TypeError {}

#[cfg(test)]
mod tests {
    use plc_core::Uuid;

    use crate::ids::TypeDeclarationId;

    use super::{
        ArrayBound, CanonicalF32, CanonicalF64, CanonicalType, PlcValue, PrimitiveType,
        StructMember, TypeError,
    };

    #[test]
    fn floating_nan_is_canonical_and_signed_zero_is_preserved() {
        assert_eq!(
            CanonicalF32::new(f32::from_bits(0x7fc0_0123)).bits(),
            CanonicalF32::QUIET_NAN_BITS
        );
        assert_eq!(
            CanonicalF64::new(f64::from_bits(0x7ff8_1234_5678_0000)).bits(),
            CanonicalF64::QUIET_NAN_BITS
        );
        assert_ne!(
            CanonicalF64::new(-0.0).bits(),
            CanonicalF64::new(0.0).bits()
        );
    }

    #[test]
    fn primitive_boundaries_and_string_capacity_are_exact() {
        let sint = CanonicalType::Primitive(PrimitiveType::Sint);
        assert!(sint.validate_value(&PlcValue::Signed(-128)).is_ok());
        assert!(sint.validate_value(&PlcValue::Signed(127)).is_ok());
        assert!(sint.validate_value(&PlcValue::Signed(128)).is_err());

        let string = CanonicalType::Primitive(PrimitiveType::String(2));
        assert!(string.validate_value(&PlcValue::String(vec![1, 2])).is_ok());
        assert!(
            string
                .validate_value(&PlcValue::String(vec![1, 2, 3]))
                .is_err()
        );
        assert_eq!(
            CanonicalType::Primitive(PrimitiveType::String(255)).validate(32, 4_096, 6, 1_000_000),
            Err(TypeError::StringCapacity)
        );

        let too_many_dimensions = CanonicalType::Array {
            dimensions: vec![
                ArrayBound { lower: 0, upper: 0 },
                ArrayBound { lower: 0, upper: 0 },
            ],
            element_type: Box::new(CanonicalType::Primitive(PrimitiveType::Bool)),
        };
        assert_eq!(
            too_many_dimensions.validate(32, 4_096, 1, 1_000_000),
            Err(TypeError::InvalidDimensionCount)
        );

        let invalid_member_identity = CanonicalType::Named {
            id: TypeDeclarationId(Uuid::deterministic_v4(b"type", 1)),
            members: vec![StructMember {
                id: Uuid::NIL,
                name: "Member".to_owned(),
                declared_order: 0,
                ty: CanonicalType::Primitive(PrimitiveType::Bool),
                reusable_default: None,
                comment: String::new(),
            }],
        };
        assert_eq!(
            invalid_member_identity.validate(32, 4_096, 6, 1_000_000),
            Err(TypeError::InvalidIdentity)
        );
    }
}
