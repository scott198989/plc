use alloc::{
    boxed::Box,
    collections::{BTreeMap, BTreeSet},
    string::String,
    vec::Vec,
};
use core::fmt;

use crate::{CanonicalF32, CanonicalF64, PrimitiveType, ScalarTypeError, ScalarValue, TypedScalar};

const VALUE_MAGIC: &[u8; 16] = b"PES-TYP-VALUE-1\0";
const TYPE_MAGIC: &[u8; 16] = b"PES-TYP-TYPE-1\0\0";
const SIGNATURE_MAGIC: &[u8; 16] = b"PES-TYP-SIGN-1\0\0";
const MAX_STANDARD_ARRAY_DIMENSIONS: u8 = 6;
const MAX_MEMBER_NAME_BYTES: usize = 128;
const MAX_STANDARD_TYPE_DEPTH: u8 = 32;
const MAX_STANDARD_MEMBERS_PER_STRUCT: u32 = 4_096;
const MAX_STANDARD_VALUE_LEAVES: u64 = 1_000_000;
const MAX_STANDARD_SERIALIZED_BYTES: usize = 16 * 1_024 * 1_024;

/// Bounded resource policy for recursive PLC types and their canonical codec.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AggregateLimits {
    pub max_depth: u8,
    pub max_members_per_struct: u32,
    pub max_dimensions: u8,
    pub max_array_elements: u64,
    pub max_serialized_bytes: usize,
}

impl AggregateLimits {
    #[must_use]
    pub const fn edu21() -> Self {
        Self {
            max_depth: MAX_STANDARD_TYPE_DEPTH,
            max_members_per_struct: MAX_STANDARD_MEMBERS_PER_STRUCT,
            max_dimensions: MAX_STANDARD_ARRAY_DIMENSIONS,
            max_array_elements: MAX_STANDARD_VALUE_LEAVES,
            max_serialized_bytes: MAX_STANDARD_SERIALIZED_BYTES,
        }
    }

    fn validate(self) -> Result<(), TypeError> {
        if self.max_depth == 0
            || self.max_depth > MAX_STANDARD_TYPE_DEPTH
            || self.max_members_per_struct == 0
            || self.max_members_per_struct > MAX_STANDARD_MEMBERS_PER_STRUCT
            || self.max_dimensions == 0
            || self.max_dimensions > MAX_STANDARD_ARRAY_DIMENSIONS
            || self.max_array_elements == 0
            || self.max_array_elements > MAX_STANDARD_VALUE_LEAVES
            || self.max_serialized_bytes == 0
            || self.max_serialized_bytes > MAX_STANDARD_SERIALIZED_BYTES
        {
            return Err(TypeError::InvalidLimits);
        }
        Ok(())
    }
}

impl Default for AggregateLimits {
    fn default() -> Self {
        Self::edu21()
    }
}

/// Stable RFC 9562 `UUIDv4` bytes used for named types and structure members.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StableUuid([u8; 16]);

impl StableUuid {
    /// Admits only an RFC 9562 `UUIDv4` representation.
    ///
    /// # Errors
    ///
    /// Returns [`TypeError::InvalidIdentity`] for a non-v4 version or variant.
    pub const fn from_bytes(bytes: [u8; 16]) -> Result<Self, TypeError> {
        if bytes[6] & 0xf0 != 0x40 || bytes[8] & 0xc0 != 0x80 {
            return Err(TypeError::InvalidIdentity);
        }
        Ok(Self(bytes))
    }

    #[must_use]
    pub const fn as_bytes(self) -> [u8; 16] {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ArrayBound {
    pub lower: i32,
    pub upper: i32,
}

impl ArrayBound {
    /// Returns the inclusive element count without host-width arithmetic.
    ///
    /// # Errors
    ///
    /// Returns [`TypeError::InvalidArrayBound`] when `lower > upper`.
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
    pub id: StableUuid,
    pub name: String,
    pub declared_order: u32,
    pub data_type: CanonicalType,
    pub reusable_default: Option<PlcValue>,
    pub comment: String,
}

/// One capability-free recursive PLC type authority for scalars and aggregates.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CanonicalType {
    Primitive(PrimitiveType),
    Array {
        dimensions: Vec<ArrayBound>,
        element_type: Box<Self>,
    },
    AnonymousStruct {
        members: Vec<StructMember>,
    },
    NamedStruct {
        id: StableUuid,
        members: Vec<StructMember>,
    },
}

impl CanonicalType {
    /// Validates the complete recursive shape against explicit profile limits.
    ///
    /// # Errors
    ///
    /// Fails closed for invalid bounds, dimensions, identities, names, defaults,
    /// duplicate members/orders, excessive depth, or resource-cap overflow.
    pub fn validate(&self, limits: AggregateLimits) -> Result<(), TypeError> {
        limits.validate()?;
        self.validate_at(1, limits)?;
        if self.value_leaf_count(limits)? > limits.max_array_elements {
            return Err(TypeError::ElementLimit);
        }
        Ok(())
    }

