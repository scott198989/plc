use std::collections::BTreeMap;

use plc_compiler::{
    GraphSourceIds, Hash32, IrBasicBlockId, IrOperationId, ResourceLimits, RuntimeMappedSite,
    SclSource, SemanticNodeId, SourceAnchor, SourceAnchorResolution, SourceAnchorUnavailableReason,
    SourceLanguage, SourceMapEntry, SourceMapId, SourceMapSite, SourceMapTable, TextRange,
    lower_scl_frontend_artifact, project_verified_ir_to_runtime, scl::analyze_scl_with_program,
};
use plc_language_tools::{
    ActivationRole, ConnectionId, ConnectionKind, EffectRole, FbdConnection, FbdDocument,
    FbdDocumentId, FbdNetwork, FbdNode, FbdPort, NetworkId, NodeId, NodeKind, PortDirection,
    PortId, PortMultiplicity, PortStatus, lower_fbd_to_verified_ir,
};
use plc_program::{
    BlockId, BlockInterface, CanonicalValue as ProgramValue, ControllerId, ControllerProgram,
    DIVIDE, DataType, EngineeringNumber, FORMAL_LEFT, FORMAL_OUTPUT, FORMAL_RIGHT, InterfaceMember,
    InterfaceMemberId, InterfaceRole, ObDeclaration, ProgramBlock, ProgramUnitKind,
};
use plc_runtime::{
    CpuState, DiagnosticCode as RuntimeDiagnosticCode, RestartKind, RunOutcome, UniverseId,
    VirtualController, VirtualControllerId,
};

const OWNER: BlockId = BlockId::new(1);
const INPUT: InterfaceMemberId = InterfaceMemberId::new(10);
const OUTPUT: InterfaceMemberId = InterfaceMemberId::new(11);

fn number(value: u16) -> EngineeringNumber {
    EngineeringNumber::new(value).expect("nonzero engineering number")
}

fn scalar_block(owner: BlockId, name: &str, number_value: u16) -> ProgramBlock {
    ProgramBlock::new(
        owner,
        name,
        number(number_value),
        ProgramUnitKind::Function,
        BlockInterface::from_members([
            InterfaceMember::plain(INPUT, "Input", InterfaceRole::Input, DataType::Bool, 0),
            InterfaceMember::plain(OUTPUT, "Output", InterfaceRole::Output, DataType::Bool, 0),
        ]),
    )
}

fn scalar_program() -> ControllerProgram {
    let mut program = ControllerProgram::new(ControllerId::new(1));
    program
        .insert_block(ProgramBlock::new(
            BlockId::new(999),
            "Main",
            number(1),
            ProgramUnitKind::OrganizationBlock(ObDeclaration::CyclicMain),
            BlockInterface::default(),
        ))
        .expect("unique cyclic main");
    program
        .insert_block(scalar_block(OWNER, "Logic", 1))
        .expect("unique block");
    program
}

fn anchor_for_text(
    artifact: &plc_compiler::FrontendArtifact,
    source: &SclSource,
    expected: &str,
) -> SourceAnchor {
    artifact
        .source_maps()
        .entries()
        .values()
        .flat_map(|entry| &entry.anchors)
        .find(|anchor| {
            anchor.text_range.and_then(|range| source.range_text(range)) == Some(expected)
        })
        .cloned()
        .expect("expected authored anchor")
}

#[test]
fn textual_ranges_are_zero_based_utf8_bytes_across_mixed_line_endings() {
    let text = "// µ\r\nOutput := Input;\n";
    let source = SclSource::new(OWNER, text);
    let mu = u32::try_from(text.find('µ').expect("multibyte marker")).expect("small source");
    let output = u32::try_from(text.find("Output").expect("output token")).expect("small source");
    let input = u32::try_from(text.find("Input").expect("input token")).expect("small source");

    assert!(
        source.line_column(mu + 1).is_none(),
        "inside UTF-8 code point"
    );
    assert_eq!(source.line_column(output).expect("line start").line, 2);
    assert_eq!(source.line_column(output).expect("line start").column, 1);
    assert_eq!(source.line_column(input).expect("input position").line, 2);
    assert_eq!(
        source.range_text(TextRange::new(input, input + 5).expect("ordered range")),
        Some("Input")
    );
    assert_eq!(
        source.range_text(TextRange::empty(
            u32::try_from(text.len()).expect("small source")
        )),
        Some("")
    );
    assert!(
        source
            .range_text(TextRange::new(mu + 1, mu + 2).expect("ordered range"))
            .is_none()
    );
}

