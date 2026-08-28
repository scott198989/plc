use std::collections::BTreeSet;

use plc_program::{
    BlockId, BlockInterface, CALL_FC, CallSite, CallSiteId, ControllerId, ControllerProgram,
    DataBlockKind, DependencyEdgeKind, DependencyGraphError, DependencyLocation, DependencyNodeId,
    DependencyOccurrenceId, DependencyResolution, EngineeringNumber, InvalidationEffect, IssueCode,
    ObDeclaration, ProgramBlock, ProgramUnitKind, SemanticChange, SemanticChangeKind,
    SemanticDependencyEdge, SemanticDependencyGraph, validate_program,
};

const ROOT: DependencyNodeId = DependencyNodeId::ExternalSemanticUnit(1);
const TRANSITIVE: DependencyNodeId = DependencyNodeId::ExternalSemanticUnit(10_000);
const UNRELATED: DependencyNodeId = DependencyNodeId::ExternalSemanticUnit(99_999);

const EDGE_KINDS: [DependencyEdgeKind; 16] = [
    DependencyEdgeKind::Declaration,
    DependencyEdgeKind::TypeUse,
    DependencyEdgeKind::MemberUse,
    DependencyEdgeKind::ValueRead,
    DependencyEdgeKind::ValueWrite,
    DependencyEdgeKind::Call,
    DependencyEdgeKind::Instance,
    DependencyEdgeKind::Interface,
    DependencyEdgeKind::StorageLayout,
    DependencyEdgeKind::InstructionCapability,
    DependencyEdgeKind::ProfileCapability,
    DependencyEdgeKind::Address,
    DependencyEdgeKind::HardwareChannel,
    DependencyEdgeKind::NetworkAssignment,
    DependencyEdgeKind::FutureHmiBinding,
    DependencyEdgeKind::FutureLibraryVersion,
];

const CHANGE_KINDS: [SemanticChangeKind; 16] = [
    SemanticChangeKind::Body,
    SemanticChangeKind::ConstantValue,
    SemanticChangeKind::PublicName,
    SemanticChangeKind::TypeShape,
    SemanticChangeKind::PublicInterface,
    SemanticChangeKind::CallTarget,
    SemanticChangeKind::StorageLayout,
    SemanticChangeKind::InstructionRegistry,
    SemanticChangeKind::TrainingProfile,
    SemanticChangeKind::GlobalIrContract,
    SemanticChangeKind::AddressContract,
    SemanticChangeKind::HardwareChannel,
    SemanticChangeKind::NetworkAssignment,
    SemanticChangeKind::SchedulingDeclaration,
    SemanticChangeKind::PresentationOnly,
    SemanticChangeKind::Deletion,
];

fn dependent(index: usize) -> DependencyNodeId {
    DependencyNodeId::ExternalSemanticUnit(100 + index as u128)
}

fn generated_graph() -> SemanticDependencyGraph {
    let mut graph = SemanticDependencyGraph::default();
    for node in [ROOT, TRANSITIVE, UNRELATED] {
        graph.insert_node(node);
    }
    for (index, kind) in EDGE_KINDS.into_iter().enumerate() {
        let node = dependent(index);
        let source_offset = u32::try_from(index).unwrap();
        graph.insert_node(node);
        graph
            .insert_edge(SemanticDependencyEdge {
                dependent: node,
                dependency: ROOT,
                kind,
                location: Some(DependencyLocation::SourceOccurrence {
                    owner: BlockId::new(500 + index as u128),
                    occurrence: DependencyOccurrenceId::External(800 + index as u128),
                    utf8_start: Some(source_offset),
                    utf8_end: Some(source_offset + 1),
                }),
                resolution: DependencyResolution::Resolved,
            })
            .unwrap();
        graph
            .insert_edge(SemanticDependencyEdge {
                dependent: TRANSITIVE,
                dependency: node,
                kind: DependencyEdgeKind::Call,
                location: Some(DependencyLocation::ProjectObject(BlockId::new(9_000))),
                resolution: DependencyResolution::Resolved,
            })
            .unwrap();
    }
    graph
}

