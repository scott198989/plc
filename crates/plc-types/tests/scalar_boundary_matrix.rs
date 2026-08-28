use plc_types::{
    BitBinaryOperator, CanonicalF32, CanonicalF64, ComparisonOperator, NumericBinaryOperator,
    PrimitiveType, RoundingOperator, ScalarFault, ScalarTypeError, ScalarValue, ShiftOperator,
    TypedScalar, absolute, assign_to, bit_binary, bit_not, compare, convert,
    explicit_conversion_allowed, implicit_conversion_allowed, maximum, minimum, negate,
    numeric_binary, round_to_integer, shift_rotate,
};

fn scalar(data_type: PrimitiveType, value: ScalarValue) -> TypedScalar {
    TypedScalar::new(data_type, value).expect("fixed oracle value must match its PLC type")
}

fn signed(data_type: PrimitiveType, value: i64) -> TypedScalar {
    scalar(data_type, ScalarValue::Signed(value))
}

fn unsigned(data_type: PrimitiveType, value: u64) -> TypedScalar {
    scalar(data_type, ScalarValue::Unsigned(value))
}

fn real(value: f32) -> TypedScalar {
    scalar(
        PrimitiveType::Real,
        ScalarValue::Real(CanonicalF32::new(value)),
    )
}

fn lreal(value: f64) -> TypedScalar {
    scalar(
        PrimitiveType::Lreal,
        ScalarValue::Lreal(CanonicalF64::new(value)),
    )
}

fn representative(data_type: PrimitiveType) -> TypedScalar {
    let value = match data_type {
        PrimitiveType::Bool => ScalarValue::Bool(true),
        PrimitiveType::Sint | PrimitiveType::Int | PrimitiveType::Dint | PrimitiveType::Lint => {
            ScalarValue::Signed(-1)
        }
        PrimitiveType::Usint
        | PrimitiveType::Uint
        | PrimitiveType::Udint
        | PrimitiveType::Ulint => ScalarValue::Unsigned(1),
        PrimitiveType::Byte | PrimitiveType::Word | PrimitiveType::Dword | PrimitiveType::Lword => {
            ScalarValue::BitString(1)
        }
        PrimitiveType::Real => ScalarValue::Real(CanonicalF32::new(-0.0)),
        PrimitiveType::Lreal => ScalarValue::Lreal(CanonicalF64::new(-0.0)),
        PrimitiveType::Char => ScalarValue::Char(u8::MAX),
        PrimitiveType::String(_) => ScalarValue::String(Vec::new()),
        PrimitiveType::Time => ScalarValue::Time(-1),
    };
    scalar(data_type, value)
}

fn expected_explicit_conversion(source: PrimitiveType, destination: PrimitiveType) -> bool {
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
        || (((source.is_integer() && destination.is_bit_string())
            || (source.is_bit_string() && destination.is_integer()))
            && source.width_bits() == destination.width_bits())
}