#[test]
#[allow(clippy::too_many_lines)]
fn scl_relocation_ignores_trivia_but_rejects_semantic_replacement() {
    let program = scalar_program();
    let before_source = SclSource::new(OWNER, "Output := Input;");
    let formatted_source = SclSource::new(
        OWNER,
        "// µ\r\nOutput\t:= (* formatting only *) Input ;\r\n",
    );
    let changed_source = SclSource::new(OWNER, "Output := FALSE;");
    let before = lower_scl_frontend_artifact(&program, &before_source, ResourceLimits::default())
        .expect("before source lowers");
    let formatted =
        lower_scl_frontend_artifact(&program, &formatted_source, ResourceLimits::default())
            .expect("formatted source lowers");
    let changed = lower_scl_frontend_artifact(&program, &changed_source, ResourceLimits::default())
        .expect("changed source lowers");
    let before_semantics = analyze_scl_with_program(
        &before_source,
        program.block(OWNER).expect("known owner"),
        &program,
        ResourceLimits::default(),
    );
    let formatted_semantics = analyze_scl_with_program(
        &formatted_source,
        program.block(OWNER).expect("known owner"),
        &program,
        ResourceLimits::default(),
    );
    let changed_semantics = analyze_scl_with_program(
        &changed_source,
        program.block(OWNER).expect("known owner"),
        &program,
        ResourceLimits::default(),
    );
    let loaded_anchor = anchor_for_text(&before, &before_source, "Input");

    assert!(matches!(
        formatted
            .source_maps()
            .resolve_source_anchor(&loaded_anchor),
        SourceAnchorResolution::Unavailable(
            SourceAnchorUnavailableReason::TextRelocationGuardRequired
        )
    ));
    let relocated = formatted.source_maps().resolve_scl_source_anchor(
        &loaded_anchor,
        &before_semantics,
        &formatted_semantics,
        ResourceLimits::default(),
    );
    let SourceAnchorResolution::Relocated(relocated) = relocated else {
        panic!("format-only edit should relocate safely");
    };
    assert_eq!(
        relocated
            .anchor
            .text_range
            .and_then(|range| formatted_source.range_text(range)),
        Some("Input")
    );
    assert!(!relocated.sites.is_empty());
    assert_eq!(
        formatted
            .probes()
            .resolved_source_to_probes(&relocated)
            .len(),
        relocated.sites.len()
    );

    assert!(matches!(
        changed.source_maps().resolve_scl_source_anchor(
            &loaded_anchor,
            &before_semantics,
            &changed_semantics,
            ResourceLimits::default(),
        ),
        SourceAnchorResolution::Unavailable(SourceAnchorUnavailableReason::SemanticContentChanged)
    ));

    let constrained_limits = ResourceLimits {
        max_source_bytes_per_block: 1,
        ..ResourceLimits::default()
    };
    let constrained_semantics = analyze_scl_with_program(
        &formatted_source,
        program.block(OWNER).expect("known owner"),
        &program,
        constrained_limits,
    );
    assert!(constrained_semantics.resource_limit().is_some());
    assert!(matches!(
        formatted.source_maps().resolve_scl_source_anchor(
            &loaded_anchor,
            &before_semantics,
            &constrained_semantics,
            ResourceLimits::default(),
        ),
        SourceAnchorResolution::Unavailable(SourceAnchorUnavailableReason::SemanticContentChanged)
    ));

    let mut recreated_program = ControllerProgram::new(ControllerId::new(2));
    recreated_program
        .insert_block(ProgramBlock::new(
            BlockId::new(999),
            "Main",
            number(1),
            ProgramUnitKind::OrganizationBlock(ObDeclaration::CyclicMain),
            BlockInterface::default(),
        ))
        .expect("unique cyclic main");
    recreated_program
        .insert_block(ProgramBlock::new(
            OWNER,
            "Logic",
            number(1),
            ProgramUnitKind::Function,
            BlockInterface::from_members([
                InterfaceMember::plain(
                    InterfaceMemberId::new(99),
                    "Input",
                    InterfaceRole::Input,
                    DataType::Bool,
                    0,
                ),
                InterfaceMember::plain(OUTPUT, "Output", InterfaceRole::Output, DataType::Bool, 0),
            ]),
        ))
        .expect("unique recreated block");
    let recreated = lower_scl_frontend_artifact(
        &recreated_program,
        &formatted_source,
        ResourceLimits::default(),
    )
    .expect("same-named replacement lowers");
    let recreated_semantics = analyze_scl_with_program(
        &formatted_source,
        recreated_program.block(OWNER).expect("known owner"),
        &recreated_program,
        ResourceLimits::default(),
    );
    assert!(matches!(
        recreated.source_maps().resolve_scl_source_anchor(
            &loaded_anchor,
            &before_semantics,
            &recreated_semantics,
            ResourceLimits::default(),
        ),
        SourceAnchorResolution::Unavailable(SourceAnchorUnavailableReason::SemanticContentChanged)
    ));
}

