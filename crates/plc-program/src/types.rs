use alloc::{collections::BTreeMap, string::String, vec::Vec};
use plc_types::{
    CanonicalF32, CanonicalF64, CanonicalType, PlcValue, PrimitiveType, ScalarValue, TypedScalar,
};

use crate::{BlockId, InterfaceMemberId, instruction::StateKind};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EngineeringNumber(u16);

impl EngineeringNumber {
    #[must_use]
    pub const fn new(value: u16) -> Option<Self> {
        if value == 0 { None } else { Some(Self(value)) }
    }

    #[must_use]
    pub const fn get(self) -> u16 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RetainPolicy {
    NonRetentive,
    Retentive,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DataType {
    Bool,
    SInt,
    Int,
    DInt,
    LInt,
    USInt,
    UInt,
    UDInt,
    ULInt,
    Byte,
    Word,
    DWord,
    LWord,
    Real,
    LReal,
    Char,
    Time,
    String { capacity: u16 },
    Named(String),
    BlockInstance(BlockId),
    InstructionState(StateKind),
}

impl DataType {
    #[must_use]
    pub fn primitive_type(&self) -> Option<PrimitiveType> {
        match self {
            Self::Bool => Some(PrimitiveType::Bool),
            Self::SInt => Some(PrimitiveType::Sint),
            Self::Int => Some(PrimitiveType::Int),
            Self::DInt => Some(PrimitiveType::Dint),
            Self::LInt => Some(PrimitiveType::Lint),
            Self::USInt => Some(PrimitiveType::Usint),
            Self::UInt => Some(PrimitiveType::Uint),
            Self::UDInt => Some(PrimitiveType::Udint),
            Self::ULInt => Some(PrimitiveType::Ulint),
            Self::Byte => Some(PrimitiveType::Byte),
            Self::Word => Some(PrimitiveType::Word),
            Self::DWord => Some(PrimitiveType::Dword),
            Self::LWord => Some(PrimitiveType::Lword),
            Self::Real => Some(PrimitiveType::Real),
            Self::LReal => Some(PrimitiveType::Lreal),
            Self::Char => Some(PrimitiveType::Char),
            Self::Time => Some(PrimitiveType::Time),
            Self::String { capacity } => u8::try_from(*capacity)
                .ok()
                .filter(|capacity| *capacity <= 254)
                .map(PrimitiveType::String),
            Self::Named(_) | Self::BlockInstance(_) | Self::InstructionState(_) => None,
        }
    }

    #[must_use]
    pub const fn from_primitive(primitive: PrimitiveType) -> Self {
        match primitive {
            PrimitiveType::Bool => Self::Bool,
            PrimitiveType::Sint => Self::SInt,
            PrimitiveType::Int => Self::Int,
            PrimitiveType::Dint => Self::DInt,
            PrimitiveType::Lint => Self::LInt,
            PrimitiveType::Usint => Self::USInt,
            PrimitiveType::Uint => Self::UInt,
            PrimitiveType::Udint => Self::UDInt,
            PrimitiveType::Ulint => Self::ULInt,
            PrimitiveType::Byte => Self::Byte,
            PrimitiveType::Word => Self::Word,
            PrimitiveType::Dword => Self::DWord,
            PrimitiveType::Lword => Self::LWord,
            PrimitiveType::Real => Self::Real,
            PrimitiveType::Lreal => Self::LReal,
            PrimitiveType::Char => Self::Char,
            PrimitiveType::String(capacity) => Self::String {
                capacity: capacity as u16,
            },
            PrimitiveType::Time => Self::Time,
        }
    }

    /// Projects a scalar program type into the shared recursive type authority.
    /// Named and instance types require their owning registry and therefore fail
    /// closed instead of manufacturing a disconnected shape.
    #[must_use]
    pub fn canonical_scalar_type(&self) -> Option<CanonicalType> {
        self.primitive_type().map(CanonicalType::Primitive)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CanonicalValue {
    Bool(bool),
    SInt(i8),
    Int(i16),
    DInt(i32),
    LInt(i64),
    USInt(u8),
    UInt(u16),
    UDInt(u32),
    ULInt(u64),
    Byte(u8),
    Word(u16),
    DWord(u32),
    LWord(u64),
    RealBits(u32),
    LRealBits(u64),
    Char(u8),
    TimeMilliseconds(i64),
    StringBytes(Vec<u8>),
}

impl CanonicalValue {
    #[must_use]
    pub fn is_compatible_with(&self, data_type: &DataType) -> bool {
        data_type
            .primitive_type()
            .and_then(|primitive| {
                self.scalar_value_for(primitive)
                    .map(|value| (primitive, value))
            })
            .is_some_and(|(primitive, value)| primitive.validate_scalar(&value).is_ok())
    }

    #[must_use]
    pub fn scalar_value_for(&self, data_type: PrimitiveType) -> Option<ScalarValue> {
        match (self, data_type) {
            (Self::Bool(value), PrimitiveType::Bool) => Some(ScalarValue::Bool(*value)),
            (Self::SInt(value), PrimitiveType::Sint) => {
                Some(ScalarValue::Signed(i64::from(*value)))
            }
            (Self::Int(value), PrimitiveType::Int) => Some(ScalarValue::Signed(i64::from(*value))),
            (Self::DInt(value), PrimitiveType::Dint) => {
                Some(ScalarValue::Signed(i64::from(*value)))
            }
            (Self::LInt(value), PrimitiveType::Lint) => Some(ScalarValue::Signed(*value)),
            (Self::USInt(value), PrimitiveType::Usint) => {
                Some(ScalarValue::Unsigned(u64::from(*value)))
            }
            (Self::UInt(value), PrimitiveType::Uint) => {
                Some(ScalarValue::Unsigned(u64::from(*value)))
            }
            (Self::UDInt(value), PrimitiveType::Udint) => {
                Some(ScalarValue::Unsigned(u64::from(*value)))
            }
            (Self::ULInt(value), PrimitiveType::Ulint) => Some(ScalarValue::Unsigned(*value)),
            (Self::Byte(value), PrimitiveType::Byte) => {
                Some(ScalarValue::BitString(u64::from(*value)))
            }
            (Self::Word(value), PrimitiveType::Word) => {
                Some(ScalarValue::BitString(u64::from(*value)))
            }
            (Self::DWord(value), PrimitiveType::Dword) => {
                Some(ScalarValue::BitString(u64::from(*value)))
            }
            (Self::LWord(value), PrimitiveType::Lword) => Some(ScalarValue::BitString(*value)),
            (Self::RealBits(bits), PrimitiveType::Real)
                if CanonicalF32::from_bits(*bits).bits() == *bits =>
            {
                Some(ScalarValue::Real(CanonicalF32::from_bits(*bits)))
            }
            (Self::LRealBits(bits), PrimitiveType::Lreal)
                if CanonicalF64::from_bits(*bits).bits() == *bits =>
            {
                Some(ScalarValue::Lreal(CanonicalF64::from_bits(*bits)))
            }
            (Self::Char(value), PrimitiveType::Char) => Some(ScalarValue::Char(*value)),
            (Self::TimeMilliseconds(value), PrimitiveType::Time) => Some(ScalarValue::Time(*value)),
            (Self::StringBytes(bytes), PrimitiveType::String(_)) => {
                Some(ScalarValue::String(bytes.clone()))
            }
            _ => None,
        }
    }

    /// Projects an existing program declaration/default/start/constant scalar
    /// into the shared recursive value authority without conversion.
    #[must_use]
    pub fn plc_value_for(&self, data_type: &DataType) -> Option<PlcValue> {
        let primitive = data_type.primitive_type()?;
        let value = self.scalar_value_for(primitive)?;
        TypedScalar::new(primitive, value)
            .ok()
            .map(PlcValue::scalar)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum InterfaceRole {
    Input,
    Output,
    InOut,
    Static,
    Temp,
    Constant,
    Return,
}

impl InterfaceRole {
    #[must_use]
    pub(crate) const fn canonical_rank(self) -> u8 {
        match self {
            Self::Input => 0,
            Self::Output => 1,
            Self::InOut => 2,
            Self::Static => 3,
            Self::Temp => 4,
            Self::Constant => 5,
            Self::Return => 6,
        }
    }

    #[must_use]
    pub(crate) const fn is_call_formal(self) -> bool {
        matches!(
            self,
            Self::Input | Self::Output | Self::InOut | Self::Return
        )
    }

    #[must_use]
    pub(crate) const fn is_public_signature(self) -> bool {
        self.is_call_formal()
    }

    #[must_use]
    pub(crate) const fn is_fb_instance_layout(self) -> bool {
        matches!(
            self,
            Self::Input | Self::Output | Self::InOut | Self::Static
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InterfaceMember {
    pub id: InterfaceMemberId,
    pub name: String,
    pub role: InterfaceRole,
    pub data_type: DataType,
    pub declared_order: u32,
    pub default_value: Option<CanonicalValue>,
    pub start_value: Option<CanonicalValue>,
    pub constant_value: Option<CanonicalValue>,
    pub retain_policy: Option<RetainPolicy>,
    pub required_output_binding: bool,
}

impl InterfaceMember {
    #[must_use]
    pub fn plain(
        id: InterfaceMemberId,
        name: impl Into<String>,
        role: InterfaceRole,
        data_type: DataType,
        declared_order: u32,
    ) -> Self {
        Self {
            id,
            name: name.into(),
            role,
            data_type,
            declared_order,
            default_value: None,
            start_value: None,
            constant_value: None,
            retain_policy: None,
            required_output_binding: false,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BlockInterface {
    pub members: BTreeMap<InterfaceMemberId, InterfaceMember>,
    /// Canonical section/order projection. Validation requires this to contain
    /// each member exactly once in `(role, declared_order, id)` order.
    pub ordered_member_ids: Vec<InterfaceMemberId>,
}

impl BlockInterface {
    #[must_use]
    pub fn from_members(members: impl IntoIterator<Item = InterfaceMember>) -> Self {
        let mut by_id = BTreeMap::new();
        for member in members {
            by_id.insert(member.id, member);
        }
        let mut ordered_member_ids: Vec<_> = by_id.keys().copied().collect();
        ordered_member_ids.sort_by_key(|id| {
            let member = &by_id[id];
            (
                member.role.canonical_rank(),
                member.declared_order,
                member.id,
            )
        });
        Self {
            members: by_id,
            ordered_member_ids,
        }
    }

    #[must_use]
    pub fn member(&self, id: InterfaceMemberId) -> Option<&InterfaceMember> {
        self.members.get(&id)
    }
}
