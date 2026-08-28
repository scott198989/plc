use plc_runtime::*;
use plc_types::{ArrayBound, CanonicalType, PlcValue, PrimitiveType, ScalarValue, TypedScalar};

const UNIVERSE: UniverseId = UniverseId(0xA663);
const CONTROLLER: VirtualControllerId = VirtualControllerId(0xF111);
const STATUS: MemoryId = MemoryId(1);
const TARGET: MemoryId = MemoryId(10);
const SOURCE: MemoryId = MemoryId(11);

fn dint(value: i32) -> PlcValue {
    PlcValue::Scalar(
        TypedScalar::new(PrimitiveType::Dint, ScalarValue::Signed(i64::from(value)))
            .expect("valid DINT"),
    )
}

fn dint_array_type(length: i32) -> CanonicalType {
    CanonicalType::Array {
        dimensions: vec![ArrayBound {
            lower: 0,
            upper: length - 1,
        }],
        element_type: Box::new(CanonicalType::Primitive(PrimitiveType::Dint)),
    }
}

fn array(values: &[i32]) -> PlcValue {
    PlcValue::Array(values.iter().copied().map(dint).collect())
}

fn string_value(capacity: u8, value: &[u8]) -> PlcValue {
    PlcValue::Scalar(
        TypedScalar::new(
            PrimitiveType::String(capacity),
            ScalarValue::String(value.to_vec()),
        )
        .expect("valid STRING"),
    )
}

fn aggregate_memory(
    id: MemoryId,
    data_type: CanonicalType,
    loaded_start: PlcValue,
) -> AggregateMemoryDefinition {
    AggregateMemoryDefinition {
        id,
        data_type,
        loaded_start,
        retentive: false,
    }
}

fn instruction(operation: Operation) -> Instruction {
    Instruction::new(1, 0xA663_0001, operation)
}

fn package(
    aggregate_memory: Vec<AggregateMemoryDefinition>,
    operation: Operation,
) -> Result<ArtifactPackage, ArtifactError> {
    ArtifactPackage::seal_verified(ArtifactSpec::edu21_with_aggregates(
        Sha256::digest(b"aggregate-instruction-profile"),
        vec![MemoryDefinition {
            id: STATUS,
            value_type: ValueType::Bool,
            loaded_start: CanonicalValue::Bool(true),
            retentive: false,
        }],
        aggregate_memory,
        vec![],
        vec![],
        ProgramImage {
            startup: None,
            timed: vec![],
            cyclic: ProgramBlock {
                id: BlockId(1),
                instructions: vec![instruction(operation)],
            },
        },
    ))
}

fn running(package: &ArtifactPackage) -> VirtualController {
    let mut controller = VirtualController::new(UNIVERSE, CONTROLLER, 0x5eed);
    controller.power_on().expect("power on");
    controller
        .install_verified_artifact(package)
        .expect("verified aggregate artifact load");
    controller
        .request_run(RestartKind::Resume)
        .expect("RUN transition");
    controller
}

#[test]
fn fill_writes_every_element_atomically_and_charges_each_scalar_leaf() {
    let target_type = dint_array_type(3);
    let artifact = package(
        vec![aggregate_memory(TARGET, target_type, array(&[0, 0, 0]))],
        Operation::AggregateInstruction {
            instruction: RuntimeAggregateInstructionCode::Fill,
            input: RuntimeAggregateSource::Scalar(Operand::Constant(CanonicalValue::I32(7))),
            target: TARGET,
            activation: None,
            status: STATUS,
            scalar_leaves: 3,
        },
    )
    .expect("valid FILL artifact");
    let mut controller = running(&artifact);
    let report = match controller.run_scan().expect("FILL scan") {
        RunOutcome::Completed(report) => report,
        RunOutcome::Faulted(event) => panic!("FILL faulted: {event:?}"),
    };

    assert_eq!(
        report.work_units, 4,
        "one dispatch plus three scalar leaves"
    );
    assert_eq!(
        controller.actual_aggregate_memory(TARGET),
        Some(&array(&[7, 7, 7]))
    );
    assert_eq!(
        controller.actual_memory(STATUS),
        Some(CanonicalValue::Bool(true))
    );
}