    fn value_leaf_count(&self, limits: AggregateLimits) -> Result<u64, TypeError> {
        match self {
            Self::Primitive(_) => Ok(1),
            Self::Array {
                dimensions,
                element_type,
            } => array_element_count(dimensions, limits)?
                .checked_mul(element_type.value_leaf_count(limits)?)
                .ok_or(TypeError::ElementLimit),
            Self::AnonymousStruct { members } | Self::NamedStruct { members, .. } => {
                members.iter().try_fold(0_u64, |count, member| {
                    count
                        .checked_add(member.data_type.value_leaf_count(limits)?)
                        .ok_or(TypeError::ElementLimit)
                })
            }
        }
    }

    fn validate_at(&self, depth: u8, limits: AggregateLimits) -> Result<(), TypeError> {
        if depth > limits.max_depth {
            return Err(TypeError::NestingLimit);
        }
        match self {
            Self::Primitive(primitive) => {
                if primitive.declaration_is_valid() {
                    Ok(())
                } else {
                    Err(TypeError::InvalidStringCapacity)
                }
            }
            Self::Array {
                dimensions,
                element_type,
            } => {
                array_element_count(dimensions, limits)?;
                element_type.validate_at(depth.saturating_add(1), limits)
            }
            Self::AnonymousStruct { members } | Self::NamedStruct { members, .. } => {
                if let Self::NamedStruct { id, .. } = self {
                    StableUuid::from_bytes(id.as_bytes())?;
                }
                if u32::try_from(members.len())
                    .map_or(true, |count| count > limits.max_members_per_struct)
                {
                    return Err(TypeError::MemberLimit);
                }
                let mut ids = BTreeSet::new();
                let mut names = BTreeSet::new();
                let mut orders = BTreeSet::new();
                for member in members {
                    StableUuid::from_bytes(member.id.as_bytes())?;
                    validate_member_name(&member.name)?;
                    if !ids.insert(member.id) {
                        return Err(TypeError::DuplicateMemberIdentity);
                    }
                    if !names.insert(member.name.to_ascii_lowercase()) {
                        return Err(TypeError::DuplicateMemberName);
                    }
                    if !orders.insert(member.declared_order) {
                        return Err(TypeError::DuplicateDeclaredOrder);
                    }
                    member
                        .data_type
                        .validate_at(depth.saturating_add(1), limits)?;
                    if let Some(default) = &member.reusable_default {
                        member
                            .data_type
                            .validate_value(default, limits)
                            .map_err(|_| TypeError::DefaultValueMismatch)?;
                    }
                }
                Ok(())
            }
        }
    }

    /// Validates an exact scalar/array/structure value without conversions.
    ///
    /// # Errors
    ///
    /// Returns a type or shape error for any leaf, bound, identity, or count
    /// mismatch.
    pub fn validate_value(
        &self,
        value: &PlcValue,
        limits: AggregateLimits,
    ) -> Result<(), TypeError> {
        self.validate(limits)?;
        self.validate_value_at(value, limits)
    }

    fn validate_value_at(
        &self,
        value: &PlcValue,
        limits: AggregateLimits,
    ) -> Result<(), TypeError> {
        match (self, value) {
            (Self::Primitive(expected), PlcValue::Scalar(value)) => {
                if value.data_type() != *expected {
                    return Err(TypeError::ValueTypeMismatch);
                }
                expected
                    .validate_scalar(value.value())
                    .map_err(TypeError::Scalar)
            }
            (
                Self::Array {
                    dimensions,
                    element_type,
                },
                PlcValue::Array(values),
            ) => {
                let expected = array_element_count(dimensions, limits)?;
                if u64::try_from(values.len()).ok() != Some(expected) {
                    return Err(TypeError::ValueShapeMismatch);
                }
                values
                    .iter()
                    .try_for_each(|value| element_type.validate_value_at(value, limits))
            }
            (
                Self::AnonymousStruct { members } | Self::NamedStruct { members, .. },
                PlcValue::Struct(fields),
            ) => validate_struct_value(members, fields, limits),
            _ => Err(TypeError::ValueTypeMismatch),
        }
    }

    /// Recursively creates the exact canonical default after validating limits.
    ///
    /// # Errors
    ///
    /// Returns an error instead of manufacturing a partial default for an
    /// invalid or over-capacity type.
    pub fn canonical_default(&self, limits: AggregateLimits) -> Result<PlcValue, TypeError> {
        self.validate(limits)?;
        self.canonical_default_at(limits)
    }

