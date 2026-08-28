use std::collections::{BTreeMap, BTreeSet};

use plc_compiler::{
    CompositionError, FrontendArtifact, GraphSourceIds, Hash32, IrBasicBlock, IrBasicBlockId,
    IrFunction, IrOperation, IrOperationId, IrOperationKind, IrTerminator, IrTerminatorKind,
    IrType, IrValue, IrValueId, ProbeDefinition, ProbeId, ProbeKind, ProbeTable, ResourceLimits,
    SclFrontendError, SclSource, SemanticNodeId, SourceAnchor, SourceLanguage, SourceMapEntry,
    SourceMapId, SourceMapSite, SourceMapTable, TYPED_IR_VERSION, TextRange, TypedIrProgram,
    compose_frontend_artifacts, lower_scl_frontend_artifact, verify_typed_ir,
};
use plc_program::{
    BlockId, BlockInterface, CanonicalValue, ControllerId, ControllerProgram, EngineeringNumber,
    ObDeclaration, ProgramBlock, ProgramUnitKind,
};

fn controller(owners: &[BlockId]) -> ControllerProgram {
    let mut program = ControllerProgram::new(ControllerId::new(0xc0de));
    for (index, &owner) in owners.iter().enumerate() {
        let engineering_number = u16::try_from(index + 1)
            .ok()
            .and_then(EngineeringNumber::new)
            .expect("test engineering number is valid");
        program
            .insert_block(ProgramBlock::new(
                owner,
                format!("FC{}", index + 1),
                engineering_number,
                ProgramUnitKind::Function,
                BlockInterface::default(),
            ))
            .expect("test owners are unique");
    }
    program
}

fn anchor(owner: BlockId, language: SourceLanguage, semantic_node: u32) -> SourceAnchor {
    let revision = Hash32::from_bytes([owner.get().to_le_bytes()[0]; 32]);
    match language {
        SourceLanguage::Scl => SourceAnchor::scl(
            owner,
            revision,
            SemanticNodeId::new(semantic_node),
            TextRange::new(semantic_node, semantic_node + 1).expect("ordered test range"),
        ),
        SourceLanguage::Lad | SourceLanguage::Fbd => SourceAnchor::graph(
            owner,
            revision,
            language,
            SemanticNodeId::new(semantic_node),
            GraphSourceIds {
                network_id: Some(owner.get() + 100),
                node_id: Some(owner.get() + u128::from(semantic_node)),
                ..GraphSourceIds::default()
            },
        )
        .expect("non-SCL graph anchor"),
    }
}

#[allow(clippy::too_many_lines)]
fn frontend(
    program: &ControllerProgram,
    owner: BlockId,
    language: SourceLanguage,
) -> FrontendArtifact {
    let block_id = IrBasicBlockId::new(1);
    let operation_id = IrOperationId::new(1);
    let value_id = IrValueId::new(1);
    let operation_site = SourceMapSite {
        function: owner,
        basic_block: block_id,
        operation: Some(operation_id),
    };
    let return_site = SourceMapSite {
        function: owner,
        basic_block: block_id,
        operation: None,
    };
    let operation_map = SourceMapId::new(1);
    let return_map = SourceMapId::new(2);
    let operation_probe = ProbeId::new(1);
    let return_probe = ProbeId::new(2);
    let source_maps = SourceMapTable::from_untrusted_entries(BTreeMap::from([
        (
            operation_map,
            SourceMapEntry {
                id: operation_map,
                site: operation_site,
                anchors: vec![anchor(owner, language, 1)],
                compiler_generated: false,
            },
        ),
        (
            return_map,
            SourceMapEntry {
                id: return_map,
                site: return_site,
                anchors: vec![anchor(owner, language, 2)],
                compiler_generated: false,
            },
        ),
    ]));
    let probes = ProbeTable::from_untrusted_entries(BTreeMap::from([
        (
            operation_probe,
            ProbeDefinition {
                id: operation_probe,
                site: operation_site,
                kind: ProbeKind::Constant,
                value_type: Some(IrType::Bool),
                source_map: operation_map,
            },
        ),
        (
            return_probe,
            ProbeDefinition {
                id: return_probe,
                site: return_site,
                kind: ProbeKind::Return,
                value_type: None,
                source_map: return_map,
            },
        ),
    ]));
    let ir = TypedIrProgram::from_untrusted_parts(
        TYPED_IR_VERSION,
        BTreeMap::from([(
            owner,
            IrFunction {
                owner,
                source_kind: program.block(owner).expect("known test owner").kind,
                entry: block_id,
                blocks: BTreeMap::from([(
                    block_id,
                    IrBasicBlock {
                        id: block_id,
                        operations: vec![IrOperation {
                            id: operation_id,
                            result: Some(IrValue {
                                id: value_id,
                                data_type: IrType::Bool,
                            }),
                            kind: IrOperationKind::Constant(CanonicalValue::Bool(true)),
                            source_map: operation_map,
                            probe: operation_probe,
                        }],
                        terminator: IrTerminator {
                            kind: IrTerminatorKind::Return,
                            source_map: return_map,
                            probe: return_probe,
                        },
                    },
                )]),
            },
        )]),
    );
    let verified_ir =
        verify_typed_ir(ir, &source_maps, &probes, program).expect("valid test frontend");
    FrontendArtifact::new(owner, language, verified_ir, source_maps, probes)
}