#[test]
fn one_scl_call_anchor_maps_to_every_lowered_call_effect_and_probe() {
    let caller_block = ProgramBlock::new(
        BlockId::new(20),
        "Caller",
        number(20),
        ProgramUnitKind::Function,
        BlockInterface::from_members([
            InterfaceMember::plain(
                InterfaceMemberId::new(201),
                "Arg",
                InterfaceRole::Input,
                DataType::Bool,
                0,
            ),
            InterfaceMember::plain(
                InterfaceMemberId::new(202),
                "Result",
                InterfaceRole::Output,
                DataType::Bool,
                0,
            ),
        ]),
    );
    let mut callee_output = InterfaceMember::plain(
        InterfaceMemberId::new(302),
        "Y",
        InterfaceRole::Output,
        DataType::Bool,
        0,
    );
    callee_output.required_output_binding = true;
    let scale_block = ProgramBlock::new(
        BlockId::new(30),
        "Scale",
        number(30),
        ProgramUnitKind::Function,
        BlockInterface::from_members([
            InterfaceMember::plain(
                InterfaceMemberId::new(301),
                "X",
                InterfaceRole::Input,
                DataType::Bool,
                0,
            ),
            callee_output,
        ]),
    );
    let mut program = ControllerProgram::new(ControllerId::new(20));
    program
        .insert_block(ProgramBlock::new(
            BlockId::new(999),
            "Main",
            number(1),
            ProgramUnitKind::OrganizationBlock(ObDeclaration::CyclicMain),
            BlockInterface::default(),
        ))
        .expect("unique cyclic main");
    program
        .insert_block(caller_block.clone())
        .expect("unique caller");
    program.insert_block(scale_block).expect("unique callee");
    let source = SclSource::new(caller_block.id, "Scale(X := Arg, Y => Result);");
    let artifact = lower_scl_frontend_artifact(&program, &source, ResourceLimits::default())
        .expect("call lowers");
    let call_anchor = artifact
        .source_maps()
        .entries()
        .values()
        .flat_map(|entry| &entry.anchors)
        .find(|anchor| {
            anchor.text_range.and_then(|range| source.range_text(range)) == Some(source.text())
                && anchor.semantic_node_id.get() != 0
        })
        .cloned()
        .expect("call statement anchor");
    let SourceAnchorResolution::Exact(resolved) =
        artifact.source_maps().resolve_source_anchor(&call_anchor)
    else {
        panic!("immutable call anchor should resolve exactly");
    };
    assert_eq!(resolved.sites.len(), 3, "call, returned value, and store");
    assert_eq!(
        artifact.probes().resolved_source_to_probes(&resolved).len(),
        3
    );
}

