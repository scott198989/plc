use core::fmt;

macro_rules! stable_id {
    ($name:ident) => {
        #[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(u128);

        impl $name {
            /// Constructs an identity allocated by the canonical project owner.
            /// This crate never generates identities or accesses entropy.
            #[must_use]
            pub const fn new(value: u128) -> Self {
                Self(value)
            }

            #[must_use]
            pub const fn get(self) -> u128 {
                self.0
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(formatter, "{}({:032x})", stringify!($name), self.0)
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(formatter, "{:032x}", self.0)
            }
        }
    };
}

stable_id!(LadDocumentId);
stable_id!(LadNetworkId);
stable_id!(LadNodeId);
stable_id!(LadPortId);
stable_id!(LadEdgeId);
stable_id!(LadBranchId);
stable_id!(LadBranchPathId);
stable_id!(LadOperandId);
stable_id!(LadStateInstanceId);
