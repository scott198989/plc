use std::collections::BTreeMap;

use plc_core::{ObjectId, PayloadValue, Uuid};
use plc_lad::{LadFormalRef, LadNodeKind, LadPinDirection, LadPortStatus};
use plc_language_tools::{InstanceIdentity, NodeKind};
use plc_program::{CALL_FB, CALL_FC, InstanceOwner, StateKind, VariableRef};
use plc_system::{
    AuthoredLanguage, CANONICAL_FBD_GRAPH_SCHEMA, CANONICAL_LAD_GRAPH_SCHEMA, DecodedGraphicalBody,
    GraphicalBodyHook, decode_graphical_body,
};

fn identity(value: u128) -> String {
    let hex = format!("{value:032x}");
    format!(
        "{}-{}-{}-{}-{}",
        &hex[..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..]
    )
}

fn id(value: u128) -> PayloadValue {
    PayloadValue::String(identity(value))
}

fn text(value: &str) -> PayloadValue {
    PayloadValue::String(value.to_owned())
}

fn record(fields: impl IntoIterator<Item = (&'static str, PayloadValue)>) -> PayloadValue {
    PayloadValue::Record(
        fields
            .into_iter()
            .map(|(key, value)| (key.to_owned(), value))
            .collect(),
    )
}

fn list(values: impl IntoIterator<Item = PayloadValue>) -> PayloadValue {
    PayloadValue::List(values.into_iter().collect())
}

fn hook(language: AuthoredLanguage, graph: PayloadValue) -> GraphicalBodyHook {
    GraphicalBodyHook {
        owner_object_id: ObjectId(Uuid::parse(&identity(1)).expect("valid fixture UUID")),
        owner_block_id: plc_program::BlockId::new(2),
        language,
        payload_schema: "edu.program-block/1".to_owned(),
        semantic_payload: BTreeMap::from([("graph".to_owned(), graph)]),
    }
}

fn caller_binding(operand_id: u128, member_id: u128) -> PayloadValue {
    record([
        ("id", id(operand_id)),
        ("kind", text("caller-member")),
        ("memberId", id(member_id)),
    ])
}

fn active_pin(
    pin_id: u128,
    formal_kind: &'static str,
    formal_id: PayloadValue,
    binding: PayloadValue,
) -> PayloadValue {
    record([
        ("id", id(pin_id)),
        ("formalKind", text(formal_kind)),
        ("formalId", formal_id),
        ("name", text("IN")),
        ("direction", text("input")),
        ("dataType", text("BOOL")),
        ("required", PayloadValue::Bool(true)),
        ("status", text("active")),
        ("binding", binding),
    ])
}

fn power_ports() -> PayloadValue {
    list([])
}

fn lad_box_node() -> PayloadValue {
    record([
        ("id", id(10)),
        ("semanticOrder", PayloadValue::Unsigned(0)),
        ("nodeKind", text("box")),
        ("instructionCode", PayloadValue::Unsigned(0x0100)),
        (
            "pins",
            list([active_pin(
                11,
                "instruction",
                PayloadValue::Unsigned(1),
                caller_binding(12, 13),
            )]),
        ),
        (
            "state",
            record([
                ("invocationId", id(14)),
                ("stateKind", text("timer")),
                (
                    "storage",
                    record([
                        ("kind", text("data-block-member")),
                        ("dataBlockId", id(15)),
                        ("memberId", id(16)),
                    ]),
                ),
            ]),
        ),
        ("powerPorts", power_ports()),
    ])
}

fn lad_fb_call_node(invalid_static_member: bool) -> PayloadValue {
    let static_member = if invalid_static_member {
        PayloadValue::Unsigned(99)
    } else {
        id(42)
    };
    record([
        ("id", id(20)),
        ("semanticOrder", PayloadValue::Unsigned(1)),
        ("nodeKind", text("call")),
        ("callSiteId", id(21)),
        (
            "instructionCode",
            PayloadValue::Unsigned(u64::from(CALL_FB.0)),
        ),
        ("targetBlockId", id(22)),
        (
            "instance",
            record([
                (
                    "owner",
                    record([
                        ("kind", text("multi-instance")),
                        ("ownerFbId", id(23)),
                        ("staticMemberId", static_member),
                    ]),
                ),
                (
                    "path",
                    record([
                        ("rootInstanceDbId", id(24)),
                        ("multiInstanceMemberIds", list([id(41), id(42)])),
                    ]),
                ),
            ]),
        ),
        (
            "pins",
            list([active_pin(
                25,
                "block-member",
                id(26),
                caller_binding(27, 28),
            )]),
        ),
        ("powerPorts", power_ports()),
    ])
}

fn lad_fc_call_node() -> PayloadValue {
    record([
        ("id", id(30)),
        ("semanticOrder", PayloadValue::Unsigned(2)),
        ("nodeKind", text("call")),
        ("callSiteId", id(31)),
        (
            "instructionCode",
            PayloadValue::Unsigned(u64::from(CALL_FC.0)),
        ),
        ("targetBlockId", id(32)),
        ("instance", PayloadValue::Null),
        ("pins", list([])),
        ("powerPorts", power_ports()),
    ])
}

fn lad_graph(invalid_static_member: bool) -> PayloadValue {
    record([
        ("schema", text(CANONICAL_LAD_GRAPH_SCHEMA)),
        ("documentId", id(3)),
        ("semanticRevision", PayloadValue::Unsigned(7)),
        (
            "networks",
            list([record([
                ("id", id(4)),
                ("semanticOrder", PayloadValue::Unsigned(0)),
                (
                    "nodes",
                    list([
                        lad_box_node(),
                        lad_fb_call_node(invalid_static_member),
                        lad_fc_call_node(),
                    ]),
                ),
                ("edges", list([])),
                ("branches", list([])),
            ])]),
        ),
    ])
}

#[test]
fn decodes_lad_box_state_ordered_pins_and_fc_fb_calls() {
    let DecodedGraphicalBody::Lad(document) =
        decode_graphical_body(&hook(AuthoredLanguage::Lad, lad_graph(false)))
            .expect("advanced LAD graph decodes")
    else {
        panic!("expected LAD document");
    };
    assert_eq!(document.semantic_revision, 7);
    let network = document.networks.values().next().expect("fixture network");

    let LadNodeKind::Box(box_node) = &network.nodes[&plc_lad::LadNodeId::new(10)].kind else {
        panic!("expected box node");
    };
    assert_eq!(box_node.ordered_pin_ids, [plc_lad::LadPortId::new(11)]);
    let pin = &box_node.pins[&plc_lad::LadPortId::new(11)];
    assert_eq!(pin.direction, LadPinDirection::Input);
    assert_eq!(pin.status, LadPortStatus::Active);
    assert_eq!(
        pin.formal,
        Some(LadFormalRef::Instruction(plc_program::InstructionFormalId(
            1
        )))
    );
    let state = box_node.state.as_ref().expect("explicit state binding");
    assert_eq!(state.kind, StateKind::Timer);
    assert!(matches!(
        &state.storage,
        VariableRef::DataBlockMember { .. }
    ));

    let LadNodeKind::Call(function_block_call) = &network.nodes[&plc_lad::LadNodeId::new(20)].kind
    else {
        panic!("expected FB call");
    };
    assert_eq!(function_block_call.instruction, CALL_FB);
    let instance = function_block_call.instance.as_ref().expect("FB instance");
    assert!(matches!(
        instance.owner,
        InstanceOwner::MultiInstance { .. }
    ));
    assert_eq!(instance.path.multi_instance_slots.len(), 2);

    let LadNodeKind::Call(function_call) = &network.nodes[&plc_lad::LadNodeId::new(30)].kind else {
        panic!("expected FC call");
    };
    assert_eq!(function_call.instruction, CALL_FC);
    assert!(function_call.instance.is_none());
}

fn fbd_graph() -> PayloadValue {
    let instruction = record([
        ("id", id(50)),
        ("semanticOrder", PayloadValue::Unsigned(0)),
        ("nodeKind", text("instruction")),
        ("instructionCode", PayloadValue::Unsigned(0x0100)),
        (
            "instance",
            record([
                ("kind", text("instruction-state")),
                ("stateInstanceId", id(51)),
            ]),
        ),
        ("ports", list([])),
    ]);
    let call = record([
        ("id", id(60)),
        ("semanticOrder", PayloadValue::Unsigned(1)),
        ("nodeKind", text("call")),
        (
            "instructionCode",
            PayloadValue::Unsigned(u64::from(CALL_FB.0)),
        ),
        ("targetBlockId", id(61)),
        (
            "instance",
            record([
                ("kind", text("function-block")),
                ("rootInstanceDbId", id(62)),
                ("multiInstanceMemberIds", list([id(63), id(64)])),
            ]),
        ),
        ("ports", list([])),
    ]);
    record([
        ("schema", text(CANONICAL_FBD_GRAPH_SCHEMA)),
        ("documentId", id(5)),
        (
            "networks",
            list([record([
                ("id", id(6)),
                ("semanticOrder", PayloadValue::Unsigned(0)),
                ("nodes", list([instruction, call])),
                ("connections", list([])),
            ])]),
        ),
    ])
}

#[test]
fn decodes_fbd_instruction_state_and_nested_fb_instance_path() {
    let DecodedGraphicalBody::Fbd(document) =
        decode_graphical_body(&hook(AuthoredLanguage::Fbd, fbd_graph()))
            .expect("advanced FBD graph decodes")
    else {
        panic!("expected FBD document");
    };
    let network = document.networks.values().next().expect("fixture network");
    assert!(matches!(
        &network.nodes[&plc_language_tools::NodeId::new(50)].kind,
        NodeKind::Instruction {
            instance: Some(InstanceIdentity::Instruction(_)),
            ..
        }
    ));
    let NodeKind::Call {
        instance:
            Some(InstanceIdentity::FunctionBlock {
                root_instance_db,
                multi_instance_members,
            }),
        ..
    } = &network.nodes[&plc_language_tools::NodeId::new(60)].kind
    else {
        panic!("expected function-block call instance");
    };
    assert_eq!(root_instance_db.get(), 62);
    assert_eq!(multi_instance_members.len(), 2);
}

#[test]
fn malformed_advanced_identity_reports_its_exact_semantic_path() {
    let error = decode_graphical_body(&hook(AuthoredLanguage::Lad, lad_graph(true)))
        .expect_err("numeric static member identity must not decode");
    assert_eq!(
        error.semantic_path,
        "graph.networks[0].nodes[1].instance.owner.staticMemberId"
    );
}