fn oracle_edge_affected(edge: DependencyEdgeKind, change: SemanticChangeKind) -> bool {
    match change {
        SemanticChangeKind::Body
        | SemanticChangeKind::PublicName
        | SemanticChangeKind::CallTarget
        | SemanticChangeKind::PresentationOnly => false,
        SemanticChangeKind::ConstantValue => matches!(
            edge,
            DependencyEdgeKind::ValueRead | DependencyEdgeKind::MemberUse
        ),
        SemanticChangeKind::TypeShape => matches!(
            edge,
            DependencyEdgeKind::TypeUse
                | DependencyEdgeKind::MemberUse
                | DependencyEdgeKind::Interface
                | DependencyEdgeKind::StorageLayout
                | DependencyEdgeKind::Instance
                | DependencyEdgeKind::FutureHmiBinding
                | DependencyEdgeKind::FutureLibraryVersion
        ),
        SemanticChangeKind::PublicInterface => matches!(
            edge,
            DependencyEdgeKind::Call
                | DependencyEdgeKind::Instance
                | DependencyEdgeKind::Interface
                | DependencyEdgeKind::MemberUse
        ),
        SemanticChangeKind::StorageLayout => matches!(
            edge,
            DependencyEdgeKind::MemberUse
                | DependencyEdgeKind::ValueRead
                | DependencyEdgeKind::ValueWrite
                | DependencyEdgeKind::Instance
                | DependencyEdgeKind::StorageLayout
                | DependencyEdgeKind::Address
                | DependencyEdgeKind::FutureHmiBinding
        ),
        SemanticChangeKind::InstructionRegistry => {
            edge == DependencyEdgeKind::InstructionCapability
        }
        SemanticChangeKind::TrainingProfile => matches!(
            edge,
            DependencyEdgeKind::ProfileCapability | DependencyEdgeKind::InstructionCapability
        ),
        SemanticChangeKind::GlobalIrContract | SemanticChangeKind::Deletion => true,
        SemanticChangeKind::AddressContract => matches!(
            edge,
            DependencyEdgeKind::Address
                | DependencyEdgeKind::HardwareChannel
                | DependencyEdgeKind::FutureHmiBinding
        ),
        SemanticChangeKind::HardwareChannel => edge == DependencyEdgeKind::HardwareChannel,
        SemanticChangeKind::NetworkAssignment => edge == DependencyEdgeKind::NetworkAssignment,
        SemanticChangeKind::SchedulingDeclaration => edge == DependencyEdgeKind::Declaration,
    }
}

#[test]
fn generated_change_matrix_reports_no_under_or_over_invalidation() {
    let graph = generated_graph();
    for change_kind in CHANGE_KINDS {
        let plan = graph.explain_change(SemanticChange {
            node: ROOT,
            kind: change_kind,
        });
        let mut expected = BTreeSet::new();
        let source_only = matches!(
            change_kind,
            SemanticChangeKind::PublicName | SemanticChangeKind::PresentationOnly
        );
        if !source_only {
            expected.insert(ROOT);
        }
        let mut any_dependent = false;
        for (index, edge_kind) in EDGE_KINDS.into_iter().enumerate() {
            if oracle_edge_affected(edge_kind, change_kind) {
                expected.insert(dependent(index));
                any_dependent = true;
            }
        }
        if any_dependent {
            expected.insert(TRANSITIVE);
        }
        assert_eq!(
            plan.semantic_nodes(),
            expected,
            "under/over invalidation for {change_kind:?}"
        );
        assert!(!plan.semantic_nodes().contains(&UNRELATED));
        if source_only {
            assert_eq!(plan.invalidations.len(), 1);
            assert_eq!(
                plan.invalidations[0].effect,
                InvalidationEffect::SourceIndexOnly
            );
        }
        for invalidation in &plan.invalidations {
            assert_eq!(invalidation.dependency_path[0], invalidation.node);
            assert_eq!(invalidation.dependency_path.last(), Some(&ROOT));
        }
    }
}

#[test]
fn deletion_keeps_every_usage_and_location_as_unresolved() {
    let mut graph = generated_graph();
    let before: BTreeSet<_> = graph.dependents_of(ROOT).copied().collect();
    let converted = graph.mark_deleted(ROOT).unwrap();
    assert_eq!(converted.len(), EDGE_KINDS.len());
    let after: BTreeSet<_> = graph
        .unresolved_edges()
        .into_iter()
        .filter(|edge| edge.dependency == ROOT)
        .collect();
    assert_eq!(before.len(), after.len());
    for prior in before {
        let mut expected = prior;
        expected.resolution = DependencyResolution::Unresolved;
        assert!(after.contains(&expected));
        assert_eq!(expected.location, prior.location);
    }
}