#[test]
fn generated_empty_program_return_retains_a_zero_width_causal_anchor() {
    let owner = BlockId::new(50);
    let block = ProgramBlock::new(
        owner,
        "Empty",
        number(50),
        ProgramUnitKind::OrganizationBlock(ObDeclaration::CyclicMain),
        BlockInterface::default(),
    );
    let mut program = ControllerProgram::new(ControllerId::new(50));
    program.insert_block(block).expect("unique block");
    let source = SclSource::new(owner, "");
    let artifact = lower_scl_frontend_artifact(&program, &source, ResourceLimits::default())
        .expect("empty body lowers to generated return");
    let entry = artifact
        .source_maps()
        .entries()
        .values()
        .next()
        .expect("generated source map");
    assert!(entry.compiler_generated);
    assert_eq!(entry.anchors.len(), 1);
    assert_eq!(entry.anchors[0].semantic_node_id, SemanticNodeId::new(0));
    assert_eq!(entry.anchors[0].text_range, Some(TextRange::empty(0)));
    assert_eq!(
        artifact.source_maps().source_to_ir(&entry.anchors[0]),
        vec![entry.site]
    );
    assert_eq!(
        artifact
            .probes()
            .source_to_probes(artifact.source_maps(), &entry.anchors[0])
            .len(),
        1
    );
}

#[test]
fn runtime_projection_retains_exact_probe_and_source_binding_for_fault_lookup() {
    let owner = BlockId::new(60);
    let block = ProgramBlock::new(
        owner,
        "Main",
        number(1),
        ProgramUnitKind::OrganizationBlock(ObDeclaration::CyclicMain),
        BlockInterface::from_members([InterfaceMember::plain(
            InterfaceMemberId::new(601),
            "Value",
            InterfaceRole::Temp,
            DataType::Bool,
            0,
        )]),
    );
    let mut program = ControllerProgram::new(ControllerId::new(60));
    program.insert_block(block).expect("unique cyclic main");
    let source = SclSource::new(owner, "Value := TRUE;");
    let artifact = lower_scl_frontend_artifact(&program, &source, ResourceLimits::default())
        .expect("runtime source lowers");
    let projection = project_verified_ir_to_runtime(
        artifact.verified_ir(),
        artifact.source_maps(),
        artifact.probes(),
        &program,
        Hash32::from_bytes([0x60; 32]),
    )
    .expect("verified source projects to runtime");
    assert!(!projection.source_bindings().is_empty());
    for binding in projection.source_bindings() {
        assert_eq!(
            artifact.source_maps().ir_to_source(binding.compiler_site),
            binding.anchors.iter().collect::<Vec<_>>()
        );
        assert_eq!(
            artifact.probes().get(binding.probe).expect("probe").site,
            binding.compiler_site
        );
        let source_identity = match binding.runtime_site {
            RuntimeMappedSite::Instruction {
                source_identity, ..
            }
            | RuntimeMappedSite::Terminator {
                source_identity, ..
            } => source_identity,
            RuntimeMappedSite::BlockReturn { .. } => continue,
        };
        assert_eq!(projection.source_for(source_identity), Some(binding));
        assert!(binding.anchors.iter().all(|anchor| {
            anchor.source_revision_hash == source.revision_hash()
                && anchor
                    .text_range
                    .is_some_and(|range| source.range_text(range).is_some())
        }));
    }
}