fn all_anchors(artifacts: &[FrontendArtifact]) -> Vec<SourceAnchor> {
    let mut anchors: Vec<_> = artifacts
        .iter()
        .flat_map(|artifact| artifact.source_maps().entries().values())
        .flat_map(|entry| entry.anchors.iter().cloned())
        .collect();
    anchors.sort();
    anchors
}

#[test]
fn mixed_scl_lad_fbd_merge_is_order_independent_and_preserves_anchors() {
    let scl = BlockId::new(30);
    let lad = BlockId::new(10);
    let fbd = BlockId::new(20);
    let program = controller(&[scl, lad, fbd]);
    let artifacts = vec![
        frontend(&program, fbd, SourceLanguage::Fbd),
        frontend(&program, scl, SourceLanguage::Scl),
        frontend(&program, lad, SourceLanguage::Lad),
    ];
    let expected_anchors = all_anchors(&artifacts);
    let composed =
        compose_frontend_artifacts(&program, &artifacts).expect("mixed composition succeeds");

    assert_eq!(composed.verified_ir().program().functions().len(), 3);
    assert_eq!(composed.source_maps().entries().len(), 6);
    assert_eq!(composed.probes().entries().len(), 6);
    assert_eq!(
        composed.owner_languages(),
        &BTreeMap::from([
            (lad, SourceLanguage::Lad),
            (fbd, SourceLanguage::Fbd),
            (scl, SourceLanguage::Scl),
        ])
    );
    let mut actual_anchors: Vec<_> = composed
        .source_maps()
        .entries()
        .values()
        .flat_map(|entry| entry.anchors.iter().cloned())
        .collect();
    actual_anchors.sort();
    assert_eq!(actual_anchors, expected_anchors);

    let reversed: Vec<_> = artifacts.iter().rev().cloned().collect();
    let recomposed = compose_frontend_artifacts(&program, &reversed)
        .expect("caller order cannot affect composition");
    assert_eq!(recomposed, composed);
}

#[test]
fn colliding_frontend_ids_are_rekeyed_into_global_namespaces() {
    let owners = [BlockId::new(1), BlockId::new(2), BlockId::new(3)];
    let program = controller(&owners);
    let artifacts = vec![
        frontend(&program, owners[0], SourceLanguage::Scl),
        frontend(&program, owners[1], SourceLanguage::Lad),
        frontend(&program, owners[2], SourceLanguage::Fbd),
    ];
    let composed = compose_frontend_artifacts(&program, &artifacts).expect("composition succeeds");

    let mut basic_blocks = BTreeSet::new();
    let mut operations = BTreeSet::new();
    let mut values = BTreeSet::new();
    for function in composed.verified_ir().program().functions().values() {
        for block in function.blocks.values() {
            assert!(basic_blocks.insert(block.id));
            for operation in &block.operations {
                assert!(operations.insert(operation.id));
                assert!(values.insert(operation.result.as_ref().expect("test result").id));
            }
        }
    }
    assert_eq!(
        basic_blocks,
        [1, 2, 3].map(IrBasicBlockId::new).into_iter().collect()
    );
    assert_eq!(
        operations,
        [1, 2, 3].map(IrOperationId::new).into_iter().collect()
    );
    assert_eq!(values, [1, 2, 3].map(IrValueId::new).into_iter().collect());
    assert_eq!(
        composed
            .source_maps()
            .entries()
            .keys()
            .map(|id| id.get())
            .collect::<Vec<_>>(),
        vec![1, 2, 3, 4, 5, 6]
    );
    assert_eq!(
        composed
            .probes()
            .entries()
            .keys()
            .map(|id| id.get())
            .collect::<Vec<_>>(),
        vec![1, 2, 3, 4, 5, 6]
    );
}

