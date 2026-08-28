use std::collections::BTreeMap;

use plc_program::*;
use plc_types::{ArrayBound, CanonicalType, PrimitiveType, StableUuid, StructMember};

fn uuid(discriminator: u8) -> StableUuid {
    let mut bytes = [0_u8; 16];
    bytes[6] = 0x40;
    bytes[8] = 0x80;
    bytes[15] = discriminator;
    StableUuid::from_bytes(bytes).expect("valid stable UUID")
}

fn member(discriminator: u8, name: &str, data_type: CanonicalType) -> StructMember {
    StructMember {
        id: uuid(discriminator),
        name: name.into(),
        declared_order: 0,
        data_type,
        reusable_default: None,
        comment: String::new(),
    }
}

fn scalar_types() -> Vec<DataType> {
    let mut values = vec![
        DataType::Bool,
        DataType::SInt,
        DataType::Int,
        DataType::DInt,
        DataType::LInt,
        DataType::USInt,
        DataType::UInt,
        DataType::UDInt,
        DataType::ULInt,
        DataType::Byte,
        DataType::Word,
        DataType::DWord,
        DataType::LWord,
        DataType::Real,
        DataType::LReal,
        DataType::Char,
        DataType::Time,
    ];
    values.extend((0..=254).map(|capacity| DataType::String { capacity }));
    values
}

fn numeric_types() -> Vec<DataType> {
    scalar_types()
        .into_iter()
        .filter(|data_type| {
            data_type
                .primitive_type()
                .is_some_and(PrimitiveType::is_numeric)
        })
        .collect()
}

fn integer_types() -> Vec<DataType> {
    scalar_types()
        .into_iter()
        .filter(|data_type| {
            data_type
                .primitive_type()
                .is_some_and(PrimitiveType::is_integer)
        })
        .collect()
}

fn aggregate_types() -> Vec<DataType> {
    let array = CanonicalType::Array {
        dimensions: vec![ArrayBound {
            lower: -1,
            upper: 1,
        }],
        element_type: Box::new(CanonicalType::Primitive(PrimitiveType::Dint)),
    };
    let anonymous = CanonicalType::AnonymousStruct {
        members: vec![member(
            1,
            "Value",
            CanonicalType::Primitive(PrimitiveType::Dint),
        )],
    };
    let named = CanonicalType::NamedStruct {
        id: uuid(2),
        members: vec![member(
            3,
            "Enabled",
            CanonicalType::Primitive(PrimitiveType::Bool),
        )],
    };
    vec![
        DataType::Aggregate(array),
        DataType::Aggregate(anonymous),
        DataType::Aggregate(named),
    ]
}

fn candidates_for(
    constraint: InstructionTypeConstraint,
    bound: &BTreeMap<InstructionFormalId, DataType>,
) -> Vec<DataType> {
    match constraint {
        InstructionTypeConstraint::Bool => vec![DataType::Bool],
        InstructionTypeConstraint::Int => vec![DataType::Int],
        InstructionTypeConstraint::DInt => vec![DataType::DInt],
        InstructionTypeConstraint::Real => vec![DataType::Real],
        InstructionTypeConstraint::Time => vec![DataType::Time],
        InstructionTypeConstraint::String => (0..=254)
            .map(|capacity| DataType::String { capacity })
            .collect(),
        InstructionTypeConstraint::Numeric => numeric_types(),
        InstructionTypeConstraint::NumericOrTime => {
            let mut values = numeric_types();
            values.push(DataType::Time);
            values
        }
        InstructionTypeConstraint::Integer => integer_types(),
        InstructionTypeConstraint::AnyValue => scalar_types(),
        InstructionTypeConstraint::AnyAssignable => {
            let mut values = scalar_types();
            values.extend(aggregate_types());
            values
        }
        InstructionTypeConstraint::ArrayOf(source) => {
            let source = bound
                .get(&source)
                .and_then(DataType::canonical_type)
                .expect("registry references an earlier assignable formal");
            vec![DataType::Aggregate(CanonicalType::Array {
                dimensions: vec![ArrayBound { lower: 0, upper: 1 }],
                element_type: Box::new(source),
            })]
        }
        InstructionTypeConstraint::BlockMovable => {
            let mut values: Vec<_> = (0..=254)
                .map(|capacity| DataType::String { capacity })
                .collect();
            values.extend(aggregate_types());
            values
        }
        InstructionTypeConstraint::AssignmentCompatibleWith(source)
        | InstructionTypeConstraint::SameAs(source) => vec![
            bound
                .get(&source)
                .expect("registry references an earlier bound formal")
                .clone(),
        ],
        InstructionTypeConstraint::InstructionState(kind) => {
            vec![DataType::InstructionState(kind)]
        }
        InstructionTypeConstraint::FunctionBlockInstance => {
            vec![DataType::BlockInstance(BlockId::new(0xB10C))]
        }
    }
}