    fn canonical_default_at(&self, limits: AggregateLimits) -> Result<PlcValue, TypeError> {
        match self {
            Self::Primitive(primitive) => {
                Ok(PlcValue::Scalar(TypedScalar::canonical_default(*primitive)))
            }
            Self::Array {
                dimensions,
                element_type,
            } => {
                let count = usize::try_from(array_element_count(dimensions, limits)?)
                    .map_err(|_| TypeError::ElementLimit)?;
                let default = element_type.canonical_default_at(limits)?;
                Ok(PlcValue::Array(alloc::vec![default; count]))
            }
            Self::AnonymousStruct { members } | Self::NamedStruct { members, .. } => {
                let mut fields = Vec::with_capacity(members.len());
                for member in ordered_members(members) {
                    fields.push(StructFieldValue {
                        member_id: member.id,
                        value: member
                            .reusable_default
                            .clone()
                            .map_or_else(|| member.data_type.canonical_default_at(limits), Ok)?,
                    });
                }
                Ok(PlcValue::Struct(fields))
            }
        }
    }

    /// Returns canonical type bytes in fixed big-endian order.
    ///
    /// Comments are intentionally nonsemantic. Reusable defaults are semantic
    /// and therefore included. Member names are ASCII case-folded.
    ///
    /// # Errors
    ///
    /// Returns a validation or capacity error; no partial byte vector escapes.
    pub fn canonical_bytes(&self, limits: AggregateLimits) -> Result<Vec<u8>, TypeError> {
        self.validate(limits)?;
        let mut encoder = Encoder::new(limits.max_serialized_bytes);
        encoder.raw(TYPE_MAGIC)?;
        encode_type(self, &mut encoder, TypeEncoding::Full)?;
        Ok(encoder.finish())
    }

    /// Canonical assignment signature used for exact aggregate compatibility.
    ///
    /// Anonymous structure signatures exclude UUID, comment, and defaults as
    /// required, but include ordered case-folded member names and recursive
    /// type signatures. Named structures additionally include nominal identity.
    ///
    /// # Errors
    ///
    /// Returns a validation or capacity error.
    pub fn assignment_signature(&self, limits: AggregateLimits) -> Result<Vec<u8>, TypeError> {
        self.validate(limits)?;
        let mut encoder = Encoder::new(limits.max_serialized_bytes);
        encoder.raw(SIGNATURE_MAGIC)?;
        encode_type(self, &mut encoder, TypeEncoding::AssignmentSignature)?;
        Ok(encoder.finish())
    }

    /// Returns whether exact baseline assignment is legal; no conversion is
    /// implied for an aggregate or any aggregate leaf.
    ///
    /// # Errors
    ///
    /// Returns a validation/capacity error for either type.
    pub fn assignment_compatible_with(
        &self,
        destination: &Self,
        limits: AggregateLimits,
    ) -> Result<bool, TypeError> {
        Ok(self.assignment_signature(limits)? == destination.assignment_signature(limits)?)
    }

    /// Serializes a typed value with an exact canonical type header.
    ///
    /// # Errors
    ///
    /// Returns a type, shape, or capacity error. NaN and signed-zero bits are
    /// serialized canonically and all numeric fields use big-endian byte order.
    pub fn serialize_value(
        &self,
        value: &PlcValue,
        limits: AggregateLimits,
    ) -> Result<Vec<u8>, TypeError> {
        self.validate_value(value, limits)?;
        let type_bytes = self.canonical_bytes(limits)?;
        let type_len = u32::try_from(type_bytes.len()).map_err(|_| TypeError::LengthOverflow)?;
        let mut encoder = Encoder::new(limits.max_serialized_bytes);
        encoder.raw(VALUE_MAGIC)?;
        encoder.u32(type_len)?;
        encoder.raw(&type_bytes)?;
        encode_value(self, value, &mut encoder)?;
        Ok(encoder.finish())
    }

    /// Deserializes only the exact expected canonical type and rejects trailing,
    /// truncated, noncanonical, wrong-shape, or over-capacity input.
    ///
    /// # Errors
    ///
    /// Returns a deterministic codec/type error without yielding a partial value.
    pub fn deserialize_value(
        &self,
        bytes: &[u8],
        limits: AggregateLimits,
    ) -> Result<PlcValue, TypeError> {
        self.validate(limits)?;
        if bytes.len() > limits.max_serialized_bytes {
            return Err(TypeError::CapacityExceeded);
        }
        let expected_type = self.canonical_bytes(limits)?;
        let mut decoder = Decoder::new(bytes);
        if decoder.take(VALUE_MAGIC.len())? != VALUE_MAGIC {
            return Err(TypeError::InvalidMagic);
        }
        let type_len = usize::try_from(decoder.u32()?).map_err(|_| TypeError::LengthOverflow)?;
        if decoder.take(type_len)? != expected_type {
            return Err(TypeError::TypeHeaderMismatch);
        }
        let value = decode_value(self, &mut decoder, limits)?;
        if !decoder.is_complete() {
            return Err(TypeError::TrailingBytes);
        }
        self.validate_value_at(&value, limits)?;
        Ok(value)
    }