#[test]
fn every_primitive_boundary_and_default_is_exact_and_host_independent() {
    let signed_cases = [
        (PrimitiveType::Sint, i64::from(i8::MIN), i64::from(i8::MAX)),
        (PrimitiveType::Int, i64::from(i16::MIN), i64::from(i16::MAX)),
        (
            PrimitiveType::Dint,
            i64::from(i32::MIN),
            i64::from(i32::MAX),
        ),
        (PrimitiveType::Lint, i64::MIN, i64::MAX),
    ];
    for (data_type, minimum, maximum) in signed_cases {
        assert!(TypedScalar::new(data_type, ScalarValue::Signed(minimum)).is_ok());
        assert!(TypedScalar::new(data_type, ScalarValue::Signed(maximum)).is_ok());
        assert_eq!(
            TypedScalar::canonical_default(data_type).value(),
            &ScalarValue::Signed(0)
        );
    }
    assert_eq!(
        TypedScalar::new(
            PrimitiveType::Sint,
            ScalarValue::Signed(i64::from(i8::MIN) - 1)
        ),
        Err(ScalarTypeError::ValueDoesNotMatchType)
    );
    assert_eq!(
        TypedScalar::new(
            PrimitiveType::Dint,
            ScalarValue::Signed(i64::from(i32::MAX) + 1)
        ),
        Err(ScalarTypeError::ValueDoesNotMatchType)
    );

    let unsigned_cases = [
        (PrimitiveType::Usint, u64::from(u8::MAX)),
        (PrimitiveType::Uint, u64::from(u16::MAX)),
        (PrimitiveType::Udint, u64::from(u32::MAX)),
        (PrimitiveType::Ulint, u64::MAX),
    ];
    for (data_type, maximum) in unsigned_cases {
        assert!(TypedScalar::new(data_type, ScalarValue::Unsigned(0)).is_ok());
        assert!(TypedScalar::new(data_type, ScalarValue::Unsigned(maximum)).is_ok());
        assert_eq!(
            TypedScalar::canonical_default(data_type).value(),
            &ScalarValue::Unsigned(0)
        );
    }
    assert_eq!(
        TypedScalar::new(
            PrimitiveType::Udint,
            ScalarValue::Unsigned(u64::from(u32::MAX) + 1)
        ),
        Err(ScalarTypeError::ValueDoesNotMatchType)
    );

    let bit_cases = [
        (PrimitiveType::Byte, u64::from(u8::MAX)),
        (PrimitiveType::Word, u64::from(u16::MAX)),
        (PrimitiveType::Dword, u64::from(u32::MAX)),
        (PrimitiveType::Lword, u64::MAX),
    ];
    for (data_type, maximum) in bit_cases {
        assert!(TypedScalar::new(data_type, ScalarValue::BitString(0)).is_ok());
        assert!(TypedScalar::new(data_type, ScalarValue::BitString(maximum)).is_ok());
        assert_eq!(
            TypedScalar::canonical_default(data_type).value(),
            &ScalarValue::BitString(0)
        );
    }
    assert_ne!(
        PrimitiveType::Byte.type_id(),
        PrimitiveType::Usint.type_id()
    );

    for value in [0, u8::MAX] {
        assert!(TypedScalar::new(PrimitiveType::Char, ScalarValue::Char(value)).is_ok());
    }
    for value in [i64::MIN, 0, i64::MAX] {
        assert!(TypedScalar::new(PrimitiveType::Time, ScalarValue::Time(value)).is_ok());
    }
    assert!(TypedScalar::new(PrimitiveType::String(0), ScalarValue::String(Vec::new())).is_ok());
    assert!(
        TypedScalar::new(
            PrimitiveType::String(254),
            ScalarValue::String(vec![u8::MAX; 254])
        )
        .is_ok()
    );
    assert_eq!(
        TypedScalar::new(PrimitiveType::String(255), ScalarValue::String(Vec::new())),
        Err(ScalarTypeError::InvalidStringCapacity)
    );
}

