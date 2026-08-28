#![forbid(unsafe_code)]

//! Canonical primitive PLC types and deterministic EDU-21 scalar semantics.
//!
//! This crate is deliberately capability-free. It owns the primitive type
//! table used by authoring, compilation, hardware projection, and execution;
//! none of its behavior is inferred from the host CPU, clock, locale, or ABI.

extern crate alloc;

use alloc::{string::String, vec::Vec};
use core::{cmp::Ordering, fmt};

pub const DEFAULT_STRING_CAPACITY: u8 = 254;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CanonicalF32(u32);

impl CanonicalF32 {
    pub const QUIET_NAN_BITS: u32 = 0x7fc0_0000;

    #[must_use]
    pub fn new(value: f32) -> Self {
        Self(if value.is_nan() {
            Self::QUIET_NAN_BITS
        } else {
            value.to_bits()
        })
    }

    #[must_use]
    pub const fn from_bits(bits: u32) -> Self {
        let exponent = bits & 0x7f80_0000;
        let fraction = bits & 0x007f_ffff;
        if exponent == 0x7f80_0000 && fraction != 0 {
            Self(Self::QUIET_NAN_BITS)
        } else {
            Self(bits)
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
        Self(if value.is_nan() {
            Self::QUIET_NAN_BITS
        } else {
            value.to_bits()
        })
    }

    #[must_use]
    pub const fn from_bits(bits: u64) -> Self {
        let exponent = bits & 0x7ff0_0000_0000_0000;
        let fraction = bits & 0x000f_ffff_ffff_ffff;
        if exponent == 0x7ff0_0000_0000_0000 && fraction != 0 {
            Self(Self::QUIET_NAN_BITS)
        } else {
            Self(bits)
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PrimitiveCategory {
    Boolean,
    SignedInteger,
    UnsignedInteger,
    BitString,
    FloatingPoint,
    Character,
    String,
    Duration,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Signedness {
    NotApplicable,
    Signed,
    Unsigned,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PrimitiveRange {
    Boolean,
    Signed { minimum: i64, maximum: i64 },
    Unsigned { minimum: u64, maximum: u64 },
    Ieee754,
    StringLength { minimum: u8, maximum: u8 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PrimitiveTypeId {
    pub stable_name: &'static str,
    pub category: PrimitiveCategory,
    pub width_bits: Option<u8>,
    pub signedness: Signedness,
    pub range: PrimitiveRange,
    pub representation: &'static str,
    pub literal_rules: &'static str,
    pub string_capacity: Option<u8>,
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
        match self.width_bits() {
            Some(bits) => Some(bits / 8),
            None => None,
        }
    }

    #[must_use]
    pub const fn width_bits(self) -> Option<u8> {
        match self {
            Self::Bool | Self::String(_) => None,
            Self::Sint | Self::Usint | Self::Byte | Self::Char => Some(8),
            Self::Int | Self::Uint | Self::Word => Some(16),
            Self::Dint | Self::Udint | Self::Dword | Self::Real => Some(32),
            Self::Lint | Self::Ulint | Self::Lword | Self::Lreal | Self::Time => Some(64),
        }
    }

    #[must_use]
    pub const fn category(self) -> PrimitiveCategory {
        match self {
            Self::Bool => PrimitiveCategory::Boolean,
            Self::Sint | Self::Int | Self::Dint | Self::Lint => PrimitiveCategory::SignedInteger,
            Self::Usint | Self::Uint | Self::Udint | Self::Ulint => {
                PrimitiveCategory::UnsignedInteger
            }
            Self::Byte | Self::Word | Self::Dword | Self::Lword => PrimitiveCategory::BitString,
            Self::Real | Self::Lreal => PrimitiveCategory::FloatingPoint,
            Self::Char => PrimitiveCategory::Character,
            Self::String(_) => PrimitiveCategory::String,
            Self::Time => PrimitiveCategory::Duration,
        }
    }

    #[must_use]
    pub const fn is_bit_string(self) -> bool {
        matches!(self.category(), PrimitiveCategory::BitString)
    }

    #[must_use]
    pub const fn is_signed_integer(self) -> bool {
        matches!(self.category(), PrimitiveCategory::SignedInteger)
    }

    #[must_use]
    pub const fn is_unsigned_integer(self) -> bool {
        matches!(self.category(), PrimitiveCategory::UnsignedInteger)
    }

    #[must_use]
    pub const fn is_integer(self) -> bool {
        self.is_signed_integer() || self.is_unsigned_integer()
    }

    #[must_use]
    pub const fn is_numeric(self) -> bool {
        self.is_integer() || matches!(self.category(), PrimitiveCategory::FloatingPoint)
    }

    #[must_use]
    pub const fn declaration_is_valid(self) -> bool {
        !matches!(self, Self::String(255))
    }

    #[must_use]
    pub const fn type_id(self) -> PrimitiveTypeId {
        let signedness = if self.is_signed_integer() || matches!(self, Self::Time) {
            Signedness::Signed
        } else if self.is_unsigned_integer() || self.is_bit_string() || matches!(self, Self::Char) {
            Signedness::Unsigned
        } else {
            Signedness::NotApplicable
        };
        let range = match self {
            Self::Bool => PrimitiveRange::Boolean,
            Self::Sint => PrimitiveRange::Signed {
                minimum: i8::MIN as i64,
                maximum: i8::MAX as i64,
            },
            Self::Int => PrimitiveRange::Signed {
                minimum: i16::MIN as i64,
                maximum: i16::MAX as i64,
            },
            Self::Dint => PrimitiveRange::Signed {
                minimum: i32::MIN as i64,
                maximum: i32::MAX as i64,
            },
            Self::Lint | Self::Time => PrimitiveRange::Signed {
                minimum: i64::MIN,
                maximum: i64::MAX,
            },
            Self::Usint | Self::Byte | Self::Char => PrimitiveRange::Unsigned {
                minimum: 0,
                maximum: u8::MAX as u64,
            },
            Self::Uint | Self::Word => PrimitiveRange::Unsigned {
                minimum: 0,
                maximum: u16::MAX as u64,
            },
            Self::Udint | Self::Dword => PrimitiveRange::Unsigned {
                minimum: 0,
                maximum: u32::MAX as u64,
            },
            Self::Ulint | Self::Lword => PrimitiveRange::Unsigned {
                minimum: 0,
                maximum: u64::MAX,
            },
            Self::Real | Self::Lreal => PrimitiveRange::Ieee754,
            Self::String(capacity) => PrimitiveRange::StringLength {
                minimum: 0,
                maximum: capacity,
            },
        };
        let representation = match self {
            Self::Bool => "canonical boolean",
            Self::Sint | Self::Int | Self::Dint | Self::Lint => "fixed-width twos-complement",
            Self::Usint | Self::Uint | Self::Udint | Self::Ulint => "fixed-width unsigned binary",
            Self::Byte | Self::Word | Self::Dword | Self::Lword => "fixed-width uninterpreted bits",
            Self::Real => "IEEE-754 binary32; canonical quiet NaN",
            Self::Lreal => "IEEE-754 binary64; canonical quiet NaN",
            Self::Char => "unsigned 8-bit character code",
            Self::String(_) => "current length followed by bounded CHAR codes",
            Self::Time => "signed 64-bit virtual milliseconds",
        };
        let literal_rules = match self {
            Self::Bool => "TRUE or FALSE",
            Self::Sint | Self::Int | Self::Dint | Self::Lint => {
                "typed signed integer in declared range"
            }
            Self::Usint | Self::Uint | Self::Udint | Self::Ulint => {
                "typed nonnegative integer in declared range"
            }
            Self::Byte | Self::Word | Self::Dword | Self::Lword => {
                "typed bit-string literal in declared width"
            }
            Self::Real | Self::Lreal => "typed decimal or IEEE special value",
            Self::Char => "one CHAR code from 0 through 255",
            Self::String(_) => "sequence of CHAR codes; no wide characters",
            Self::Time => "typed signed duration in virtual milliseconds",
        };
        PrimitiveTypeId {
            stable_name: self.stable_id(),
            category: self.category(),
            width_bits: self.width_bits(),
            signedness,
            range,
            representation,
            literal_rules,
            string_capacity: if let Self::String(capacity) = self {
                Some(capacity)
            } else {
                None
            },
        }
    }

    #[must_use]
    pub fn canonical_default(self) -> ScalarValue {
        match self.category() {
            PrimitiveCategory::Boolean => ScalarValue::Bool(false),
            PrimitiveCategory::SignedInteger => ScalarValue::Signed(0),
            PrimitiveCategory::UnsignedInteger => ScalarValue::Unsigned(0),
            PrimitiveCategory::BitString => ScalarValue::BitString(0),
            PrimitiveCategory::FloatingPoint if self == Self::Real => {
                ScalarValue::Real(CanonicalF32::new(0.0))
            }
            PrimitiveCategory::FloatingPoint => ScalarValue::Lreal(CanonicalF64::new(0.0)),
            PrimitiveCategory::Character => ScalarValue::Char(0),
            PrimitiveCategory::String => ScalarValue::String(Vec::new()),
            PrimitiveCategory::Duration => ScalarValue::Time(0),
        }
    }

    /// Validates that a scalar uses this type's exact canonical representation.
    ///
    /// # Errors
    ///
    /// Returns an error for an invalid `STRING` capacity or a mismatched/out-of-range value.
    pub fn validate_scalar(self, value: &ScalarValue) -> Result<(), ScalarTypeError> {
        if !self.declaration_is_valid() {
            return Err(ScalarTypeError::InvalidStringCapacity);
        }
        let valid = match (self, value) {
            (Self::Bool, ScalarValue::Bool(_))
            | (Self::Real, ScalarValue::Real(_))
            | (Self::Lreal, ScalarValue::Lreal(_))
            | (Self::Char, ScalarValue::Char(_))
            | (Self::Time, ScalarValue::Time(_))
            | (Self::Lint, ScalarValue::Signed(_))
            | (Self::Ulint, ScalarValue::Unsigned(_))
            | (Self::Lword, ScalarValue::BitString(_)) => true,
            (Self::Sint, ScalarValue::Signed(value)) => i8::try_from(*value).is_ok(),
            (Self::Int, ScalarValue::Signed(value)) => i16::try_from(*value).is_ok(),
            (Self::Dint, ScalarValue::Signed(value)) => i32::try_from(*value).is_ok(),
            (Self::Usint, ScalarValue::Unsigned(value))
            | (Self::Byte, ScalarValue::BitString(value)) => u8::try_from(*value).is_ok(),
            (Self::Uint, ScalarValue::Unsigned(value))
            | (Self::Word, ScalarValue::BitString(value)) => u16::try_from(*value).is_ok(),
            (Self::Udint, ScalarValue::Unsigned(value))
            | (Self::Dword, ScalarValue::BitString(value)) => u32::try_from(*value).is_ok(),
            (Self::String(capacity), ScalarValue::String(value)) => {
                value.len() <= usize::from(capacity)
            }
            _ => false,
        };
        if valid {
            Ok(())
        } else {
            Err(ScalarTypeError::ValueDoesNotMatchType)
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ScalarValue {
    Bool(bool),
    Signed(i64),
    Unsigned(u64),
    BitString(u64),
    Real(CanonicalF32),
    Lreal(CanonicalF64),
    Char(u8),
    String(Vec<u8>),
    Time(i64),
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TypedScalar {
    data_type: PrimitiveType,
    value: ScalarValue,
}

impl TypedScalar {
    /// Constructs a scalar after validating its canonical representation.
    ///
    /// # Errors
    ///
    /// Returns an error when `value` is not valid for `data_type`.
    pub fn new(data_type: PrimitiveType, value: ScalarValue) -> Result<Self, ScalarTypeError> {
        data_type.validate_scalar(&value)?;
        Ok(Self { data_type, value })
    }

    #[must_use]
    pub fn canonical_default(data_type: PrimitiveType) -> Self {
        Self {
            data_type,
            value: data_type.canonical_default(),
        }
    }

    #[must_use]
    pub const fn data_type(&self) -> PrimitiveType {
        self.data_type
    }

    #[must_use]
    pub const fn value(&self) -> &ScalarValue {
        &self.value
    }

    #[must_use]
    pub fn into_value(self) -> ScalarValue {
        self.value
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScalarTypeError {
    InvalidStringCapacity,
    ValueDoesNotMatchType,
    TypeMismatch,
    UnsupportedOperation,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScalarFault {
    DivideByZero,
    ArithmeticOverflow,
    InvalidShiftCount,
    InvalidArgument,
    Conversion,
    Bounds,
    Type(ScalarTypeError),
}

impl From<ScalarTypeError> for ScalarFault {
    fn from(value: ScalarTypeError) -> Self {
        Self::Type(value)
    }
}

impl fmt::Display for ScalarFault {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NumericBinaryOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ComparisonOperator {
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BitBinaryOperator {
    And,
    Or,
    Xor,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShiftOperator {
    ShiftLeft,
    ShiftRight,
    RotateLeft,
    RotateRight,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RoundingOperator {
    Round,
    Trunc,
    Floor,
    Ceil,
}

fn require_same_type(
    left: &TypedScalar,
    right: &TypedScalar,
) -> Result<PrimitiveType, ScalarFault> {
    if left.data_type == right.data_type {
        Ok(left.data_type)
    } else {
        Err(ScalarTypeError::TypeMismatch.into())
    }
}

const fn width_mask(width: u8) -> u64 {
    if width == 64 {
        u64::MAX
    } else {
        (1_u64 << width) - 1
    }
}

const fn sign_extend(bits: u64, width: u8) -> i64 {
    if width == 64 {
        bits.cast_signed()
    } else {
        let mask = width_mask(width);
        let sign = 1_u64 << (width - 1);
        if bits & sign == 0 {
            bits.cast_signed()
        } else {
            (bits | !mask).cast_signed()
        }
    }
}

fn integer_bits(value: &TypedScalar) -> Result<u64, ScalarFault> {
    let width = value
        .data_type
        .width_bits()
        .ok_or(ScalarTypeError::UnsupportedOperation)?;
    match value.value {
        ScalarValue::Signed(value) => Ok(value.cast_unsigned() & width_mask(width)),
        ScalarValue::Unsigned(value) | ScalarValue::BitString(value) => {
            Ok(value & width_mask(width))
        }
        _ => Err(ScalarTypeError::ValueDoesNotMatchType.into()),
    }
}

fn integer_from_bits(data_type: PrimitiveType, bits: u64) -> Result<TypedScalar, ScalarFault> {
    let width = data_type
        .width_bits()
        .ok_or(ScalarTypeError::UnsupportedOperation)?;
    let bits = bits & width_mask(width);
    let value = if data_type.is_signed_integer() {
        ScalarValue::Signed(sign_extend(bits, width))
    } else if data_type.is_unsigned_integer() {
        ScalarValue::Unsigned(bits)
    } else if data_type.is_bit_string() {
        ScalarValue::BitString(bits)
    } else {
        return Err(ScalarTypeError::UnsupportedOperation.into());
    };
    TypedScalar::new(data_type, value).map_err(Into::into)
}

/// Applies canonical arithmetic to two equal-typed numeric scalars.
///
/// # Errors
///
/// Returns a deterministic fault for type mismatches, unsupported operations,
/// divide-by-zero, or the two signed division overflow cases.
#[allow(clippy::too_many_lines)]
pub fn numeric_binary(
    operator: NumericBinaryOperator,
    left: &TypedScalar,
    right: &TypedScalar,
) -> Result<TypedScalar, ScalarFault> {
    let data_type = require_same_type(left, right)?;
    if data_type.is_integer() {
        let width = data_type
            .width_bits()
            .ok_or(ScalarTypeError::UnsupportedOperation)?;
        let mask = width_mask(width);
        if data_type.is_signed_integer() {
            let (ScalarValue::Signed(left), ScalarValue::Signed(right)) =
                (&left.value, &right.value)
            else {
                return Err(ScalarTypeError::ValueDoesNotMatchType.into());
            };
            return match operator {
                NumericBinaryOperator::Add => integer_from_bits(
                    data_type,
                    left.cast_unsigned().wrapping_add(right.cast_unsigned()) & mask,
                ),
                NumericBinaryOperator::Subtract => integer_from_bits(
                    data_type,
                    left.cast_unsigned().wrapping_sub(right.cast_unsigned()) & mask,
                ),
                NumericBinaryOperator::Multiply => integer_from_bits(
                    data_type,
                    left.cast_unsigned().wrapping_mul(right.cast_unsigned()) & mask,
                ),
                NumericBinaryOperator::Divide => {
                    if *right == 0 {
                        return Err(ScalarFault::DivideByZero);
                    }
                    let result = left
                        .checked_div(*right)
                        .ok_or(ScalarFault::ArithmeticOverflow)?;
                    TypedScalar::new(data_type, ScalarValue::Signed(result))
                        .map_err(|_| ScalarFault::ArithmeticOverflow)
                }
                NumericBinaryOperator::Modulo => {
                    if *right == 0 {
                        return Err(ScalarFault::DivideByZero);
                    }
                    let PrimitiveRange::Signed { minimum, .. } = data_type.type_id().range else {
                        return Err(ScalarTypeError::UnsupportedOperation.into());
                    };
                    if *left == minimum && *right == -1 {
                        return Err(ScalarFault::ArithmeticOverflow);
                    }
                    let result = left
                        .checked_rem(*right)
                        .ok_or(ScalarFault::ArithmeticOverflow)?;
                    TypedScalar::new(data_type, ScalarValue::Signed(result)).map_err(Into::into)
                }
            };
        }
        let (ScalarValue::Unsigned(left), ScalarValue::Unsigned(right)) =
            (&left.value, &right.value)
        else {
            return Err(ScalarTypeError::ValueDoesNotMatchType.into());
        };
        return match operator {
            NumericBinaryOperator::Add => {
                integer_from_bits(data_type, left.wrapping_add(*right) & mask)
            }
            NumericBinaryOperator::Subtract => {
                integer_from_bits(data_type, left.wrapping_sub(*right) & mask)
            }
            NumericBinaryOperator::Multiply => {
                integer_from_bits(data_type, left.wrapping_mul(*right) & mask)
            }
            NumericBinaryOperator::Divide => {
                if *right == 0 {
                    Err(ScalarFault::DivideByZero)
                } else {
                    integer_from_bits(data_type, left / right)
                }
            }
            NumericBinaryOperator::Modulo => {
                if *right == 0 {
                    Err(ScalarFault::DivideByZero)
                } else {
                    integer_from_bits(data_type, left % right)
                }
            }
        };
    }
    match (data_type, &left.value, &right.value) {
        (PrimitiveType::Real, ScalarValue::Real(left), ScalarValue::Real(right)) => {
            if matches!(operator, NumericBinaryOperator::Modulo) {
                return Err(ScalarTypeError::UnsupportedOperation.into());
            }
            if matches!(operator, NumericBinaryOperator::Divide) && right.get() == 0.0 {
                return Err(ScalarFault::DivideByZero);
            }
            let value = match operator {
                NumericBinaryOperator::Add => left.get() + right.get(),
                NumericBinaryOperator::Subtract => left.get() - right.get(),
                NumericBinaryOperator::Multiply => left.get() * right.get(),
                NumericBinaryOperator::Divide => left.get() / right.get(),
                NumericBinaryOperator::Modulo => unreachable!(),
            };
            Ok(TypedScalar {
                data_type,
                value: ScalarValue::Real(CanonicalF32::new(value)),
            })
        }
        (PrimitiveType::Lreal, ScalarValue::Lreal(left), ScalarValue::Lreal(right)) => {
            if matches!(operator, NumericBinaryOperator::Modulo) {
                return Err(ScalarTypeError::UnsupportedOperation.into());
            }
            if matches!(operator, NumericBinaryOperator::Divide) && right.get() == 0.0 {
                return Err(ScalarFault::DivideByZero);
            }
            let value = match operator {
                NumericBinaryOperator::Add => left.get() + right.get(),
                NumericBinaryOperator::Subtract => left.get() - right.get(),
                NumericBinaryOperator::Multiply => left.get() * right.get(),
                NumericBinaryOperator::Divide => left.get() / right.get(),
                NumericBinaryOperator::Modulo => unreachable!(),
            };
            Ok(TypedScalar {
                data_type,
                value: ScalarValue::Lreal(CanonicalF64::new(value)),
            })
        }
        _ => Err(ScalarTypeError::UnsupportedOperation.into()),
    }
}

/// Negates a signed integer or floating-point scalar.
///
/// # Errors
///
/// Returns an error when the operand is not a supported canonical scalar.
pub fn negate(value: &TypedScalar) -> Result<TypedScalar, ScalarFault> {
    if value.data_type.is_signed_integer() {
        let width = value
            .data_type
            .width_bits()
            .ok_or(ScalarTypeError::UnsupportedOperation)?;
        return integer_from_bits(
            value.data_type,
            0_u64.wrapping_sub(integer_bits(value)?) & width_mask(width),
        );
    }
    match value.value {
        ScalarValue::Real(value_bits) if value.data_type == PrimitiveType::Real => {
            Ok(TypedScalar {
                data_type: value.data_type,
                value: ScalarValue::Real(CanonicalF32::from_bits(value_bits.bits() ^ (1 << 31))),
            })
        }
        ScalarValue::Lreal(value_bits) if value.data_type == PrimitiveType::Lreal => {
            Ok(TypedScalar {
                data_type: value.data_type,
                value: ScalarValue::Lreal(CanonicalF64::from_bits(
                    value_bits.bits() ^ (1_u64 << 63),
                )),
            })
        }
        _ => Err(ScalarTypeError::UnsupportedOperation.into()),
    }
}

/// Computes canonical absolute value without silently overflowing signed minima.
///
/// # Errors
///
/// Returns `ArithmeticOverflow` for a signed type's minimum value and a type
/// fault for unsupported operands.
pub fn absolute(value: &TypedScalar) -> Result<TypedScalar, ScalarFault> {
    match value.value {
        ScalarValue::Signed(raw) if value.data_type.is_signed_integer() => {
            let PrimitiveRange::Signed { minimum, .. } = value.data_type.type_id().range else {
                return Err(ScalarTypeError::UnsupportedOperation.into());
            };
            if raw == minimum {
                return Err(ScalarFault::ArithmeticOverflow);
            }
            TypedScalar::new(value.data_type, ScalarValue::Signed(raw.abs())).map_err(Into::into)
        }
        ScalarValue::Real(raw) if value.data_type == PrimitiveType::Real => Ok(TypedScalar {
            data_type: value.data_type,
            value: ScalarValue::Real(CanonicalF32::from_bits(raw.bits() & 0x7fff_ffff)),
        }),
        ScalarValue::Lreal(raw) if value.data_type == PrimitiveType::Lreal => Ok(TypedScalar {
            data_type: value.data_type,
            value: ScalarValue::Lreal(CanonicalF64::from_bits(raw.bits() & 0x7fff_ffff_ffff_ffff)),
        }),
        _ => Err(ScalarTypeError::UnsupportedOperation.into()),
    }
}

fn float_has_nan(left: &TypedScalar, right: &TypedScalar) -> bool {
    matches!((&left.value, &right.value), (ScalarValue::Real(a), ScalarValue::Real(b)) if a.get().is_nan() || b.get().is_nan())
        || matches!((&left.value, &right.value), (ScalarValue::Lreal(a), ScalarValue::Lreal(b)) if a.get().is_nan() || b.get().is_nan())
}

/// Compares two scalars under the canonical PLC comparison rules.
///
/// # Errors
///
/// Returns an error for mismatched types, invalid representations, or ordered
/// comparison of a type that only supports equality.
pub fn compare(
    operator: ComparisonOperator,
    left: &TypedScalar,
    right: &TypedScalar,
) -> Result<bool, ScalarFault> {
    let data_type = require_same_type(left, right)?;
    if float_has_nan(left, right) {
        return Ok(matches!(operator, ComparisonOperator::NotEqual));
    }
    let ordering = match (&left.value, &right.value) {
        (ScalarValue::Bool(left), ScalarValue::Bool(right)) => left.cmp(right),
        (ScalarValue::Signed(left), ScalarValue::Signed(right))
        | (ScalarValue::Time(left), ScalarValue::Time(right)) => left.cmp(right),
        (ScalarValue::Unsigned(left), ScalarValue::Unsigned(right)) => left.cmp(right),
        (ScalarValue::BitString(left), ScalarValue::BitString(right)) => {
            if !matches!(
                operator,
                ComparisonOperator::Equal | ComparisonOperator::NotEqual
            ) {
                return Err(ScalarTypeError::UnsupportedOperation.into());
            }
            left.cmp(right)
        }
        (ScalarValue::Real(left), ScalarValue::Real(right)) => left
            .get()
            .partial_cmp(&right.get())
            .ok_or(ScalarFault::InvalidArgument)?,
        (ScalarValue::Lreal(left), ScalarValue::Lreal(right)) => left
            .get()
            .partial_cmp(&right.get())
            .ok_or(ScalarFault::InvalidArgument)?,
        (ScalarValue::Char(left), ScalarValue::Char(right)) => left.cmp(right),
        (ScalarValue::String(left), ScalarValue::String(right)) => left.cmp(right),
        _ => return Err(ScalarTypeError::ValueDoesNotMatchType.into()),
    };
    if data_type == PrimitiveType::Bool
        && !matches!(
            operator,
            ComparisonOperator::Equal | ComparisonOperator::NotEqual
        )
    {
        return Err(ScalarTypeError::UnsupportedOperation.into());
    }
    Ok(match operator {
        ComparisonOperator::Equal => ordering == Ordering::Equal,
        ComparisonOperator::NotEqual => ordering != Ordering::Equal,
        ComparisonOperator::Less => ordering == Ordering::Less,
        ComparisonOperator::LessEqual => ordering != Ordering::Greater,
        ComparisonOperator::Greater => ordering == Ordering::Greater,
        ComparisonOperator::GreaterEqual => ordering != Ordering::Less,
    })
}

/// Returns the canonical minimum of two equal-typed numeric scalars.
///
/// # Errors
///
/// Returns an error for mismatched, invalid, or nonnumeric operands.
pub fn minimum(left: &TypedScalar, right: &TypedScalar) -> Result<TypedScalar, ScalarFault> {
    min_max(left, right, true)
}

/// Returns the canonical maximum of two equal-typed numeric scalars.
///
/// # Errors
///
/// Returns an error for mismatched, invalid, or nonnumeric operands.
pub fn maximum(left: &TypedScalar, right: &TypedScalar) -> Result<TypedScalar, ScalarFault> {
    min_max(left, right, false)
}

fn min_max(
    left: &TypedScalar,
    right: &TypedScalar,
    minimum: bool,
) -> Result<TypedScalar, ScalarFault> {
    let data_type = require_same_type(left, right)?;
    if !data_type.is_numeric() {
        return Err(ScalarTypeError::UnsupportedOperation.into());
    }
    if float_has_nan(left, right) {
        return Ok(TypedScalar::new(
            data_type,
            match data_type {
                PrimitiveType::Real => ScalarValue::Real(CanonicalF32::new(f32::NAN)),
                PrimitiveType::Lreal => ScalarValue::Lreal(CanonicalF64::new(f64::NAN)),
                _ => unreachable!(),
            },
        )?);
    }
    if let (ScalarValue::Real(a), ScalarValue::Real(b)) = (&left.value, &right.value)
        && a.get() == 0.0
        && b.get() == 0.0
    {
        let sign = if minimum {
            a.bits() | b.bits()
        } else {
            a.bits() & b.bits()
        };
        return TypedScalar::new(data_type, ScalarValue::Real(CanonicalF32::from_bits(sign)))
            .map_err(Into::into);
    }
    if let (ScalarValue::Lreal(a), ScalarValue::Lreal(b)) = (&left.value, &right.value)
        && a.get() == 0.0
        && b.get() == 0.0
    {
        let sign = if minimum {
            a.bits() | b.bits()
        } else {
            a.bits() & b.bits()
        };
        return TypedScalar::new(data_type, ScalarValue::Lreal(CanonicalF64::from_bits(sign)))
            .map_err(Into::into);
    }
    let take_left = compare(
        if minimum {
            ComparisonOperator::LessEqual
        } else {
            ComparisonOperator::GreaterEqual
        },
        left,
        right,
    )?;
    Ok(if take_left {
        left.clone()
    } else {
        right.clone()
    })
}

/// Clamps `input` to the inclusive canonical range `minimum..=maximum`.
///
/// # Errors
///
/// Returns an error for type mismatches, unsupported comparisons, or a minimum
/// greater than the maximum.
pub fn limit(
    minimum: &TypedScalar,
    input: &TypedScalar,
    maximum: &TypedScalar,
) -> Result<TypedScalar, ScalarFault> {
    require_same_type(minimum, input)?;
    require_same_type(input, maximum)?;
    if float_has_nan(minimum, input)
        || float_has_nan(input, maximum)
        || float_has_nan(minimum, maximum)
    {
        return Ok(TypedScalar::new(
            input.data_type,
            match input.data_type {
                PrimitiveType::Real => ScalarValue::Real(CanonicalF32::new(f32::NAN)),
                PrimitiveType::Lreal => ScalarValue::Lreal(CanonicalF64::new(f64::NAN)),
                _ => return Err(ScalarTypeError::UnsupportedOperation.into()),
            },
        )?);
    }
    if compare(ComparisonOperator::Greater, minimum, maximum)? {
        return Err(ScalarFault::InvalidArgument);
    }
    if compare(ComparisonOperator::Less, input, minimum)? {
        return Ok(minimum.clone());
    }
    if compare(ComparisonOperator::Greater, input, maximum)? {
        return Ok(maximum.clone());
    }
    Ok(input.clone())
}

/// Applies a bitwise binary operation to two equal-width bit strings.
///
/// # Errors
///
/// Returns an error for mismatched, invalid, or non-bit-string operands.
pub fn bit_binary(
    operator: BitBinaryOperator,
    left: &TypedScalar,
    right: &TypedScalar,
) -> Result<TypedScalar, ScalarFault> {
    let data_type = require_same_type(left, right)?;
    if !data_type.is_bit_string() {
        return Err(ScalarTypeError::UnsupportedOperation.into());
    }
    let left = integer_bits(left)?;
    let right = integer_bits(right)?;
    integer_from_bits(
        data_type,
        match operator {
            BitBinaryOperator::And => left & right,
            BitBinaryOperator::Or => left | right,
            BitBinaryOperator::Xor => left ^ right,
        },
    )
}

/// Inverts every bit within a bit string's declared width.
///
/// # Errors
///
/// Returns an error when the operand is not a canonical bit string.
pub fn bit_not(value: &TypedScalar) -> Result<TypedScalar, ScalarFault> {
    if !value.data_type.is_bit_string() {
        return Err(ScalarTypeError::UnsupportedOperation.into());
    }
    integer_from_bits(value.data_type, !integer_bits(value)?)
}

/// Shifts or rotates a bit string within its declared fixed width.
///
/// # Errors
///
/// Returns `InvalidShiftCount` when `count` is outside `0..width`, or a type
/// fault when the operand is not a canonical bit string.
pub fn shift_rotate(
    operator: ShiftOperator,
    value: &TypedScalar,
    count: u16,
) -> Result<TypedScalar, ScalarFault> {
    if !value.data_type.is_bit_string() {
        return Err(ScalarTypeError::UnsupportedOperation.into());
    }
    let width = value
        .data_type
        .width_bits()
        .ok_or(ScalarTypeError::UnsupportedOperation)?;
    let count = u8::try_from(count).map_err(|_| ScalarFault::InvalidShiftCount)?;
    if count >= width {
        return Err(ScalarFault::InvalidShiftCount);
    }
    let bits = integer_bits(value)?;
    let mask = width_mask(width);
    let result = match operator {
        ShiftOperator::ShiftLeft => (bits << count) & mask,
        ShiftOperator::ShiftRight => bits >> count,
        ShiftOperator::RotateLeft | ShiftOperator::RotateRight if count == 0 => bits,
        ShiftOperator::RotateLeft => ((bits << count) | (bits >> (width - count))) & mask,
        ShiftOperator::RotateRight => ((bits >> count) | (bits << (width - count))) & mask,
    };
    integer_from_bits(value.data_type, result)
}

/// Performs an explicit conversion from one canonical scalar type to another.
///
/// Integer narrowing and signedness changes use fixed-width modulo semantics;
/// float conversions follow IEEE-754 precision loss and NaN canonicalization.
///
/// # Errors
///
/// Returns `Conversion` for unsupported conversions and a type fault for an
/// invalid source representation.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    reason = "IEC explicit numeric conversions intentionally use IEEE and fixed-width casts"
)]
pub fn convert(
    value: &TypedScalar,
    destination: PrimitiveType,
) -> Result<TypedScalar, ScalarFault> {
    let source = value.data_type;
    if source == destination {
        return Ok(value.clone());
    }
    if source.is_integer() && destination.is_integer() {
        let mathematical_bits = match value.value {
            ScalarValue::Signed(raw) => raw.cast_unsigned(),
            ScalarValue::Unsigned(raw) => raw,
            _ => return Err(ScalarTypeError::ValueDoesNotMatchType.into()),
        };
        return integer_from_bits(destination, mathematical_bits);
    }
    if source.is_integer() && matches!(destination, PrimitiveType::Real | PrimitiveType::Lreal) {
        let as_f64 = match value.value {
            ScalarValue::Signed(raw) => raw as f64,
            ScalarValue::Unsigned(raw) => raw as f64,
            _ => return Err(ScalarTypeError::ValueDoesNotMatchType.into()),
        };
        return TypedScalar::new(
            destination,
            if destination == PrimitiveType::Real {
                ScalarValue::Real(CanonicalF32::new(as_f64 as f32))
            } else {
                ScalarValue::Lreal(CanonicalF64::new(as_f64))
            },
        )
        .map_err(Into::into);
    }
    match (&value.value, source, destination) {
        (ScalarValue::Real(raw), PrimitiveType::Real, PrimitiveType::Lreal) => TypedScalar::new(
            destination,
            ScalarValue::Lreal(CanonicalF64::new(f64::from(raw.get()))),
        )
        .map_err(Into::into),
        (ScalarValue::Lreal(raw), PrimitiveType::Lreal, PrimitiveType::Real) => TypedScalar::new(
            destination,
            ScalarValue::Real(CanonicalF32::new(raw.get() as f32)),
        )
        .map_err(Into::into),
        (ScalarValue::Char(raw), PrimitiveType::Char, PrimitiveType::Usint) => {
            TypedScalar::new(destination, ScalarValue::Unsigned(u64::from(*raw)))
                .map_err(Into::into)
        }
        (ScalarValue::Unsigned(raw), PrimitiveType::Usint, PrimitiveType::Char) => {
            TypedScalar::new(
                destination,
                ScalarValue::Char(u8::try_from(*raw).map_err(|_| ScalarFault::Conversion)?),
            )
            .map_err(Into::into)
        }
        (ScalarValue::Time(raw), PrimitiveType::Time, PrimitiveType::Lint) => {
            TypedScalar::new(destination, ScalarValue::Signed(*raw)).map_err(Into::into)
        }
        (ScalarValue::Signed(raw), PrimitiveType::Lint, PrimitiveType::Time) => {
            TypedScalar::new(destination, ScalarValue::Time(*raw)).map_err(Into::into)
        }
        _ if source.is_integer()
            && destination.is_bit_string()
            && source.width_bits() == destination.width_bits() =>
        {
            integer_from_bits(destination, integer_bits(value)?)
        }
        (ScalarValue::BitString(_), _, _)
            if source.is_bit_string()
                && destination.is_integer()
                && source.width_bits() == destination.width_bits() =>
        {
            integer_from_bits(destination, integer_bits(value)?)
        }
        _ => Err(ScalarFault::Conversion),
    }
}

#[must_use]
pub fn explicit_conversion_allowed(source: PrimitiveType, destination: PrimitiveType) -> bool {
    if source == destination {
        return true;
    }
    (source.is_integer() && destination.is_integer())
        || (source.is_integer()
            && matches!(destination, PrimitiveType::Real | PrimitiveType::Lreal))
        || matches!(
            (source, destination),
            (PrimitiveType::Real, PrimitiveType::Lreal)
                | (PrimitiveType::Lreal, PrimitiveType::Real)
                | (PrimitiveType::Char, PrimitiveType::Usint)
                | (PrimitiveType::Usint, PrimitiveType::Char)
                | (PrimitiveType::Time, PrimitiveType::Lint)
                | (PrimitiveType::Lint, PrimitiveType::Time)
        )
        || ((source.is_integer() && destination.is_bit_string())
            || (source.is_bit_string() && destination.is_integer()))
            && source.width_bits() == destination.width_bits()
}

#[must_use]
pub fn implicit_conversion_allowed(source: PrimitiveType, destination: PrimitiveType) -> bool {
    if source == destination
        || matches!(
            (source, destination),
            (PrimitiveType::Real, PrimitiveType::Lreal)
        )
    {
        return true;
    }
    let Some(source_width) = source.width_bits() else {
        return false;
    };
    let Some(destination_width) = destination.width_bits() else {
        return false;
    };
    if source.is_signed_integer() && destination.is_signed_integer() {
        return source_width < destination_width;
    }
    if source.is_unsigned_integer() && destination.is_unsigned_integer() {
        return source_width < destination_width;
    }
    source.is_unsigned_integer()
        && destination.is_signed_integer()
        && source_width < destination_width
}

/// Converts a finite floating-point value to an integer using the selected rule.
///
/// # Errors
///
/// Returns `Conversion` for a non-float source, non-integer destination,
/// non-finite input, or rounded result outside the destination range.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "range and finiteness checks precede the final integral float cast"
)]
pub fn round_to_integer(
    value: &TypedScalar,
    destination: PrimitiveType,
    operator: RoundingOperator,
) -> Result<TypedScalar, ScalarFault> {
    if !destination.is_integer() {
        return Err(ScalarFault::Conversion);
    }
    let raw = match value.value {
        ScalarValue::Real(raw) if value.data_type == PrimitiveType::Real => f64::from(raw.get()),
        ScalarValue::Lreal(raw) if value.data_type == PrimitiveType::Lreal => raw.get(),
        _ => return Err(ScalarFault::Conversion),
    };
    if !raw.is_finite() {
        return Err(ScalarFault::Conversion);
    }
    let rounded = match operator {
        RoundingOperator::Round => raw.round_ties_even(),
        RoundingOperator::Trunc => raw.trunc(),
        RoundingOperator::Floor => raw.floor(),
        RoundingOperator::Ceil => raw.ceil(),
    };
    let width = destination
        .width_bits()
        .ok_or(ScalarTypeError::UnsupportedOperation)?;
    if destination.is_signed_integer() {
        let lower = -(2_f64.powi(i32::from(width) - 1));
        let upper_exclusive = 2_f64.powi(i32::from(width) - 1);
        if rounded < lower || rounded >= upper_exclusive {
            return Err(ScalarFault::Conversion);
        }
        TypedScalar::new(destination, ScalarValue::Signed(rounded as i64)).map_err(Into::into)
    } else {
        let upper_exclusive = 2_f64.powi(i32::from(width));
        if rounded < 0.0 || rounded >= upper_exclusive {
            return Err(ScalarFault::Conversion);
        }
        TypedScalar::new(destination, ScalarValue::Unsigned(rounded as u64)).map_err(Into::into)
    }
}

/// Assigns a scalar while enforcing destination bounds atomically.
///
/// # Errors
///
/// Returns `Bounds` when a string exceeds its destination capacity, or a type
/// fault when no exact assignment rule exists.
pub fn assign_to(
    value: &TypedScalar,
    destination: PrimitiveType,
) -> Result<TypedScalar, ScalarFault> {
    if value.data_type == destination {
        return Ok(value.clone());
    }
    match (&value.value, value.data_type, destination) {
        (ScalarValue::String(bytes), PrimitiveType::String(_), PrimitiveType::String(capacity)) => {
            if bytes.len() > usize::from(capacity) {
                return Err(ScalarFault::Bounds);
            }
            TypedScalar::new(destination, ScalarValue::String(bytes.clone())).map_err(Into::into)
        }
        _ => Err(ScalarTypeError::TypeMismatch.into()),
    }
}

#[must_use]
pub fn primitive_type_name(data_type: PrimitiveType) -> String {
    match data_type {
        PrimitiveType::String(capacity) => alloc::format!("STRING[{capacity}]"),
        _ => String::from(data_type.stable_id()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scalar(data_type: PrimitiveType, value: ScalarValue) -> TypedScalar {
        TypedScalar::new(data_type, value).unwrap()
    }

    #[test]
    fn table_has_exact_widths_ranges_defaults_and_distinct_categories() {
        let cases = [
            (
                PrimitiveType::Sint,
                8,
                i64::from(i8::MIN),
                i64::from(i8::MAX),
            ),
            (
                PrimitiveType::Int,
                16,
                i64::from(i16::MIN),
                i64::from(i16::MAX),
            ),
            (
                PrimitiveType::Dint,
                32,
                i64::from(i32::MIN),
                i64::from(i32::MAX),
            ),
            (PrimitiveType::Lint, 64, i64::MIN, i64::MAX),
        ];
        for (data_type, width, minimum, maximum) in cases {
            assert_eq!(data_type.width_bits(), Some(width));
            assert_eq!(
                data_type.type_id().range,
                PrimitiveRange::Signed { minimum, maximum }
            );
            assert_eq!(data_type.canonical_default(), ScalarValue::Signed(0));
        }
        assert_ne!(
            PrimitiveType::Byte.type_id().category,
            PrimitiveType::Usint.type_id().category
        );
        assert_eq!(
            PrimitiveType::Time.type_id().range,
            PrimitiveRange::Signed {
                minimum: i64::MIN,
                maximum: i64::MAX
            }
        );
        assert!(!PrimitiveType::String(255).declaration_is_valid());
        assert!(PrimitiveType::String(DEFAULT_STRING_CAPACITY).declaration_is_valid());
    }

    #[test]
    fn fixed_width_integer_arithmetic_wraps_and_faults_only_where_required() {
        let max = scalar(PrimitiveType::Sint, ScalarValue::Signed(127));
        let one = scalar(PrimitiveType::Sint, ScalarValue::Signed(1));
        assert_eq!(
            numeric_binary(NumericBinaryOperator::Add, &max, &one)
                .unwrap()
                .value(),
            &ScalarValue::Signed(-128)
        );
        assert_eq!(
            negate(&scalar(PrimitiveType::Sint, ScalarValue::Signed(-128)))
                .unwrap()
                .value(),
            &ScalarValue::Signed(-128)
        );
        assert_eq!(
            absolute(&scalar(PrimitiveType::Sint, ScalarValue::Signed(-128))),
            Err(ScalarFault::ArithmeticOverflow)
        );
        let minus_one = scalar(PrimitiveType::Sint, ScalarValue::Signed(-1));
        assert_eq!(
            numeric_binary(
                NumericBinaryOperator::Divide,
                &scalar(PrimitiveType::Sint, ScalarValue::Signed(-128)),
                &minus_one
            ),
            Err(ScalarFault::ArithmeticOverflow)
        );
        assert_eq!(
            numeric_binary(
                NumericBinaryOperator::Modulo,
                &scalar(PrimitiveType::Dint, ScalarValue::Signed(-5)),
                &scalar(PrimitiveType::Dint, ScalarValue::Signed(2))
            )
            .unwrap()
            .value(),
            &ScalarValue::Signed(-1)
        );
        assert_eq!(
            numeric_binary(
                NumericBinaryOperator::Divide,
                &one,
                &scalar(PrimitiveType::Sint, ScalarValue::Signed(0))
            ),
            Err(ScalarFault::DivideByZero)
        );
    }

    #[test]
    fn floating_semantics_canonicalize_nan_and_preserve_signed_zero() {
        let nan32 = CanonicalF32::from_bits(0x7fa1_2345);
        assert_eq!(nan32.bits(), CanonicalF32::QUIET_NAN_BITS);
        let nan64 = CanonicalF64::from_bits(0x7ff0_0000_0000_0001);
        assert_eq!(nan64.bits(), CanonicalF64::QUIET_NAN_BITS);
        let negative_zero = scalar(
            PrimitiveType::Real,
            ScalarValue::Real(CanonicalF32::new(-0.0)),
        );
        let positive_zero = scalar(
            PrimitiveType::Real,
            ScalarValue::Real(CanonicalF32::new(0.0)),
        );
        assert!(compare(ComparisonOperator::Equal, &negative_zero, &positive_zero).unwrap());
        assert_eq!(
            minimum(&negative_zero, &positive_zero).unwrap().value(),
            &ScalarValue::Real(CanonicalF32::new(-0.0))
        );
        assert_eq!(
            maximum(&negative_zero, &positive_zero).unwrap().value(),
            &ScalarValue::Real(CanonicalF32::new(0.0))
        );
        let nan = scalar(PrimitiveType::Real, ScalarValue::Real(nan32));
        assert!(!compare(ComparisonOperator::Equal, &nan, &nan).unwrap());
        assert!(compare(ComparisonOperator::NotEqual, &nan, &nan).unwrap());
        assert_eq!(
            minimum(&nan, &positive_zero).unwrap().value(),
            &ScalarValue::Real(CanonicalF32::new(f32::NAN))
        );
    }

    #[test]
    fn conversions_are_exhaustive_and_modulo_or_bit_preserving() {
        let minus_one = scalar(PrimitiveType::Sint, ScalarValue::Signed(-1));
        assert_eq!(
            convert(&minus_one, PrimitiveType::Uint).unwrap().value(),
            &ScalarValue::Unsigned(u64::from(u16::MAX))
        );
        let bits = convert(
            &scalar(PrimitiveType::Dint, ScalarValue::Signed(-1)),
            PrimitiveType::Dword,
        )
        .unwrap();
        assert_eq!(bits.value(), &ScalarValue::BitString(u64::from(u32::MAX)));
        assert_eq!(
            convert(&bits, PrimitiveType::Dint).unwrap().value(),
            &ScalarValue::Signed(-1)
        );
        assert_eq!(
            convert(
                &scalar(PrimitiveType::Char, ScalarValue::Char(255)),
                PrimitiveType::Usint
            )
            .unwrap()
            .value(),
            &ScalarValue::Unsigned(255)
        );
        assert_eq!(
            convert(
                &scalar(PrimitiveType::Time, ScalarValue::Time(-25)),
                PrimitiveType::Lint
            )
            .unwrap()
            .value(),
            &ScalarValue::Signed(-25)
        );
        assert_eq!(
            convert(
                &scalar(PrimitiveType::Bool, ScalarValue::Bool(true)),
                PrimitiveType::Usint
            ),
            Err(ScalarFault::Conversion)
        );
    }

    #[test]
    fn shifts_strings_rounding_and_limit_are_bounded_and_atomic() {
        let byte = scalar(PrimitiveType::Byte, ScalarValue::BitString(0x81));
        assert_eq!(
            shift_rotate(ShiftOperator::RotateLeft, &byte, 1)
                .unwrap()
                .value(),
            &ScalarValue::BitString(0x03)
        );
        assert_eq!(
            shift_rotate(ShiftOperator::ShiftLeft, &byte, 8),
            Err(ScalarFault::InvalidShiftCount)
        );
        let text = scalar(
            PrimitiveType::String(3),
            ScalarValue::String(vec![65, 66, 67]),
        );
        assert_eq!(
            assign_to(&text, PrimitiveType::String(2)),
            Err(ScalarFault::Bounds)
        );
        let real = scalar(
            PrimitiveType::Lreal,
            ScalarValue::Lreal(CanonicalF64::new(2.5)),
        );
        assert_eq!(
            round_to_integer(&real, PrimitiveType::Dint, RoundingOperator::Round)
                .unwrap()
                .value(),
            &ScalarValue::Signed(2)
        );
        let min = scalar(PrimitiveType::Dint, ScalarValue::Signed(10));
        let input = scalar(PrimitiveType::Dint, ScalarValue::Signed(5));
        let max = scalar(PrimitiveType::Dint, ScalarValue::Signed(20));
        assert_eq!(limit(&min, &input, &max).unwrap(), min);
        assert_eq!(limit(&max, &input, &min), Err(ScalarFault::InvalidArgument));
    }
}
