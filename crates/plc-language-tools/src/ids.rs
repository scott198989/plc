use core::fmt;

macro_rules! stable_id {
    ($name:ident) => {
        #[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(u128);

        impl $name {
            /// Constructs an identity supplied by the project kernel.
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

stable_id!(FbdDocumentId);
stable_id!(NetworkId);
stable_id!(NodeId);
stable_id!(PortId);
stable_id!(ConnectionId);
stable_id!(StateInstanceId);
