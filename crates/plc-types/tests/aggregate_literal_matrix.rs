use plc_types::{
    AggregateLimits, ArrayBound, CanonicalF32, CanonicalType, PlcValue, PrimitiveType, ScalarValue,
    StableUuid, StructFieldValue, StructMember, TypeError, TypedScalar, assign_value,
    store_array_element,
};

fn id(discriminator: u8) -> StableUuid {
    let mut bytes = [0_u8; 16];
    bytes[6] = 0x40;
    bytes[8] = 0x80;
    bytes[15] = discriminator;
    StableUuid::from_bytes(bytes).expect("test identity is RFC 9562 UUIDv4")
}

fn scalar(data_type: PrimitiveType, value: ScalarValue) -> PlcValue {
    PlcValue::scalar(
        TypedScalar::new(data_type, value).expect("fixed aggregate scalar oracle is canonical"),
    )
}

fn member(
    identity: u8,
    name: &str,
    order: u32,
    data_type: CanonicalType,
    reusable_default: Option<PlcValue>,
) -> StructMember {
    StructMember {
        id: id(identity),
        name: name.to_owned(),
        declared_order: order,
        data_type,
        reusable_default,
        comment: String::new(),
    }
}

fn field(identity: u8, value: PlcValue) -> StructFieldValue {
    StructFieldValue {
        member_id: id(identity),
        value,
    }
}

#[test]
fn signed_array_bounds_cover_the_full_i32_domain_in_canonical_row_major_order() {
    let limits = AggregateLimits::edu21();
    let data_type = CanonicalType::Array {
        dimensions: vec![
            ArrayBound {
                lower: i32::MIN,
                upper: i32::MIN,
            },
            ArrayBound {
                lower: -1,
                upper: 0,
            },
            ArrayBound {
                lower: i32::MAX,
                upper: i32::MAX,
            },
        ],
        element_type: Box::new(CanonicalType::Primitive(PrimitiveType::Dint)),
    };
    let original = PlcValue::Array(vec![
        scalar(PrimitiveType::Dint, ScalarValue::Signed(-1)),
        scalar(PrimitiveType::Dint, ScalarValue::Signed(1)),
    ]);
    data_type.validate_value(&original, limits).unwrap();
    assert_eq!(
        data_type
            .array_linear_index(&[i32::MIN, -1, i32::MAX], limits)
            .unwrap(),
        0
    );
    assert_eq!(
        data_type
            .array_linear_index(&[i32::MIN, 0, i32::MAX], limits)
            .unwrap(),
        1
    );
    assert_eq!(
        data_type.array_linear_index(&[i32::MIN, 1, i32::MAX], limits),
        Err(TypeError::Bounds)
    );
    assert_eq!(
        data_type.array_linear_index(&[i32::MIN, 0], limits),
        Err(TypeError::ValueShapeMismatch)
    );

    let replacement = scalar(PrimitiveType::Dint, ScalarValue::Signed(99));
    assert_eq!(
        store_array_element(
            &data_type,
            &original,
            &[i32::MIN, 0, i32::MAX],
            &replacement,
            limits
        )
        .unwrap(),
        PlcValue::Array(vec![
            scalar(PrimitiveType::Dint, ScalarValue::Signed(-1)),
            replacement,
        ])
    );
    assert_eq!(
        store_array_element(
            &data_type,
            &original,
            &[i32::MIN, 0, i32::MAX],
            &scalar(PrimitiveType::Int, ScalarValue::Signed(99)),
            limits
        ),
        Err(TypeError::ValueTypeMismatch)
    );
    assert_eq!(
        original,
        PlcValue::Array(vec![
            scalar(PrimitiveType::Dint, ScalarValue::Signed(-1)),
            scalar(PrimitiveType::Dint, ScalarValue::Signed(1)),
        ]),
        "failed replacement must not mutate the source aggregate"
    );
}