#[test]
fn graphical_relocation_uses_stable_graph_ids_and_fails_closed_when_removed() {
    let owner = BlockId::new(70);
    let old = SourceAnchor::graph(
        owner,
        Hash32::from_bytes([1; 32]),
        SourceLanguage::Lad,
        SemanticNodeId::new(1),
        GraphSourceIds {
            network_id: Some(700),
            node_id: Some(701),
            port_id: Some(702),
            edge_id: Some(703),
            ..GraphSourceIds::default()
        },
    )
    .expect("LAD anchor");
    let current = SourceAnchor::graph(
        owner,
        Hash32::from_bytes([2; 32]),
        SourceLanguage::Lad,
        SemanticNodeId::new(99),
        GraphSourceIds {
            network_id: Some(700),
            node_id: Some(701),
            port_id: Some(702),
            edge_id: Some(703),
            ..GraphSourceIds::default()
        },
    )
    .expect("relocated LAD anchor");
    let site = SourceMapSite {
        function: owner,
        basic_block: IrBasicBlockId::new(1),
        operation: Some(IrOperationId::new(1)),
    };
    let table = SourceMapTable::from_untrusted_entries(BTreeMap::from([(
        SourceMapId::new(1),
        SourceMapEntry {
            id: SourceMapId::new(1),
            site,
            anchors: vec![current.clone()],
            compiler_generated: false,
        },
    )]));
    assert_eq!(
        table.resolve_source_anchor(&old),
        SourceAnchorResolution::Relocated(plc_compiler::ResolvedSourceAnchor {
            anchor: current,
            sites: vec![site],
        })
    );

    let removed = SourceAnchor::graph(
        owner,
        Hash32::from_bytes([1; 32]),
        SourceLanguage::Lad,
        SemanticNodeId::new(1),
        GraphSourceIds {
            network_id: Some(700),
            node_id: Some(999),
            ..GraphSourceIds::default()
        },
    )
    .expect("removed LAD anchor");
    assert_eq!(
        table.resolve_source_anchor(&removed),
        SourceAnchorResolution::Unavailable(SourceAnchorUnavailableReason::StableIdentityMissing)
    );
}

fn numeric_fbd_port(
    id: u128,
    name: &str,
    direction: PortDirection,
    formal: Option<plc_compiler::IrFormalRef>,
) -> FbdPort {
    FbdPort {
        id: PortId::new(id),
        name: name.into(),
        direction,
        data_type: Some(DataType::DInt),
        required: direction == PortDirection::Input,
        multiplicity: if direction == PortDirection::Output {
            PortMultiplicity::Many
        } else {
            PortMultiplicity::One
        },
        activation: ActivationRole::None,
        status: PortStatus::Active,
        effect_role: EffectRole::Value,
        formal,
    }
}

fn faulting_fbd(owner: BlockId, result: InterfaceMemberId, edited: bool) -> FbdDocument {
    let numerator = FbdNode::from_ports(
        NodeId::new(801),
        0,
        NodeKind::Constant {
            value: ProgramValue::DInt(10),
        },
        [numeric_fbd_port(8_011, "OUT", PortDirection::Output, None)],
    );
    let denominator = FbdNode::from_ports(
        NodeId::new(802),
        1,
        NodeKind::Constant {
            value: ProgramValue::DInt(0),
        },
        [numeric_fbd_port(8_021, "OUT", PortDirection::Output, None)],
    );
    let divide = FbdNode::from_ports(
        NodeId::new(803),
        2,
        NodeKind::Instruction {
            code: DIVIDE,
            instance: None,
        },
        [
            numeric_fbd_port(
                8_031,
                "A",
                PortDirection::Input,
                Some(plc_compiler::IrFormalRef::Instruction(FORMAL_LEFT)),
            ),
            numeric_fbd_port(
                8_032,
                "B",
                PortDirection::Input,
                Some(plc_compiler::IrFormalRef::Instruction(FORMAL_RIGHT)),
            ),
            numeric_fbd_port(
                8_033,
                "OUT",
                PortDirection::Output,
                Some(plc_compiler::IrFormalRef::Instruction(FORMAL_OUTPUT)),
            ),
        ],
    );
    let store = FbdNode::from_ports(
        NodeId::new(804),
        3,
        NodeKind::StoreMember { member: result },
        [numeric_fbd_port(8_041, "IN", PortDirection::Input, None)],
    );
    let mut nodes = vec![numerator, denominator, divide, store];
    if edited {
        nodes.push(FbdNode::from_ports(
            NodeId::new(805),
            4,
            NodeKind::Constant {
                value: ProgramValue::DInt(99),
            },
            [numeric_fbd_port(8_051, "OUT", PortDirection::Output, None)],
        ));
    }
    FbdDocument::new(
        FbdDocumentId::new(0xF803),
        owner,
        [FbdNetwork::from_parts(
            NetworkId::new(800),
            0,
            nodes,
            [
                FbdConnection {
                    id: ConnectionId::new(8_001),
                    source: PortId::new(8_011),
                    target: PortId::new(8_031),
                    kind: ConnectionKind::Data,
                },
                FbdConnection {
                    id: ConnectionId::new(8_002),
                    source: PortId::new(8_021),
                    target: PortId::new(8_032),
                    kind: ConnectionKind::Data,
                },
                FbdConnection {
                    id: ConnectionId::new(8_003),
                    source: PortId::new(8_033),
                    target: PortId::new(8_041),
                    kind: ConnectionKind::Data,
                },
            ],
        )],
    )
}

