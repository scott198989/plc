use core::fmt;

use plc_core::Uuid;

macro_rules! typed_id {
    ($name:ident) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(pub Uuid);

        impl $name {
            #[must_use]
            pub const fn new(value: Uuid) -> Self {
                Self(value)
            }

            #[must_use]
            pub const fn uuid(self) -> Uuid {
                self.0
            }
        }

        impl From<Uuid> for $name {
            fn from(value: Uuid) -> Self {
                Self(value)
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::Display::fmt(&self.0, formatter)
            }
        }
    };
}

typed_id!(ControllerId);
typed_id!(RackId);
typed_id!(SlotId);
typed_id!(ModuleId);
typed_id!(ChannelId);
typed_id!(ParameterId);
typed_id!(StationId);
typed_id!(VirtualDeviceId);
typed_id!(VirtualInterfaceId);
typed_id!(VirtualSubnetId);
typed_id!(VirtualPortId);
typed_id!(VirtualLinkId);
typed_id!(ScopeId);
typed_id!(DeclarationId);
typed_id!(ReferenceId);
typed_id!(TagTableId);
typed_id!(TagId);
typed_id!(TypeDeclarationId);
typed_id!(SourceObjectId);
