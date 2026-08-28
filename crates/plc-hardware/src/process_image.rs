use core::fmt;

use crate::hardware::{ChannelAddress, HardwareChannelBinding};
use crate::types::PrimitiveType;

/// Exact raw values carried by every EDU-21 hardware channel.
///
/// Digital channels are single bits. Analog and temperature channels are
/// signed 16-bit two's-complement integers; engineering LREAL values are a
/// projection and never occupy a second process-image address.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ChannelRawValue {
    Bool(bool),
    Int(i16),
}

impl ChannelRawValue {
    #[must_use]
    pub const fn canonical_default(raw_type: PrimitiveType) -> Option<Self> {
        match raw_type {
            PrimitiveType::Bool => Some(Self::Bool(false)),
            PrimitiveType::Int => Some(Self::Int(0)),
            _ => None,
        }
    }

    #[must_use]
    pub const fn matches(self, raw_type: PrimitiveType) -> bool {
        matches!(
            (self, raw_type),
            (Self::Bool(_), PrimitiveType::Bool) | (Self::Int(_), PrimitiveType::Int)
        )
    }
}

impl HardwareChannelBinding {
    /// Reads the channel's exact raw value using EDU-21 little-endian and LSB0
    /// process-image rules.
    ///
    /// # Errors
    ///
    /// Returns an error for a channel/address type mismatch, an invalid bit
    /// address, or an image too short for the configured channel.
    pub fn read_raw(&self, image: &[u8]) -> Result<ChannelRawValue, ProcessImageError> {
        let address_area = match self.address {
            ChannelAddress::Bit { area, .. } | ChannelAddress::Word { area, .. } => area,
        };
        if address_area != self.direction.area() {
            return Err(ProcessImageError::InvalidAddress);
        }
        match (self.address, self.raw_type) {
            (ChannelAddress::Bit { byte, bit, .. }, PrimitiveType::Bool) if bit < 8 => {
                let index = usize::try_from(byte).map_err(|_| ProcessImageError::OutOfBounds)?;
                let value = image.get(index).ok_or(ProcessImageError::OutOfBounds)?;
                Ok(ChannelRawValue::Bool(value & (1_u8 << bit) != 0))
            }
            (ChannelAddress::Word { byte, .. }, PrimitiveType::Int) => {
                let index = usize::try_from(byte).map_err(|_| ProcessImageError::OutOfBounds)?;
                let low = *image.get(index).ok_or(ProcessImageError::OutOfBounds)?;
                let high = *image
                    .get(index.checked_add(1).ok_or(ProcessImageError::OutOfBounds)?)
                    .ok_or(ProcessImageError::OutOfBounds)?;
                Ok(ChannelRawValue::Int(i16::from_le_bytes([low, high])))
            }
            (ChannelAddress::Bit { .. }, PrimitiveType::Bool) => {
                Err(ProcessImageError::InvalidAddress)
            }
            _ => Err(ProcessImageError::TypeMismatch),
        }
    }

    /// Writes the channel's exact raw value without disturbing unrelated bits.
    ///
    /// # Errors
    ///
    /// Returns an error for a raw-value/address type mismatch, an invalid bit
    /// address, or an image too short for the configured channel.
    pub fn write_raw(
        &self,
        image: &mut [u8],
        value: ChannelRawValue,
    ) -> Result<(), ProcessImageError> {
        if !value.matches(self.raw_type) {
            return Err(ProcessImageError::TypeMismatch);
        }
        let address_area = match self.address {
            ChannelAddress::Bit { area, .. } | ChannelAddress::Word { area, .. } => area,
        };
        if address_area != self.direction.area() {
            return Err(ProcessImageError::InvalidAddress);
        }
        match (self.address, value) {
            (ChannelAddress::Bit { byte, bit, .. }, ChannelRawValue::Bool(value)) if bit < 8 => {
                let index = usize::try_from(byte).map_err(|_| ProcessImageError::OutOfBounds)?;
                let target = image.get_mut(index).ok_or(ProcessImageError::OutOfBounds)?;
                let mask = 1_u8 << bit;
                if value {
                    *target |= mask;
                } else {
                    *target &= !mask;
                }
                Ok(())
            }
            (ChannelAddress::Word { byte, .. }, ChannelRawValue::Int(value)) => {
                let index = usize::try_from(byte).map_err(|_| ProcessImageError::OutOfBounds)?;
                let next = index.checked_add(1).ok_or(ProcessImageError::OutOfBounds)?;
                if next >= image.len() {
                    return Err(ProcessImageError::OutOfBounds);
                }
                let [low, high] = value.to_le_bytes();
                image[index] = low;
                image[next] = high;
                Ok(())
            }
            (ChannelAddress::Bit { .. }, ChannelRawValue::Bool(_)) => {
                Err(ProcessImageError::InvalidAddress)
            }
            _ => Err(ProcessImageError::TypeMismatch),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProcessImageError {
    TypeMismatch,
    InvalidAddress,
    OutOfBounds,
}

impl fmt::Display for ProcessImageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for ProcessImageError {}
