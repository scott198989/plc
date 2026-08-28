use alloc::{collections::BTreeMap, string::String, vec::Vec};

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
    Int,
    DInt,
    Real,
    Time,
    String { capacity: u16 },
    Named(String),
    BlockInstance(BlockId),
    InstructionState(StateKind),
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CanonicalValue {
    Bool(bool),
    Int(i16),
    DInt(i32),
    RealBits(u32),
    TimeMilliseconds(i64),
    StringBytes(Vec<u8>),
}

impl CanonicalValue {
    #[must_use]
    pub fn is_compatible_with(&self, data_type: &DataType) -> bool {
        match (self, data_type) {
            (Self::Bool(_), DataType::Bool)
            | (Self::Int(_), DataType::Int)
            | (Self::DInt(_), DataType::DInt)
            | (Self::TimeMilliseconds(_), DataType::Time) => true,
            (Self::RealBits(bits), DataType::Real) => {
                let exponent = (bits >> 23) & 0xff;
                let fraction = bits & 0x7f_ffff;
                exponent != 0xff || fraction == 0 || *bits == 0x7fc0_0000
            }
            (Self::StringBytes(bytes), DataType::String { capacity }) => {
                bytes.len() <= usize::from(*capacity)
            }
            _ => false,
        }
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