#[test]
fn disabled_fill_suppresses_target_effects_and_charges_only_dispatch() {
    let target_type = dint_array_type(3);
    let artifact = package(
        vec![aggregate_memory(TARGET, target_type, array(&[1, 2, 3]))],
        Operation::AggregateInstruction {
            instruction: RuntimeAggregateInstructionCode::Fill,
            input: RuntimeAggregateSource::Scalar(Operand::Constant(CanonicalValue::I32(9))),
            target: TARGET,
            activation: Some(Operand::Constant(CanonicalValue::Bool(false))),
            status: STATUS,
            scalar_leaves: 3,
        },
    )
    .expect("valid disabled FILL artifact");
    let mut controller = running(&artifact);
    let report = match controller.run_scan().expect("disabled FILL scan") {
        RunOutcome::Completed(report) => report,
        RunOutcome::Faulted(event) => panic!("disabled FILL faulted: {event:?}"),
    };

    assert_eq!(report.work_units, 1);
    assert_eq!(
        controller.actual_aggregate_memory(TARGET),
        Some(&array(&[1, 2, 3]))
    );
    assert_eq!(
        controller.actual_memory(STATUS),
        Some(CanonicalValue::Bool(false))
    );
}

#[test]
fn block_move_snapshots_source_then_atomically_replaces_the_target() {
    let array_type = dint_array_type(3);
    let artifact = package(
        vec![
            aggregate_memory(SOURCE, array_type.clone(), array(&[1, 2, 3])),
            aggregate_memory(TARGET, array_type, array(&[9, 9, 9])),
        ],
        Operation::AggregateInstruction {
            instruction: RuntimeAggregateInstructionCode::BlockMove,
            input: RuntimeAggregateSource::AggregateMemory(SOURCE),
            target: TARGET,
            activation: None,
            status: STATUS,
            scalar_leaves: 3,
        },
    )
    .expect("valid BLKMOVE artifact");
    let mut controller = running(&artifact);
    let report = match controller.run_scan().expect("BLKMOVE scan") {
        RunOutcome::Completed(report) => report,
        RunOutcome::Faulted(event) => panic!("BLKMOVE faulted: {event:?}"),
    };

    assert_eq!(report.work_units, 4);
    assert_eq!(
        controller.actual_aggregate_memory(SOURCE),
        Some(&array(&[1, 2, 3]))
    );
    assert_eq!(
        controller.actual_aggregate_memory(TARGET),
        Some(&array(&[1, 2, 3]))
    );
    assert_eq!(
        controller.actual_memory(STATUS),
        Some(CanonicalValue::Bool(true))
    );
}

#[test]
fn verifier_rejects_malformed_aggregate_shapes_and_declared_costs() {
    let target_type = dint_array_type(3);
    let wrong_cost = package(
        vec![aggregate_memory(
            TARGET,
            target_type.clone(),
            array(&[0, 0, 0]),
        )],
        Operation::AggregateInstruction {
            instruction: RuntimeAggregateInstructionCode::Fill,
            input: RuntimeAggregateSource::Scalar(Operand::Constant(CanonicalValue::I32(7))),
            target: TARGET,
            activation: None,
            status: STATUS,
            scalar_leaves: 2,
        },
    );
    assert_eq!(wrong_cost, Err(ArtifactError::TypeMismatch));

    let scalar_block_move = package(
        vec![aggregate_memory(TARGET, target_type, array(&[0, 0, 0]))],
        Operation::AggregateInstruction {
            instruction: RuntimeAggregateInstructionCode::BlockMove,
            input: RuntimeAggregateSource::Scalar(Operand::Constant(CanonicalValue::I32(7))),
            target: TARGET,
            activation: None,
            status: STATUS,
            scalar_leaves: 3,
        },
    );
    assert_eq!(scalar_block_move, Err(ArtifactError::TypeMismatch));
}