#[test]
fn unknown_and_duplicate_owners_fail_closed() {
    let known = BlockId::new(1);
    let unknown = BlockId::new(99);
    let target = controller(&[known]);
    let source = controller(&[known, unknown]);
    let unknown_inputs = vec![
        frontend(&target, known, SourceLanguage::Scl),
        frontend(&source, unknown, SourceLanguage::Fbd),
    ];
    assert_eq!(
        compose_frontend_artifacts(&target, &unknown_inputs),
        Err(CompositionError::UnknownOwner(unknown))
    );

    let artifact = frontend(&target, known, SourceLanguage::Lad);
    assert_eq!(
        compose_frontend_artifacts(&target, &[artifact.clone(), artifact]),
        Err(CompositionError::DuplicateOwner(known))
    );
}

#[test]
fn supplied_verification_is_rechecked_and_merged_program_is_verified_again() {
    let owner = BlockId::new(7);
    let program = controller(&[owner]);
    let artifact = frontend(&program, owner, SourceLanguage::Scl);
    let composed = compose_frontend_artifacts(&program, core::slice::from_ref(&artifact))
        .expect("trusted artifact composes");
    let independently_reverified = verify_typed_ir(
        composed.verified_ir().program().clone(),
        composed.source_maps(),
        composed.probes(),
        &program,
    )
    .expect("merged IR passes the shared verifier");
    assert_eq!(
        independently_reverified.verification_hash(),
        composed.verified_ir().verification_hash()
    );

    let mut changed_entries = artifact.source_maps().entries().clone();
    changed_entries
        .get_mut(&SourceMapId::new(1))
        .expect("test mapping exists")
        .anchors[0]
        .source_revision_hash = Hash32::from_bytes([0xa5; 32]);
    let changed_maps = SourceMapTable::from_untrusted_entries(changed_entries);
    let mismatched = FrontendArtifact::new(
        owner,
        artifact.language(),
        artifact.verified_ir().clone(),
        changed_maps,
        artifact.probes().clone(),
    );
    assert!(matches!(
        compose_frontend_artifacts(&program, &[mismatched]),
        Err(CompositionError::InputVerificationHashMismatch {
            owner: rejected,
            ..
        }) if rejected == owner
    ));
}

#[test]
fn canonical_scl_helper_runs_real_contextual_frontend_and_returns_typed_failures() {
    let source_owner = BlockId::new(41);
    let target_owner = BlockId::new(42);
    let cyclic_main = BlockId::new(40);
    let mut program = controller(&[source_owner, target_owner]);
    program
        .insert_block(ProgramBlock::new(
            cyclic_main,
            "Main",
            EngineeringNumber::new(1).expect("valid OB number"),
            ProgramUnitKind::OrganizationBlock(ObDeclaration::CyclicMain),
            BlockInterface::default(),
        ))
        .expect("unique cyclic main");
    let source = SclSource::new(source_owner, "FC2(); RETURN;");
    let artifact = lower_scl_frontend_artifact(&program, &source, ResourceLimits::default())
        .expect("canonical SCL call lowers with controller context");

    assert_eq!(artifact.owner(), source_owner);
    assert_eq!(artifact.language(), SourceLanguage::Scl);
    assert!(artifact.source_maps().entries().values().all(|entry| {
        entry
            .anchors
            .iter()
            .all(|anchor| anchor.language == SourceLanguage::Scl)
    }));
    assert!(
        artifact
            .verified_ir()
            .program()
            .functions()
            .get(&source_owner)
            .expect("caller function exists")
            .blocks
            .values()
            .flat_map(|block| &block.operations)
            .any(|operation| matches!(
                operation.kind,
                IrOperationKind::CallBlock { target, .. } if target == target_owner
            ))
    );

    let malformed = SclSource::new(source_owner, "RETURN");
    assert!(matches!(
        lower_scl_frontend_artifact(&program, &malformed, ResourceLimits::default()),
        Err(SclFrontendError::Diagnostics(issues)) if !issues.is_empty()
    ));
}