    /// Converts validated multi-dimensional indexes to row-major storage offset.
    ///
    /// # Errors
    ///
    /// Rejects a non-array, wrong index count, or any out-of-bounds index before
    /// returning an offset.
    pub fn array_linear_index(
        &self,
        indexes: &[i32],
        limits: AggregateLimits,
    ) -> Result<usize, TypeError> {
        self.validate(limits)?;
        let Self::Array { dimensions, .. } = self else {
            return Err(TypeError::ValueTypeMismatch);
        };
        if indexes.len() != dimensions.len() {
            return Err(TypeError::ValueShapeMismatch);
        }
        let mut offset = 0_u64;
        for (index, bound) in indexes.iter().copied().zip(dimensions.iter().copied()) {
            if index < bound.lower || index > bound.upper {
                return Err(TypeError::Bounds);
            }
            offset = offset
                .checked_mul(bound.element_count()?)
                .and_then(|value| {
                    value
                        .checked_add(u64::try_from(i64::from(index) - i64::from(bound.lower)).ok()?)
                })
                .ok_or(TypeError::ElementLimit)?;
        }
        usize::try_from(offset).map_err(|_| TypeError::ElementLimit)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StructFieldValue {
    pub member_id: StableUuid,
    pub value: PlcValue,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PlcValue {
    Scalar(TypedScalar),
    Array(Vec<Self>),
    Struct(Vec<StructFieldValue>),
}

impl PlcValue {
    #[must_use]
    pub fn scalar(value: TypedScalar) -> Self {
        Self::Scalar(value)
    }
}

/// Performs one exact, atomic aggregate assignment.
///
/// Anonymous structure values are remapped to destination member identities by
/// canonical member order after signature equality is proven. Named and array
/// identities/bounds remain exact. No partial destination value is returned.
///
/// # Errors
///
/// Returns a validation or compatibility error.
pub fn assign_value(
    source_type: &CanonicalType,
    source_value: &PlcValue,
    destination_type: &CanonicalType,
    limits: AggregateLimits,
) -> Result<PlcValue, TypeError> {
    source_type.validate_value(source_value, limits)?;
    destination_type.validate(limits)?;
    if !source_type.assignment_compatible_with(destination_type, limits)? {
        return Err(TypeError::AssignmentTypeMismatch);
    }
    let assigned = assign_value_at(source_type, source_value, destination_type)?;
    destination_type.validate_value_at(&assigned, limits)?;
    Ok(assigned)
}

fn assign_value_at(
    source_type: &CanonicalType,
    source_value: &PlcValue,
    destination_type: &CanonicalType,
) -> Result<PlcValue, TypeError> {
    match (source_type, source_value, destination_type) {
        (CanonicalType::Primitive(_), PlcValue::Scalar(_), CanonicalType::Primitive(_)) => {
            Ok(source_value.clone())
        }
        (
            CanonicalType::Array {
                element_type: source_element,
                ..
            },
            PlcValue::Array(values),
            CanonicalType::Array {
                element_type: destination_element,
                ..
            },
        ) => values
            .iter()
            .map(|value| assign_value_at(source_element, value, destination_element))
            .collect::<Result<Vec<_>, _>>()
            .map(PlcValue::Array),
        (
            CanonicalType::AnonymousStruct {
                members: source_members,
            }
            | CanonicalType::NamedStruct {
                members: source_members,
                ..
            },
            PlcValue::Struct(fields),
            CanonicalType::AnonymousStruct {
                members: destination_members,
            }
            | CanonicalType::NamedStruct {
                members: destination_members,
                ..
            },
        ) => {
            let source_fields = fields
                .iter()
                .map(|field| (field.member_id, &field.value))
                .collect::<BTreeMap<_, _>>();
            let source_members = ordered_members(source_members);
            let destination_members = ordered_members(destination_members);
            let mut assigned = Vec::with_capacity(destination_members.len());
            for (source_member, destination_member) in
                source_members.into_iter().zip(destination_members)
            {
                let source = source_fields
                    .get(&source_member.id)
                    .ok_or(TypeError::ValueShapeMismatch)?;
                assigned.push(StructFieldValue {
                    member_id: destination_member.id,
                    value: assign_value_at(
                        &source_member.data_type,
                        source,
                        &destination_member.data_type,
                    )?,
                });
            }
            Ok(PlcValue::Struct(assigned))
        }
        _ => Err(TypeError::AssignmentTypeMismatch),
    }
}

/// Returns a cloned array value with one element replaced only after all
/// indexes, shape, and replacement type checks pass.
///
/// # Errors
///
/// Returns a bounds/type/shape error without modifying `current`.
pub fn store_array_element(
    data_type: &CanonicalType,
    current: &PlcValue,
    indexes: &[i32],
    replacement: &PlcValue,
    limits: AggregateLimits,
) -> Result<PlcValue, TypeError> {
    data_type.validate_value(current, limits)?;
    let CanonicalType::Array { element_type, .. } = data_type else {
        return Err(TypeError::ValueTypeMismatch);
    };
    element_type.validate_value(replacement, limits)?;
    let offset = data_type.array_linear_index(indexes, limits)?;
    let PlcValue::Array(values) = current else {
        return Err(TypeError::ValueTypeMismatch);
    };
    let mut assigned = values.clone();
    let slot = assigned
        .get_mut(offset)
        .ok_or(TypeError::ValueShapeMismatch)?;
    *slot = replacement.clone();
    Ok(PlcValue::Array(assigned))
}

fn validate_struct_value(
    members: &[StructMember],
    fields: &[StructFieldValue],
    limits: AggregateLimits,
) -> Result<(), TypeError> {
    if members.len() != fields.len() {
        return Err(TypeError::ValueShapeMismatch);
    }
    let mut by_id = BTreeMap::new();
    for field in fields {
        if by_id.insert(field.member_id, &field.value).is_some() {
            return Err(TypeError::ValueShapeMismatch);
        }
    }
    for member in members {
        let value = by_id.get(&member.id).ok_or(TypeError::ValueShapeMismatch)?;
        member.data_type.validate_value_at(value, limits)?;
    }
    Ok(())
}

fn array_element_count(
    dimensions: &[ArrayBound],
    limits: AggregateLimits,
) -> Result<u64, TypeError> {
    if dimensions.is_empty() || dimensions.len() > usize::from(limits.max_dimensions) {
        return Err(TypeError::InvalidDimensionCount);
    }
    let mut count = 1_u64;
    for dimension in dimensions {
        count = count
            .checked_mul(dimension.element_count()?)
            .ok_or(TypeError::ElementLimit)?;
        if count > limits.max_array_elements {
            return Err(TypeError::ElementLimit);
        }
    }
    Ok(count)
}

fn validate_member_name(name: &str) -> Result<(), TypeError> {
    if name.is_empty()
        || name.len() > MAX_MEMBER_NAME_BYTES
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

fn ordered_members(members: &[StructMember]) -> Vec<&StructMember> {
    let mut ordered = members.iter().collect::<Vec<_>>();
    ordered.sort_by_key(|member| (member.declared_order, member.id));
    ordered
}

#[derive(Clone, Copy)]
enum TypeEncoding {
    Full,
    AssignmentSignature,
}

fn encode_type(
    data_type: &CanonicalType,
    encoder: &mut Encoder,
    mode: TypeEncoding,
) -> Result<(), TypeError> {
    match data_type {
        CanonicalType::Primitive(primitive) => {
            encoder.u8(1)?;
            encode_primitive_type(*primitive, encoder)
        }
        CanonicalType::Array {
            dimensions,
            element_type,
        } => {
            encoder.u8(2)?;
            encoder
                .u8(u8::try_from(dimensions.len()).map_err(|_| TypeError::InvalidDimensionCount)?)?;
            for dimension in dimensions {
                encoder.i32(dimension.lower)?;
                encoder.i32(dimension.upper)?;
            }
            encode_type(element_type, encoder, mode)
        }
        CanonicalType::AnonymousStruct { members } => {
            encoder.u8(3)?;
            encode_members(
                members,
                encoder,
                match mode {
                    TypeEncoding::Full => MemberEncoding::Full,
                    TypeEncoding::AssignmentSignature => MemberEncoding::AnonymousSignature,
                },
                mode,
            )
        }
        CanonicalType::NamedStruct { id, members } => {
            encoder.u8(4)?;
            encoder.raw(&id.as_bytes())?;
            encode_members(
                members,
                encoder,
                match mode {
                    TypeEncoding::Full => MemberEncoding::Full,
                    TypeEncoding::AssignmentSignature => MemberEncoding::NamedSignature,
                },
                mode,
            )
        }
    }
}

#[derive(Clone, Copy)]
enum MemberEncoding {
    Full,
    AnonymousSignature,
    NamedSignature,
}

fn encode_members(
    members: &[StructMember],
    encoder: &mut Encoder,
    member_mode: MemberEncoding,
    nested_mode: TypeEncoding,
) -> Result<(), TypeError> {
    encoder.u32(u32::try_from(members.len()).map_err(|_| TypeError::LengthOverflow)?)?;
    for member in ordered_members(members) {
        if matches!(
            member_mode,
            MemberEncoding::Full | MemberEncoding::NamedSignature
        ) {
            encoder.raw(&member.id.as_bytes())?;
            encoder.u32(member.declared_order)?;
        }
        let folded = member.name.to_ascii_lowercase();
        encoder.text(&folded)?;
        encode_type(&member.data_type, encoder, nested_mode)?;
        if matches!(member_mode, MemberEncoding::Full) {
            match &member.reusable_default {
                Some(value) => {
                    encoder.u8(1)?;
                    encode_value(&member.data_type, value, encoder)?;
                }
                None => encoder.u8(0)?,
            }
        }
    }
    Ok(())
}

fn encode_primitive_type(primitive: PrimitiveType, encoder: &mut Encoder) -> Result<(), TypeError> {
    let tag = match primitive {
        PrimitiveType::Bool => 1,
        PrimitiveType::Sint => 2,
        PrimitiveType::Int => 3,
        PrimitiveType::Dint => 4,
        PrimitiveType::Lint => 5,
        PrimitiveType::Usint => 6,
        PrimitiveType::Uint => 7,
        PrimitiveType::Udint => 8,
        PrimitiveType::Ulint => 9,
        PrimitiveType::Byte => 10,
        PrimitiveType::Word => 11,
        PrimitiveType::Dword => 12,
        PrimitiveType::Lword => 13,
        PrimitiveType::Real => 14,
        PrimitiveType::Lreal => 15,
        PrimitiveType::Char => 16,
        PrimitiveType::String(_) => 17,
        PrimitiveType::Time => 18,
    };
    encoder.u8(tag)?;
    if let PrimitiveType::String(capacity) = primitive {
        encoder.u8(capacity)?;
    }
    Ok(())
}

fn encode_value(
    data_type: &CanonicalType,
    value: &PlcValue,
    encoder: &mut Encoder,
) -> Result<(), TypeError> {
    match (data_type, value) {
        (CanonicalType::Primitive(primitive), PlcValue::Scalar(value)) => {
            encode_scalar(*primitive, value, encoder)
        }
        (CanonicalType::Array { element_type, .. }, PlcValue::Array(values)) => {
            encoder.u64(u64::try_from(values.len()).map_err(|_| TypeError::LengthOverflow)?)?;
            for value in values {
                encode_value(element_type, value, encoder)?;
            }
            Ok(())
        }
        (
            CanonicalType::AnonymousStruct { members } | CanonicalType::NamedStruct { members, .. },
            PlcValue::Struct(fields),
        ) => {
            encoder.u32(u32::try_from(fields.len()).map_err(|_| TypeError::LengthOverflow)?)?;
            let by_id = fields
                .iter()
                .map(|field| (field.member_id, &field.value))
                .collect::<BTreeMap<_, _>>();
            for member in ordered_members(members) {
                encoder.raw(&member.id.as_bytes())?;
                let field = by_id.get(&member.id).ok_or(TypeError::ValueShapeMismatch)?;
                encode_value(&member.data_type, field, encoder)?;
            }
            Ok(())
        }
        _ => Err(TypeError::ValueTypeMismatch),
    }
}

#[allow(clippy::too_many_lines)]
fn encode_scalar(
    primitive: PrimitiveType,
    value: &TypedScalar,
    encoder: &mut Encoder,
) -> Result<(), TypeError> {
    if value.data_type() != primitive {
        return Err(TypeError::ValueTypeMismatch);
    }
    match (primitive, value.value()) {
        (PrimitiveType::Bool, ScalarValue::Bool(value)) => encoder.u8(u8::from(*value)),
        (PrimitiveType::Sint, ScalarValue::Signed(value)) => encoder.raw(
            &i8::try_from(*value)
                .map_err(|_| TypeError::ValueTypeMismatch)?
                .to_be_bytes(),
        ),
        (PrimitiveType::Int, ScalarValue::Signed(value)) => encoder.raw(
            &i16::try_from(*value)
                .map_err(|_| TypeError::ValueTypeMismatch)?
                .to_be_bytes(),
        ),
        (PrimitiveType::Dint, ScalarValue::Signed(value)) => encoder.raw(
            &i32::try_from(*value)
                .map_err(|_| TypeError::ValueTypeMismatch)?
                .to_be_bytes(),
        ),
        (PrimitiveType::Lint, ScalarValue::Signed(value))
        | (PrimitiveType::Time, ScalarValue::Time(value)) => encoder.i64(*value),
        (PrimitiveType::Usint, ScalarValue::Unsigned(value)) => {
            encoder.u8(u8::try_from(*value).map_err(|_| TypeError::ValueTypeMismatch)?)
        }
        (PrimitiveType::Uint, ScalarValue::Unsigned(value)) => {
            encoder.u16(u16::try_from(*value).map_err(|_| TypeError::ValueTypeMismatch)?)
        }
        (PrimitiveType::Udint, ScalarValue::Unsigned(value)) => {
            encoder.u32(u32::try_from(*value).map_err(|_| TypeError::ValueTypeMismatch)?)
        }
        (PrimitiveType::Ulint, ScalarValue::Unsigned(value))
        | (PrimitiveType::Lword, ScalarValue::BitString(value)) => encoder.u64(*value),
        (PrimitiveType::Byte, ScalarValue::BitString(value)) => {
            encoder.u8(u8::try_from(*value).map_err(|_| TypeError::ValueTypeMismatch)?)
        }
        (PrimitiveType::Word, ScalarValue::BitString(value)) => {
            encoder.u16(u16::try_from(*value).map_err(|_| TypeError::ValueTypeMismatch)?)
        }
        (PrimitiveType::Dword, ScalarValue::BitString(value)) => {
            encoder.u32(u32::try_from(*value).map_err(|_| TypeError::ValueTypeMismatch)?)
        }
        (PrimitiveType::Real, ScalarValue::Real(value)) => encoder.u32(value.bits()),
        (PrimitiveType::Lreal, ScalarValue::Lreal(value)) => encoder.u64(value.bits()),
        (PrimitiveType::Char, ScalarValue::Char(value)) => encoder.u8(*value),
        (PrimitiveType::String(_), ScalarValue::String(value)) => {
            encoder.u8(u8::try_from(value.len()).map_err(|_| TypeError::ValueTypeMismatch)?)?;
            encoder.raw(value)
        }
        _ => Err(TypeError::ValueTypeMismatch),
    }
}

fn decode_value(
    data_type: &CanonicalType,
    decoder: &mut Decoder<'_>,
    limits: AggregateLimits,
) -> Result<PlcValue, TypeError> {
    match data_type {
        CanonicalType::Primitive(primitive) => {
            decode_scalar(*primitive, decoder).map(PlcValue::Scalar)
        }
        CanonicalType::Array {
            dimensions,
            element_type,
        } => {
            let expected = array_element_count(dimensions, limits)?;
            if decoder.u64()? != expected {
                return Err(TypeError::ValueShapeMismatch);
            }
            let count = usize::try_from(expected).map_err(|_| TypeError::ElementLimit)?;
            let mut values = Vec::with_capacity(count);
            for _ in 0..count {
                values.push(decode_value(element_type, decoder, limits)?);
            }
            Ok(PlcValue::Array(values))
        }
        CanonicalType::AnonymousStruct { members } | CanonicalType::NamedStruct { members, .. } => {
            if usize::try_from(decoder.u32()?).ok() != Some(members.len()) {
                return Err(TypeError::ValueShapeMismatch);
            }
            let mut fields = Vec::with_capacity(members.len());
            for member in ordered_members(members) {
                let id_bytes: [u8; 16] = decoder
                    .take(16)?
                    .try_into()
                    .map_err(|_| TypeError::Truncated)?;
                let id = StableUuid::from_bytes(id_bytes)?;
                if id != member.id {
                    return Err(TypeError::ValueShapeMismatch);
                }
                fields.push(StructFieldValue {
                    member_id: id,
                    value: decode_value(&member.data_type, decoder, limits)?,
                });
            }
            Ok(PlcValue::Struct(fields))
        }
    }
}

#[allow(clippy::too_many_lines)]
fn decode_scalar(
    primitive: PrimitiveType,
    decoder: &mut Decoder<'_>,
) -> Result<TypedScalar, TypeError> {
    let value = match primitive {
        PrimitiveType::Bool => match decoder.u8()? {
            0 => ScalarValue::Bool(false),
            1 => ScalarValue::Bool(true),
            _ => return Err(TypeError::NonCanonicalEncoding),
        },
        PrimitiveType::Sint => ScalarValue::Signed(i64::from(i8::from_be_bytes([decoder.u8()?]))),
        PrimitiveType::Int => ScalarValue::Signed(i64::from(i16::from_be_bytes(decoder.array()?))),
        PrimitiveType::Dint => ScalarValue::Signed(i64::from(i32::from_be_bytes(decoder.array()?))),
        PrimitiveType::Lint => ScalarValue::Signed(decoder.i64()?),
        PrimitiveType::Usint => ScalarValue::Unsigned(u64::from(decoder.u8()?)),
        PrimitiveType::Uint => ScalarValue::Unsigned(u64::from(decoder.u16()?)),
        PrimitiveType::Udint => ScalarValue::Unsigned(u64::from(decoder.u32()?)),
        PrimitiveType::Ulint => ScalarValue::Unsigned(decoder.u64()?),
        PrimitiveType::Byte => ScalarValue::BitString(u64::from(decoder.u8()?)),
        PrimitiveType::Word => ScalarValue::BitString(u64::from(decoder.u16()?)),
        PrimitiveType::Dword => ScalarValue::BitString(u64::from(decoder.u32()?)),
        PrimitiveType::Lword => ScalarValue::BitString(decoder.u64()?),
        PrimitiveType::Real => {
            let bits = decoder.u32()?;
            let canonical = CanonicalF32::from_bits(bits);
            if canonical.bits() != bits {
                return Err(TypeError::NonCanonicalEncoding);
            }
            ScalarValue::Real(canonical)
        }
        PrimitiveType::Lreal => {
            let bits = decoder.u64()?;
            let canonical = CanonicalF64::from_bits(bits);
            if canonical.bits() != bits {
                return Err(TypeError::NonCanonicalEncoding);
            }
            ScalarValue::Lreal(canonical)
        }
        PrimitiveType::Char => ScalarValue::Char(decoder.u8()?),
        PrimitiveType::String(capacity) => {
            let length = usize::from(decoder.u8()?);
            if length > usize::from(capacity) {
                return Err(TypeError::ValueTypeMismatch);
            }
            ScalarValue::String(decoder.take(length)?.to_vec())
        }
        PrimitiveType::Time => ScalarValue::Time(decoder.i64()?),
    };
    TypedScalar::new(primitive, value).map_err(TypeError::Scalar)
}

struct Encoder {
    bytes: Vec<u8>,
    limit: usize,
}

impl Encoder {
    fn new(limit: usize) -> Self {
        Self {
            bytes: Vec::new(),
            limit,
        }
    }

    fn raw(&mut self, value: &[u8]) -> Result<(), TypeError> {
        let next = self
            .bytes
            .len()
            .checked_add(value.len())
            .ok_or(TypeError::CapacityExceeded)?;
        if next > self.limit {
            return Err(TypeError::CapacityExceeded);
        }
        self.bytes.extend_from_slice(value);
        Ok(())
    }

    fn u8(&mut self, value: u8) -> Result<(), TypeError> {
        self.raw(&[value])
    }

    fn u16(&mut self, value: u16) -> Result<(), TypeError> {
        self.raw(&value.to_be_bytes())
    }

    fn u32(&mut self, value: u32) -> Result<(), TypeError> {
        self.raw(&value.to_be_bytes())
    }

    fn i32(&mut self, value: i32) -> Result<(), TypeError> {
        self.raw(&value.to_be_bytes())
    }

    fn u64(&mut self, value: u64) -> Result<(), TypeError> {
        self.raw(&value.to_be_bytes())
    }

    fn i64(&mut self, value: i64) -> Result<(), TypeError> {
        self.raw(&value.to_be_bytes())
    }

    fn text(&mut self, value: &str) -> Result<(), TypeError> {
        self.u16(u16::try_from(value.len()).map_err(|_| TypeError::LengthOverflow)?)?;
        self.raw(value.as_bytes())
    }

    fn finish(self) -> Vec<u8> {
        self.bytes
    }
}

struct Decoder<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> Decoder<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, position: 0 }
    }

    fn take(&mut self, count: usize) -> Result<&'a [u8], TypeError> {
        let end = self
            .position
            .checked_add(count)
            .ok_or(TypeError::Truncated)?;
        let value = self
            .bytes
            .get(self.position..end)
            .ok_or(TypeError::Truncated)?;
        self.position = end;
        Ok(value)
    }