#[test]
fn block_move_supports_bounded_strings_and_suppresses_disabled_effects() {
    let string_type = CanonicalType::Primitive(PrimitiveType::String(8));
    let enabled = package(
        vec![
            aggregate_memory(SOURCE, string_type.clone(), string_value(8, b"source")),
            aggregate_memory(TARGET, string_type.clone(), string_value(8, b"old")),
        ],
        Operation::AggregateInstruction {
            instruction: RuntimeAggregateInstructionCode::BlockMove,
            input: RuntimeAggregateSource::AggregateMemory(SOURCE),
            target: TARGET,
            activation: None,
            status: STATUS,
            scalar_leaves: 1,
        },
    )
    .expect("valid STRING BLKMOVE artifact");
    let mut controller = running(&enabled);
    let report = match controller.run_scan().expect("STRING BLKMOVE scan") {
        RunOutcome::Completed(report) => report,
        RunOutcome::Faulted(event) => panic!("STRING BLKMOVE faulted: {event:?}"),
    };
    assert_eq!(report.work_units, 2);
    assert_eq!(
        controller.actual_aggregate_memory(TARGET),
        Some(&string_value(8, b"source"))
    );

    let disabled = package(
        vec![
            aggregate_memory(SOURCE, string_type.clone(), string_value(8, b"source")),
            aggregate_memory(TARGET, string_type, string_value(8, b"old")),
        ],
        Operation::AggregateInstruction {
            instruction: RuntimeAggregateInstructionCode::BlockMove,
            input: RuntimeAggregateSource::AggregateMemory(SOURCE),
            target: TARGET,
            activation: Some(Operand::Constant(CanonicalValue::Bool(false))),
            status: STATUS,
            scalar_leaves: 1,
        },
    )
    .expect("valid disabled STRING BLKMOVE artifact");
    let mut controller = running(&disabled);
    let report = match controller.run_scan().expect("disabled STRING BLKMOVE scan") {
        RunOutcome::Completed(report) => report,
        RunOutcome::Faulted(event) => panic!("disabled STRING BLKMOVE faulted: {event:?}"),
    };
    assert_eq!(report.work_units, 1);
    assert_eq!(
        controller.actual_aggregate_memory(TARGET),
        Some(&string_value(8, b"old"))
    );
    assert_eq!(
        controller.actual_memory(STATUS),
        Some(CanonicalValue::Bool(false))
    );
}

#[test]
fn aggregate_work_budget_faults_before_any_destination_store() {
    let element_count = MAX_WORK_UNITS_PER_SCAN;
    let loaded = PlcValue::Array((0..element_count).map(|_| dint(0)).collect());
    let target_type = dint_array_type(i32::try_from(element_count).expect("bounded count"));
    let artifact = package(
        vec![aggregate_memory(TARGET, target_type, loaded.clone())],
        Operation::AggregateInstruction {
            instruction: RuntimeAggregateInstructionCode::Fill,
            input: RuntimeAggregateSource::Scalar(Operand::Constant(CanonicalValue::I32(7))),
            target: TARGET,
            activation: None,
            status: STATUS,
            scalar_leaves: element_count,
        },
    )
    .expect("valid over-budget aggregate artifact");
    let mut controller = running(&artifact);
    let event = match controller.run_scan().expect("budgeted aggregate scan") {
        RunOutcome::Faulted(event) => event,
        RunOutcome::Completed(report) => panic!("over-budget FILL completed: {report:?}"),
    };
    assert_eq!(event.code, DiagnosticCode::WorkUnitBudgetExceeded);
    assert_eq!(controller.actual_aggregate_memory(TARGET), Some(&loaded));
    assert_eq!(
        controller.actual_memory(STATUS),
        Some(CanonicalValue::Bool(true)),
        "budget admission fails before the operation can update status or OUT"
    );
}