#[test]
#[allow(clippy::too_many_lines)]
fn fixed_width_integer_operations_cover_every_width_and_fault_boundary() {
    let signed_cases = [
        (PrimitiveType::Sint, i64::from(i8::MIN), i64::from(i8::MAX)),
        (PrimitiveType::Int, i64::from(i16::MIN), i64::from(i16::MAX)),
        (
            PrimitiveType::Dint,
            i64::from(i32::MIN),
            i64::from(i32::MAX),
        ),
        (PrimitiveType::Lint, i64::MIN, i64::MAX),
    ];
    for (data_type, minimum, maximum) in signed_cases {
        let min = signed(data_type, minimum);
        let max = signed(data_type, maximum);
        let one = signed(data_type, 1);
        let two = signed(data_type, 2);
        let minus_one = signed(data_type, -1);
        assert_eq!(
            numeric_binary(NumericBinaryOperator::Add, &max, &one)
                .unwrap()
                .value(),
            &ScalarValue::Signed(minimum)
        );
        assert_eq!(
            numeric_binary(NumericBinaryOperator::Subtract, &min, &one)
                .unwrap()
                .value(),
            &ScalarValue::Signed(maximum)
        );
        assert_eq!(
            numeric_binary(NumericBinaryOperator::Multiply, &max, &two)
                .unwrap()
                .value(),
            &ScalarValue::Signed(-2)
        );
        assert_eq!(negate(&min).unwrap(), min);
        assert_eq!(absolute(&min), Err(ScalarFault::ArithmeticOverflow));
        assert_eq!(
            numeric_binary(NumericBinaryOperator::Divide, &min, &minus_one),
            Err(ScalarFault::ArithmeticOverflow)
        );
        assert_eq!(
            numeric_binary(NumericBinaryOperator::Modulo, &min, &minus_one),
            Err(ScalarFault::ArithmeticOverflow)
        );
        assert_eq!(
            numeric_binary(NumericBinaryOperator::Modulo, &signed(data_type, -5), &two)
                .unwrap()
                .value(),
            &ScalarValue::Signed(-1)
        );
        assert_eq!(
            numeric_binary(
                NumericBinaryOperator::Modulo,
                &signed(data_type, 5),
                &signed(data_type, -2)
            )
            .unwrap()
            .value(),
            &ScalarValue::Signed(1)
        );
        assert_eq!(
            numeric_binary(NumericBinaryOperator::Divide, &one, &signed(data_type, 0)),
            Err(ScalarFault::DivideByZero)
        );
    }

    let unsigned_cases = [
        (PrimitiveType::Usint, u64::from(u8::MAX)),
        (PrimitiveType::Uint, u64::from(u16::MAX)),
        (PrimitiveType::Udint, u64::from(u32::MAX)),
        (PrimitiveType::Ulint, u64::MAX),
    ];
    for (data_type, maximum) in unsigned_cases {
        let zero = unsigned(data_type, 0);
        let one = unsigned(data_type, 1);
        let two = unsigned(data_type, 2);
        let max = unsigned(data_type, maximum);
        assert_eq!(
            numeric_binary(NumericBinaryOperator::Add, &max, &one)
                .unwrap()
                .value(),
            &ScalarValue::Unsigned(0)
        );
        assert_eq!(
            numeric_binary(NumericBinaryOperator::Subtract, &zero, &one)
                .unwrap()
                .value(),
            &ScalarValue::Unsigned(maximum)
        );
        assert_eq!(
            numeric_binary(NumericBinaryOperator::Multiply, &max, &two)
                .unwrap()
                .value(),
            &ScalarValue::Unsigned(maximum - 1)
        );
        assert_eq!(
            numeric_binary(NumericBinaryOperator::Modulo, &max, &two)
                .unwrap()
                .value(),
            &ScalarValue::Unsigned(1)
        );
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn ieee_special_values_are_canonical_across_arithmetic_comparison_and_rounding() {
    let infinity =
        numeric_binary(NumericBinaryOperator::Multiply, &real(f32::MAX), &real(2.0)).unwrap();
    assert_eq!(
        infinity.value(),
        &ScalarValue::Real(CanonicalF32::new(f32::INFINITY))
    );
    let nan = numeric_binary(NumericBinaryOperator::Subtract, &infinity, &infinity).unwrap();
    assert_eq!(
        nan.value(),
        &ScalarValue::Real(CanonicalF32::from_bits(CanonicalF32::QUIET_NAN_BITS))
    );
    let positive_zero = real(0.0);
    let negative_zero = real(-0.0);
    let comparisons = [
        (ComparisonOperator::Equal, false),
        (ComparisonOperator::NotEqual, true),
        (ComparisonOperator::Less, false),
        (ComparisonOperator::LessEqual, false),
        (ComparisonOperator::Greater, false),
        (ComparisonOperator::GreaterEqual, false),
    ];
    for (operator, expected) in comparisons {
        assert_eq!(compare(operator, &nan, &positive_zero).unwrap(), expected);
        assert_eq!(compare(operator, &positive_zero, &nan).unwrap(), expected);
    }
    assert!(compare(ComparisonOperator::Equal, &negative_zero, &positive_zero).unwrap());
    assert_eq!(
        minimum(&negative_zero, &positive_zero).unwrap(),
        negative_zero
    );
    assert_eq!(
        minimum(&positive_zero, &negative_zero).unwrap(),
        negative_zero
    );
    assert_eq!(
        maximum(&negative_zero, &positive_zero).unwrap(),
        positive_zero
    );
    assert_eq!(
        maximum(&positive_zero, &negative_zero).unwrap(),
        positive_zero
    );
    assert_eq!(minimum(&nan, &positive_zero).unwrap(), nan);
    assert_eq!(maximum(&positive_zero, &nan).unwrap(), nan);
    assert_eq!(absolute(&real(-0.0)).unwrap(), real(0.0));
    assert_eq!(absolute(&nan).unwrap(), nan);
    assert_eq!(
        numeric_binary(NumericBinaryOperator::Divide, &real(1.0), &real(0.0)),
        Err(ScalarFault::DivideByZero)
    );
    assert_eq!(
        numeric_binary(NumericBinaryOperator::Modulo, &real(1.0), &real(2.0)),
        Err(ScalarFault::Type(ScalarTypeError::UnsupportedOperation))
    );
    assert_eq!(
        numeric_binary(
            NumericBinaryOperator::Multiply,
            &real(f32::MIN_POSITIVE),
            &real(0.5)
        )
        .unwrap()
        .value(),
        &ScalarValue::Real(CanonicalF32::from_bits(0x0040_0000))
    );

    let infinity64 = numeric_binary(
        NumericBinaryOperator::Multiply,
        &lreal(f64::MAX),
        &lreal(2.0),
    )
    .unwrap();
    assert_eq!(
        infinity64.value(),
        &ScalarValue::Lreal(CanonicalF64::new(f64::INFINITY))
    );
    let nan64 = numeric_binary(NumericBinaryOperator::Subtract, &infinity64, &infinity64).unwrap();
    assert_eq!(
        nan64.value(),
        &ScalarValue::Lreal(CanonicalF64::from_bits(CanonicalF64::QUIET_NAN_BITS))
    );
    assert_eq!(minimum(&lreal(-0.0), &lreal(0.0)).unwrap(), lreal(-0.0));
    assert_eq!(maximum(&lreal(-0.0), &lreal(0.0)).unwrap(), lreal(0.0));

    let rounding_cases = [
        (2.5, RoundingOperator::Round, 2),
        (3.5, RoundingOperator::Round, 4),
        (-2.5, RoundingOperator::Round, -2),
        (-3.5, RoundingOperator::Round, -4),
        (-2.9, RoundingOperator::Trunc, -2),
        (-2.1, RoundingOperator::Floor, -3),
        (-2.9, RoundingOperator::Ceil, -2),
    ];
    for (input, operator, expected) in rounding_cases {
        assert_eq!(
            round_to_integer(&lreal(input), PrimitiveType::Dint, operator)
                .unwrap()
                .value(),
            &ScalarValue::Signed(expected)
        );
    }
    for invalid in [lreal(f64::NAN), lreal(f64::INFINITY), lreal(1.0e40)] {
        assert_eq!(
            round_to_integer(&invalid, PrimitiveType::Dint, RoundingOperator::Round),
            Err(ScalarFault::Conversion)
        );
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn explicit_conversion_registry_is_exhaustive_and_runtime_parity_is_total() {
    let types = [
        PrimitiveType::Bool,
        PrimitiveType::Sint,
        PrimitiveType::Int,
        PrimitiveType::Dint,
        PrimitiveType::Lint,
        PrimitiveType::Usint,
        PrimitiveType::Uint,
        PrimitiveType::Udint,
        PrimitiveType::Ulint,
        PrimitiveType::Byte,
        PrimitiveType::Word,
        PrimitiveType::Dword,
        PrimitiveType::Lword,
        PrimitiveType::Real,
        PrimitiveType::Lreal,
        PrimitiveType::Char,
        PrimitiveType::String(0),
        PrimitiveType::String(254),
        PrimitiveType::Time,
    ];
    for source in types {
        let value = representative(source);
        for destination in types {
            let expected = expected_explicit_conversion(source, destination);
            assert_eq!(
                explicit_conversion_allowed(source, destination),
                expected,
                "registry mismatch for {source:?} -> {destination:?}"
            );
            let converted = convert(&value, destination);
            assert_eq!(
                converted.is_ok(),
                expected,
                "execution mismatch for {source:?} -> {destination:?}: {converted:?}"
            );
            if let Ok(converted) = converted {
                assert_eq!(converted.data_type(), destination);
                destination
                    .validate_scalar(converted.value())
                    .expect("admitted conversion returns a canonical destination value");
            }
        }
    }

    for data_type in [
        PrimitiveType::Bool,
        PrimitiveType::Byte,
        PrimitiveType::Real,
        PrimitiveType::Lreal,
        PrimitiveType::Char,
        PrimitiveType::String(0),
        PrimitiveType::String(254),
        PrimitiveType::Time,
    ] {
        assert_eq!(
            convert(&representative(data_type), data_type),
            Err(ScalarFault::Conversion),
            "exact assignment is MOVE, not an unregistered same-type conversion for {data_type:?}"
        );
    }

    assert_eq!(
        convert(
            &unsigned(PrimitiveType::Udint, 16_777_217),
            PrimitiveType::Real
        )
        .unwrap()
        .value(),
        &ScalarValue::Real(CanonicalF32::new(16_777_216.0))
    );
    assert_eq!(
        convert(
            &unsigned(PrimitiveType::Ulint, 9_007_199_254_740_993),
            PrimitiveType::Lreal
        )
        .unwrap()
        .value(),
        &ScalarValue::Lreal(CanonicalF64::new(9_007_199_254_740_992.0))
    );
    assert_eq!(
        convert(&lreal(f64::MAX), PrimitiveType::Real)
            .unwrap()
            .value(),
        &ScalarValue::Real(CanonicalF32::new(f32::INFINITY))
    );
    assert_eq!(
        convert(&lreal(-f64::MIN_POSITIVE), PrimitiveType::Real)
            .unwrap()
            .value(),
        &ScalarValue::Real(CanonicalF32::new(-0.0))
    );
    assert_eq!(
        convert(&real(f32::NAN), PrimitiveType::Lreal)
            .unwrap()
            .value(),
        &ScalarValue::Lreal(CanonicalF64::from_bits(CanonicalF64::QUIET_NAN_BITS))
    );

    assert!(implicit_conversion_allowed(
        PrimitiveType::Sint,
        PrimitiveType::Dint
    ));
    assert!(implicit_conversion_allowed(
        PrimitiveType::Uint,
        PrimitiveType::Dint
    ));
    assert!(implicit_conversion_allowed(
        PrimitiveType::Real,
        PrimitiveType::Lreal
    ));
    assert!(!implicit_conversion_allowed(
        PrimitiveType::Int,
        PrimitiveType::Uint
    ));
    assert!(!implicit_conversion_allowed(
        PrimitiveType::Lreal,
        PrimitiveType::Real
    ));
    assert!(!implicit_conversion_allowed(
        PrimitiveType::Bool,
        PrimitiveType::Usint
    ));
}

#[test]
fn strings_char_time_and_bit_strings_keep_their_declared_semantics() {
    let source = scalar(
        PrimitiveType::String(3),
        ScalarValue::String(vec![0x41, 0x42, 0xff]),
    );
    assert_eq!(
        assign_to(&source, PrimitiveType::String(3)).unwrap(),
        source
    );
    assert_eq!(
        assign_to(&source, PrimitiveType::String(254))
            .unwrap()
            .value(),
        &ScalarValue::String(vec![0x41, 0x42, 0xff])
    );
    assert_eq!(
        assign_to(&source, PrimitiveType::String(2)),
        Err(ScalarFault::Bounds)
    );
    assert!(
        compare(
            ComparisonOperator::Less,
            &scalar(PrimitiveType::String(2), ScalarValue::String(vec![0x41])),
            &scalar(
                PrimitiveType::String(2),
                ScalarValue::String(vec![0x41, 0x00])
            )
        )
        .unwrap()
    );
    assert!(
        compare(
            ComparisonOperator::Greater,
            &scalar(PrimitiveType::Char, ScalarValue::Char(0xff)),
            &scalar(PrimitiveType::Char, ScalarValue::Char(0x7f))
        )
        .unwrap()
    );

    let time_min = scalar(PrimitiveType::Time, ScalarValue::Time(i64::MIN));
    let time_max = scalar(PrimitiveType::Time, ScalarValue::Time(i64::MAX));
    assert!(compare(ComparisonOperator::Less, &time_min, &time_max).unwrap());
    assert_eq!(
        convert(&time_min, PrimitiveType::Lint).unwrap().value(),
        &ScalarValue::Signed(i64::MIN)
    );

    let byte = scalar(PrimitiveType::Byte, ScalarValue::BitString(0x81));
    assert_eq!(
        bit_not(&byte).unwrap().value(),
        &ScalarValue::BitString(0x7e)
    );
    assert_eq!(
        bit_binary(
            BitBinaryOperator::Xor,
            &byte,
            &scalar(PrimitiveType::Byte, ScalarValue::BitString(0xff))
        )
        .unwrap()
        .value(),
        &ScalarValue::BitString(0x7e)
    );
    assert_eq!(
        shift_rotate(ShiftOperator::RotateRight, &byte, 1)
            .unwrap()
            .value(),
        &ScalarValue::BitString(0xc0)
    );
    assert_eq!(
        compare(ComparisonOperator::Less, &byte, &byte),
        Err(ScalarFault::Type(ScalarTypeError::UnsupportedOperation))
    );
    assert_eq!(
        numeric_binary(
            NumericBinaryOperator::Add,
            &unsigned(PrimitiveType::Usint, 1),
            &byte
        ),
        Err(ScalarFault::Type(ScalarTypeError::TypeMismatch))
    );
}