    fn array<const N: usize>(&mut self) -> Result<[u8; N], TypeError> {
        self.take(N)?.try_into().map_err(|_| TypeError::Truncated)
    }

    fn u8(&mut self) -> Result<u8, TypeError> {
        Ok(self.array::<1>()?[0])
    }

    fn u16(&mut self) -> Result<u16, TypeError> {
        Ok(u16::from_be_bytes(self.array()?))
    }

    fn u32(&mut self) -> Result<u32, TypeError> {
        Ok(u32::from_be_bytes(self.array()?))
    }

    fn u64(&mut self) -> Result<u64, TypeError> {
        Ok(u64::from_be_bytes(self.array()?))
    }

    fn i64(&mut self) -> Result<i64, TypeError> {
        Ok(i64::from_be_bytes(self.array()?))
    }

    const fn is_complete(&self) -> bool {
        self.position == self.bytes.len()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TypeError {
    InvalidLimits,
    InvalidIdentity,
    InvalidStringCapacity,
    InvalidArrayBound,
    InvalidDimensionCount,
    NestingLimit,
    MemberLimit,
    ElementLimit,
    InvalidMemberName,
    DuplicateMemberName,
    DuplicateMemberIdentity,
    DuplicateDeclaredOrder,
    DefaultValueMismatch,
    ValueTypeMismatch,
    ValueShapeMismatch,
    AssignmentTypeMismatch,
    Bounds,
    CapacityExceeded,
    LengthOverflow,
    InvalidMagic,
    TypeHeaderMismatch,
    Truncated,
    TrailingBytes,
    NonCanonicalEncoding,
    Scalar(ScalarTypeError),
}

impl fmt::Display for TypeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl core::error::Error for TypeError {}