#[test]
fn graph_rejects_invalid_identity_and_source_range_combinations() {
    let mut graph = SemanticDependencyGraph::default();
    graph.insert_node(ROOT);
    let missing = DependencyNodeId::ExternalSemanticUnit(2);
    assert_eq!(
        graph.insert_edge(SemanticDependencyEdge {
            dependent: missing,
            dependency: ROOT,
            kind: DependencyEdgeKind::Call,
            location: None,
            resolution: DependencyResolution::Resolved,
        }),
        Err(DependencyGraphError::MissingDependent(missing))
    );
    assert_eq!(
        graph.insert_edge(SemanticDependencyEdge {
            dependent: ROOT,
            dependency: missing,
            kind: DependencyEdgeKind::Call,
            location: None,
            resolution: DependencyResolution::Resolved,
        }),
        Err(DependencyGraphError::MissingResolvedDependency(missing))
    );
    assert_eq!(
        graph.insert_edge(SemanticDependencyEdge {
            dependent: ROOT,
            dependency: missing,
            kind: DependencyEdgeKind::Call,
            location: Some(DependencyLocation::SourceOccurrence {
                owner: BlockId::new(1),
                occurrence: DependencyOccurrenceId::External(1),
                utf8_start: Some(8),
                utf8_end: Some(7),
            }),
            resolution: DependencyResolution::Unresolved,
        }),
        Err(DependencyGraphError::InvalidSourceRange { start: 8, end: 7 })
    );
}

#[test]
fn validation_preserves_a_missing_call_as_an_unresolved_typed_edge() {
    let owner = BlockId::new(10);
    let missing = BlockId::new(20);
    let mut main = ProgramBlock::new(
        owner,
        "Main",
        EngineeringNumber::new(1).unwrap(),
        ProgramUnitKind::OrganizationBlock(ObDeclaration::CyclicMain),
        BlockInterface::default(),
    );
    main.calls.push(CallSite {
        id: CallSiteId::new(30),
        instruction: CALL_FC,
        callee: missing,
        bindings: Vec::new(),
        instance_owner: None,
    });
    let mut program = ControllerProgram::new(ControllerId::new(40));
    program.insert_block(main).unwrap();
    let report = validate_program(&program);
    assert!(report.has(IssueCode::MissingCallee));
    assert!(report.semantic_dependency_graph.edges().iter().any(|edge| {
        edge.dependent == DependencyNodeId::Object(owner)
            && edge.dependency == DependencyNodeId::Object(missing)
            && edge.kind == DependencyEdgeKind::Call
            && edge.resolution == DependencyResolution::Unresolved
            && matches!(
                edge.location,
                Some(DependencyLocation::SourceOccurrence {
                    occurrence: DependencyOccurrenceId::CallSite(call_site),
                    ..
                }) if call_site == CallSiteId::new(30)
            )
    }));
}

#[test]
fn validation_preserves_a_missing_instance_type_without_panicking() {
    let main_id = BlockId::new(100);
    let instance_id = BlockId::new(101);
    let missing_fb = BlockId::new(102);
    let main = ProgramBlock::new(
        main_id,
        "Main",
        EngineeringNumber::new(1).unwrap(),
        ProgramUnitKind::OrganizationBlock(ObDeclaration::CyclicMain),
        BlockInterface::default(),
    );
    let instance = ProgramBlock::new(
        instance_id,
        "BrokenInstance",
        EngineeringNumber::new(1).unwrap(),
        ProgramUnitKind::DataBlock(DataBlockKind::Instance {
            fb_type: missing_fb,
        }),
        BlockInterface::default(),
    );
    let mut program = ControllerProgram::new(ControllerId::new(103));
    program.insert_block(main).unwrap();
    program.insert_block(instance).unwrap();
    let report = validate_program(&program);
    assert!(report.has(IssueCode::InvalidInstanceDb));
    assert!(report.semantic_dependency_graph.edges().iter().any(|edge| {
        edge.dependent == DependencyNodeId::Object(instance_id)
            && edge.dependency == DependencyNodeId::Object(missing_fb)
            && edge.kind == DependencyEdgeKind::Instance
            && edge.resolution == DependencyResolution::Unresolved
    }));
}
