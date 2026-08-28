use plc_program::{
    CanonicalF32, CanonicalType, CanonicalValue, DataType, PlcValue, PrimitiveType, ScalarValue,
    TypedScalar,
};

#[test]
fn declaration_values_project_without_losing_scalar_identity_or_float_bits() {
    let vectors = [
        (
            DataType::SInt,
            CanonicalValue::SInt(i8::MIN),
            PrimitiveType::Sint,
            ScalarValue::Signed(i64::from(i8::MIN)),
        ),
        (
            DataType::Byte,
            CanonicalValue::Byte(u8::MAX),
            PrimitiveType::Byte,
            ScalarValue::BitString(u64::from(u8::MAX)),
        ),
        (
            DataType::Time,
            CanonicalValue::TimeMilliseconds(i64::MIN),
            PrimitiveType::Time,
            ScalarValue::Time(i64::MIN),
        ),
    ];

    for (program_type, program_value, primitive, scalar) in vectors {
        assert_eq!(
            program_type.canonical_scalar_type(),
            Some(CanonicalType::Primitive(primitive))
        );
        assert_eq!(
            program_value.plc_value_for(&program_type),
            Some(PlcValue::Scalar(
                TypedScalar::new(primitive, scalar).unwrap()
            ))
        );
    }

    let negative_zero = CanonicalValue::RealBits(CanonicalF32::new(-0.0).bits());
    assert_eq!(
        negative_zero.plc_value_for(&DataType::Real),
        Some(PlcValue::Scalar(
            TypedScalar::new(
                PrimitiveType::Real,
                ScalarValue::Real(CanonicalF32::new(-0.0)),
            )
            .unwrap()
        ))
    );
}

#[test]
fn bridge_fails_closed_for_wrong_or_unresolved_types_and_noncanonical_nan() {
    assert_eq!(CanonicalValue::SInt(1).plc_value_for(&DataType::Byte), None);
    assert_eq!(
        CanonicalValue::Bool(true).plc_value_for(&DataType::Named("TYPE:unresolved".to_owned())),
        None
    );
    assert_eq!(
        CanonicalValue::RealBits(0x7fa1_2345).plc_value_for(&DataType::Real),
        None
    );
    assert_eq!(
        DataType::String { capacity: 255 }.canonical_scalar_type(),
        None
    );
}
