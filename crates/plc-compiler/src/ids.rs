use core::fmt;

macro_rules! stable_id {
    ($name:ident, $inner:ty) => {
        #[derive(Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name($inner);

        impl $name {
            #[must_use]
            pub const fn new(value: $inner) -> Self {
                Self(value)
            }

            #[must_use]
            pub const fn get(self) -> $inner {
                self.0
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(formatter, "{}({})", stringify!($name), self.0)
            }
        }
    };
}

stable_id!(BuildAttemptId, u128);
stable_id!(SemanticNodeId, u32);
stable_id!(SourceMapId, u32);
stable_id!(ProbeId, u32);
stable_id!(IrBasicBlockId, u32);
stable_id!(IrOperationId, u32);
stable_id!(IrValueId, u32);