#[test]
fn array_assignment_requires_exact_dimensions_bounds_and_recursive_element_type() {
    let limits = AggregateLimits::edu21();
    let exact = CanonicalType::Array {
        dimensions: vec![ArrayBound {
            lower: -1,
            upper: 0,
        }],
        element_type: Box::new(CanonicalType::Primitive(PrimitiveType::Sint)),
    };
    let literal = PlcValue::Array(vec![
        scalar(PrimitiveType::Sint, ScalarValue::Signed(i64::from(i8::MIN))),
        scalar(PrimitiveType::Sint, ScalarValue::Signed(i64::from(i8::MAX))),
    ]);
    assert_eq!(
        assign_value(&exact, &literal, &exact, limits).unwrap(),
        literal
    );

    let wrong_bounds = CanonicalType::Array {
        dimensions: vec![ArrayBound { lower: 0, upper: 1 }],
        element_type: Box::new(CanonicalType::Primitive(PrimitiveType::Sint)),
    };
    let wrong_dimensions = CanonicalType::Array {
        dimensions: vec![
            ArrayBound {
                lower: -1,
                upper: -1,
            },
            ArrayBound { lower: 0, upper: 1 },
        ],
        element_type: Box::new(CanonicalType::Primitive(PrimitiveType::Sint)),
    };
    let wrong_element = CanonicalType::Array {
        dimensions: vec![ArrayBound {
            lower: -1,
            upper: 0,
        }],
        element_type: Box::new(CanonicalType::Primitive(PrimitiveType::Usint)),
    };
    for destination in [&wrong_bounds, &wrong_dimensions, &wrong_element] {
        assert_eq!(
            assign_value(&exact, &literal, destination, limits),
            Err(TypeError::AssignmentTypeMismatch)
        );
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn anonymous_structure_literals_use_shape_while_named_literals_remain_nominal() {
    let limits = AggregateLimits::edu21();
    let source = CanonicalType::AnonymousStruct {
        members: vec![
            member(
                1,
                "Reading",
                1,
                CanonicalType::Primitive(PrimitiveType::Real),
                None,
            ),
            member(
                2,
                "Samples",
                0,
                CanonicalType::Array {
                    dimensions: vec![ArrayBound { lower: 1, upper: 2 }],
                    element_type: Box::new(CanonicalType::Primitive(PrimitiveType::Dint)),
                },
                None,
            ),
        ],
    };
    let source_literal = PlcValue::Struct(vec![
        field(
            1,
            scalar(
                PrimitiveType::Real,
                ScalarValue::Real(CanonicalF32::new(-0.0)),
            ),
        ),
        field(
            2,
            PlcValue::Array(vec![
                scalar(
                    PrimitiveType::Dint,
                    ScalarValue::Signed(i64::from(i32::MIN)),
                ),
                scalar(
                    PrimitiveType::Dint,
                    ScalarValue::Signed(i64::from(i32::MAX)),
                ),
            ]),
        ),
    ]);
    source.validate_value(&source_literal, limits).unwrap();

    let mut destination_reading = member(
        11,
        "READING",
        1,
        CanonicalType::Primitive(PrimitiveType::Real),
        Some(scalar(
            PrimitiveType::Real,
            ScalarValue::Real(CanonicalF32::new(5.0)),
        )),
    );
    destination_reading.comment = "nonsemantic destination metadata".to_owned();
    let destination = CanonicalType::AnonymousStruct {
        members: vec![
            destination_reading,
            member(
                12,
                "samples",
                0,
                CanonicalType::Array {
                    dimensions: vec![ArrayBound { lower: 1, upper: 2 }],
                    element_type: Box::new(CanonicalType::Primitive(PrimitiveType::Dint)),
                },
                None,
            ),
        ],
    };
    assert_eq!(
        source.assignment_signature(limits).unwrap(),
        destination.assignment_signature(limits).unwrap()
    );
    assert_ne!(
        source.canonical_bytes(limits).unwrap(),
        destination.canonical_bytes(limits).unwrap()
    );
    let assigned = assign_value(&source, &source_literal, &destination, limits).unwrap();
    assert_eq!(
        assigned,
        PlcValue::Struct(vec![
            field(
                12,
                PlcValue::Array(vec![
                    scalar(
                        PrimitiveType::Dint,
                        ScalarValue::Signed(i64::from(i32::MIN))
                    ),
                    scalar(
                        PrimitiveType::Dint,
                        ScalarValue::Signed(i64::from(i32::MAX))
                    ),
                ])
            ),
            field(
                11,
                scalar(
                    PrimitiveType::Real,
                    ScalarValue::Real(CanonicalF32::new(-0.0))
                )
            ),
        ])
    );

    let named = CanonicalType::NamedStruct {
        id: id(20),
        members: vec![member(
            21,
            "Reading",
            0,
            CanonicalType::Primitive(PrimitiveType::Real),
            None,
        )],
    };
    let same_shape_different_name = CanonicalType::NamedStruct {
        id: id(22),
        members: vec![member(
            21,
            "Reading",
            0,
            CanonicalType::Primitive(PrimitiveType::Real),
            None,
        )],
    };
    assert!(
        !named
            .assignment_compatible_with(&same_shape_different_name, limits)
            .unwrap()
    );
    assert!(!source.assignment_compatible_with(&named, limits).unwrap());
}

#[test]
fn recursive_defaults_and_serialization_are_order_independent_and_byte_stable() {
    let limits = AggregateLimits::edu21();
    let data_type = CanonicalType::AnonymousStruct {
        members: vec![
            member(
                31,
                "Flag",
                1,
                CanonicalType::Primitive(PrimitiveType::Bool),
                Some(scalar(PrimitiveType::Bool, ScalarValue::Bool(true))),
            ),
            member(
                32,
                "Values",
                0,
                CanonicalType::Array {
                    dimensions: vec![ArrayBound {
                        lower: -1,
                        upper: 0,
                    }],
                    element_type: Box::new(CanonicalType::Primitive(PrimitiveType::Int)),
                },
                None,
            ),
        ],
    };
    let expected_default = PlcValue::Struct(vec![
        field(
            32,
            PlcValue::Array(vec![
                scalar(PrimitiveType::Int, ScalarValue::Signed(0)),
                scalar(PrimitiveType::Int, ScalarValue::Signed(0)),
            ]),
        ),
        field(31, scalar(PrimitiveType::Bool, ScalarValue::Bool(true))),
    ]);
    assert_eq!(
        data_type.canonical_default(limits).unwrap(),
        expected_default
    );

    let reordered_fields = PlcValue::Struct(vec![
        field(31, scalar(PrimitiveType::Bool, ScalarValue::Bool(true))),
        field(
            32,
            PlcValue::Array(vec![
                scalar(PrimitiveType::Int, ScalarValue::Signed(0)),
                scalar(PrimitiveType::Int, ScalarValue::Signed(0)),
            ]),
        ),
    ]);
    let first = data_type
        .serialize_value(&expected_default, limits)
        .unwrap();
    let second = data_type
        .serialize_value(&reordered_fields, limits)
        .unwrap();
    assert_eq!(first, second);
    assert_eq!(
        data_type.deserialize_value(&first, limits).unwrap(),
        expected_default
    );

    let duplicate_case_folded_name = CanonicalType::AnonymousStruct {
        members: vec![
            member(
                40,
                "Value",
                0,
                CanonicalType::Primitive(PrimitiveType::Bool),
                None,
            ),
            member(
                41,
                "VALUE",
                1,
                CanonicalType::Primitive(PrimitiveType::Bool),
                None,
            ),
        ],
    };
    assert_eq!(
        duplicate_case_folded_name.validate(limits),
        Err(TypeError::DuplicateMemberName)
    );
}