#[test]
fn fbd_runtime_fault_relocates_to_same_stable_node_after_offline_graph_edit() {
    let owner = BlockId::new(80);
    let result = InterfaceMemberId::new(801);
    let block = ProgramBlock::new(
        owner,
        "Main",
        number(1),
        ProgramUnitKind::OrganizationBlock(ObDeclaration::CyclicMain),
        BlockInterface::from_members([InterfaceMember::plain(
            result,
            "Result",
            InterfaceRole::Temp,
            DataType::DInt,
            0,
        )]),
    );
    let mut program = ControllerProgram::new(ControllerId::new(80));
    program.insert_block(block).unwrap();

    let loaded = lower_fbd_to_verified_ir(&faulting_fbd(owner, result, false), &program)
        .expect("loaded FBD verifies");
    let projection = project_verified_ir_to_runtime(
        &loaded.verified_ir,
        &loaded.lowered.compiler_source_maps,
        &loaded.lowered.compiler_probes,
        &program,
        Hash32::from_bytes([0x80; 32]),
    )
    .expect("loaded FBD projects to runtime");
    let mut controller = VirtualController::new(UniverseId(80), VirtualControllerId(80), 0x8000);
    controller.power_on().unwrap();
    controller
        .install_verified_artifact(projection.package())
        .unwrap();
    controller.request_run(RestartKind::Resume).unwrap();
    let event = match controller.run_scan().unwrap() {
        RunOutcome::Faulted(event) => event,
        RunOutcome::Completed(report) => panic!("divide-by-zero FBD completed: {report:?}"),
    };
    assert_eq!(event.code, RuntimeDiagnosticCode::ArithmeticDivideByZero);
    assert_eq!(controller.cpu_state(), CpuState::Faulted);
    let context = event
        .fault_context
        .expect("fault retains exact runtime site");
    let binding = projection
        .source_for(context.source_identity)
        .expect("runtime fault source identity maps to compiler source");
    let fault_anchor = binding
        .anchors
        .iter()
        .find(|anchor| {
            anchor.language == SourceLanguage::Fbd
                && anchor.network_id == Some(800)
                && anchor.node_id == Some(803)
        })
        .cloned()
        .expect("faulting DIV node has an authored stable anchor");

    let edited = lower_fbd_to_verified_ir(&faulting_fbd(owner, result, true), &program)
        .expect("offline unrelated-node edit remains valid FBD");
    assert_ne!(
        fault_anchor.source_revision_hash,
        edited
            .lowered
            .compiler_source_maps
            .entries()
            .values()
            .flat_map(|entry| &entry.anchors)
            .find(|anchor| anchor.node_id == Some(803))
            .unwrap()
            .source_revision_hash,
        "offline edit must produce a new immutable graph revision"
    );
    let SourceAnchorResolution::Relocated(relocated) = edited
        .lowered
        .compiler_source_maps
        .resolve_source_anchor(&fault_anchor)
    else {
        panic!("runtime fault anchor must relocate by stable FBD identity");
    };
    assert_eq!(relocated.anchor.language, SourceLanguage::Fbd);
    assert_eq!(relocated.anchor.network_id, Some(800));
    assert_eq!(relocated.anchor.node_id, Some(803));
    assert!(!relocated.sites.is_empty());
    assert_eq!(
        edited
            .lowered
            .compiler_probes
            .resolved_source_to_probes(&relocated)
            .len(),
        relocated.sites.len()
    );
}