fn required_signatures(definition: &InstructionDefinition) -> Vec<Vec<BoundInstructionFormal>> {
    let required: Vec<_> = definition
        .formals
        .iter()
        .filter(|formal| formal.required)
        .collect();
    let mut rows = vec![BTreeMap::<InstructionFormalId, DataType>::new()];
    for formal in required {
        let mut expanded = Vec::new();
        for row in rows {
            for candidate in candidates_for(formal.type_constraint, &row) {
                let mut candidate_row = row.clone();
                candidate_row.insert(formal.id, candidate);
                expanded.push(candidate_row);
            }
        }
        rows = expanded;
    }
    rows.into_iter()
        .map(|row| {
            row.into_iter()
                .map(|(formal, data_type)| BoundInstructionFormal { formal, data_type })
                .collect()
        })
        .collect()
}

fn expected_disabled_policy(code: InstructionCode) -> Option<DisabledExecutionBehavior> {
    if matches!(code, MOVE | FILL | BLKMOVE | CALL_FC | CALL_FB) {
        Some(DisabledExecutionBehavior::SuppressEffects)
    } else if matches!(
        code,
        TIMER_ON_DELAY
            | TIMER_OFF_DELAY
            | TIMER_PULSE
            | COUNTER_UP
            | COUNTER_DOWN
            | COUNTER_UP_DOWN
    ) {
        Some(DisabledExecutionBehavior::PreserveOutputsNoStateChange)
    } else if matches!(
        code,
        BOOL_NOT
            | BOOL_AND
            | BOOL_OR
            | BOOL_XOR
            | COMPARE_EQ
            | COMPARE_NE
            | COMPARE_LT
            | COMPARE_LE
            | COMPARE_GT
            | COMPARE_GE
            | ADD
            | SUBTRACT
            | MULTIPLY
            | DIVIDE
            | MODULO
            | LIMIT
            | RISING_EDGE
            | FALLING_EDGE
    ) {
        Some(DisabledExecutionBehavior::DefaultOutputsNoStateChange)
    } else {
        None
    }
}

#[test]
fn generated_matrix_binds_every_registry_entry_type_family_and_activation_form() {
    let registry = *phase2_instruction_registry();
    let mut row_count = 0_usize;
    for definition in registry.definitions() {
        let rows = required_signatures(definition);
        assert!(
            !rows.is_empty(),
            "{} has no generated row",
            definition.mnemonic
        );
        for row in rows {
            registry
                .bind_types(definition.code, row.clone())
                .unwrap_or_else(|error| {
                    panic!(
                        "required binding row failed for {}: {error:?} {row:?}",
                        definition.mnemonic
                    )
                });
            row_count += 1;
            if let InstructionActivationPolicy::EnableStatus { enable, status, .. } =
                definition.activation
            {
                let mut activated = row;
                activated.push(BoundInstructionFormal {
                    formal: enable,
                    data_type: DataType::Bool,
                });
                activated.push(BoundInstructionFormal {
                    formal: status,
                    data_type: DataType::Bool,
                });
                registry
                    .bind_types(definition.code, activated)
                    .unwrap_or_else(|error| {
                        panic!(
                            "activated binding row failed for {}: {error:?}",
                            definition.mnemonic
                        )
                    });
                row_count += 1;
            }
        }
    }
    assert!(
        row_count >= 400,
        "matrix unexpectedly shrank to {row_count} rows"
    );
}

#[test]
fn every_registry_entry_has_the_exact_generated_disabled_output_policy() {
    for definition in phase2_instruction_registry().definitions() {
        let actual = match definition.activation {
            InstructionActivationPolicy::None => None,
            InstructionActivationPolicy::EnableStatus {
                status_when_disabled,
                when_disabled,
                ..
            } => {
                assert!(
                    !status_when_disabled,
                    "{} must publish false ENO",
                    definition.mnemonic
                );
                Some(when_disabled)
            }
        };
        assert_eq!(
            actual,
            expected_disabled_policy(definition.code),
            "disabled policy drift for {}",
            definition.mnemonic
        );
    }
}
