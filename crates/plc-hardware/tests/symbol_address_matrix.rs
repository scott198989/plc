#![allow(clippy::too_many_lines)]

use std::collections::{BTreeMap, BTreeSet};

use plc_hardware::{
    Address, AddressArea, AddressError, AddressIntent, BindingKind, BlockValueRole, CanonicalType,
    ChannelAddress, ChannelDirection, ChannelId, ControllerCatalogId, ControllerConfig,
    ControllerId, Declaration, DeclarationId, DeclarationKind, DiagnosticCode, HardwareArtifact,
    HardwareChannelBinding, HardwareProject, Identifier, IdentifierError, ModuleId, Namespace,
    PlcValue, PrimitiveType, RackConfig, RackOwner, ReferenceId, ReferenceState, Resolution,
    RetainPolicy, Scope, ScopeId, ScopeKind, Sha256Digest, SourceIdentity, SourceObjectId,
    SymbolAddressArea, SymbolError, SymbolUniverse, Tag, TagId, TagKind, TagTable, TagTableId,
    TrainingProfile, Uuid, VirtualDeviceId, VirtualNetwork,
};

#[derive(Default)]
struct Ids {
    next: u64,
}

impl Ids {
    fn next<T: From<Uuid>>(&mut self) -> T {
        self.next += 1;
        T::from(Uuid::deterministic_v4(
            b"plc-hardware-symbol-address-matrix",
            self.next,
        ))
    }
}

#[derive(Clone, Copy)]
struct Channels {
    input_bit_0: ChannelId,
    input_bit_1: ChannelId,
    output_bit_0: ChannelId,
    input_word_2: ChannelId,
    output_word_2: ChannelId,
}

struct Fixture {
    profile: TrainingProfile,
    project: HardwareProject,
    artifact: HardwareArtifact,
    symbols: SymbolUniverse,
    ids: Ids,
    controller_id: ControllerId,
    global_scope: ScopeId,
    table_id: TagTableId,
    channels: Channels,
}

#[allow(clippy::too_many_arguments)]
fn channel_binding(
    ids: &mut Ids,
    controller_id: ControllerId,
    channel_id: ChannelId,
    slot_number: u8,
    channel_index: u8,
    direction: ChannelDirection,
    raw_type: PrimitiveType,
    address: ChannelAddress,
) -> HardwareChannelBinding {
    HardwareChannelBinding {
        controller_id,
        controller_creation_ordinal: 1,
        module_id: ids.next::<ModuleId>(),
        location_rank: 0,
        station_creation_ordinal: 0,
        slot_number,
        module_creation_ordinal: u64::from(slot_number),
        channel_id,
        channel_index,
        direction,
        raw_type,
        address,
    }
}

fn fixture() -> Fixture {
    let profile = TrainingProfile::edu21();
    let mut ids = Ids::default();
    let controller_id = ids.next::<ControllerId>();
    let virtual_device_id = ids.next::<VirtualDeviceId>();
    let rack_id = ids.next();
    let mut project = HardwareProject::new(profile.pin(), VirtualNetwork::new());
    project
        .add_controller(ControllerConfig {
            id: controller_id,
            creation_ordinal: 1,
            catalog_id: ControllerCatalogId::VctrlC1,
            virtual_device_id,
            local_rack: RackConfig {
                id: rack_id,
                creation_ordinal: 1,
                owner: RackOwner::Controller(controller_id),
                slots: BTreeMap::new(),
            },
            reserved_input_spans: Vec::new(),
            reserved_output_spans: Vec::new(),
            configured_block_count: 0,
        })
        .expect("controller fixture");

    let input_word_1 = ids.next::<ChannelId>();
    let channels = Channels {
        input_bit_0: ids.next(),
        input_bit_1: ids.next(),
        output_bit_0: ids.next(),
        input_word_2: ids.next(),
        output_word_2: ids.next(),
    };
    let mut channel_bindings = BTreeMap::new();
    // The lower-address channel intentionally ranks second. Automatic binding
    // must use hardware ordering, not address or map iteration order.
    channel_bindings.insert(
        channels.input_bit_0,
        channel_binding(
            &mut ids,
            controller_id,
            channels.input_bit_0,
            2,
            0,
            ChannelDirection::Input,
            PrimitiveType::Bool,
            ChannelAddress::Bit {
                area: AddressArea::Input,
                byte: 0,
                bit: 0,
            },
        ),
    );
    channel_bindings.insert(
        channels.input_bit_1,
        channel_binding(
            &mut ids,
            controller_id,
            channels.input_bit_1,
            1,
            1,
            ChannelDirection::Input,
            PrimitiveType::Bool,
            ChannelAddress::Bit {
                area: AddressArea::Input,
                byte: 0,
                bit: 1,
            },
        ),
    );
    channel_bindings.insert(
        channels.output_bit_0,
        channel_binding(
            &mut ids,
            controller_id,
            channels.output_bit_0,
            3,
            0,
            ChannelDirection::Output,
            PrimitiveType::Bool,
            ChannelAddress::Bit {
                area: AddressArea::Output,
                byte: 0,
                bit: 0,
            },
        ),
    );
    channel_bindings.insert(
        input_word_1,
        channel_binding(
            &mut ids,
            controller_id,
            input_word_1,
            6,
            0,
            ChannelDirection::Input,
            PrimitiveType::Int,
            ChannelAddress::Word {
                area: AddressArea::Input,
                byte: 1,
            },
        ),
    );
    channel_bindings.insert(
        channels.input_word_2,
        channel_binding(
            &mut ids,
            controller_id,
            channels.input_word_2,
            4,
            0,
            ChannelDirection::Input,
            PrimitiveType::Int,
            ChannelAddress::Word {
                area: AddressArea::Input,
                byte: 2,
            },
        ),
    );
    channel_bindings.insert(
        channels.output_word_2,
        channel_binding(
            &mut ids,
            controller_id,
            channels.output_word_2,
            5,
            0,
            ChannelDirection::Output,
            PrimitiveType::Int,
            ChannelAddress::Word {
                area: AddressArea::Output,
                byte: 2,
            },
        ),
    );
    let artifact = HardwareArtifact {
        profile_pin: profile.pin(),
        hardware_fingerprint: Sha256Digest([0x11; 32]),
        network_configuration_fingerprint: Sha256Digest([0x22; 32]),
        channel_bindings,
    };

    let global_scope = ids.next::<ScopeId>();
    let table_id = ids.next::<TagTableId>();
    let mut symbols = SymbolUniverse::new(profile.pin());
    symbols
        .add_scope(Scope {
            id: global_scope,
            creation_ordinal: 1,
            kind: ScopeKind::ControllerGlobal(controller_id),
            parent_scope_id: None,
        })
        .expect("global scope");
    symbols
        .add_tag_table(TagTable {
            id: table_id,
            controller_id,
            creation_ordinal: 1,
            name: Identifier::parse("DefaultTags").expect("table name"),
            is_default: true,
        })
        .expect("default tag table");

    Fixture {
        profile,
        project,
        artifact,
        symbols,
        ids,
        controller_id,
        global_scope,
        table_id,
        channels,
    }
}

fn declaration_with_id(
    id: DeclarationId,
    ordinal: u64,
    name: &str,
    scope_id: ScopeId,
    kind: DeclarationKind,
) -> Declaration {
    Declaration {
        id,
        creation_ordinal: ordinal,
        name: Identifier::parse(name).expect("declaration name"),
        scope_id,
        namespace: kind.expected_namespace(),
        kind,
        member_scope_id: None,
        deleted: false,
    }
}

fn declaration(ids: &mut Ids, name: &str, scope_id: ScopeId, kind: DeclarationKind) -> Declaration {
    let id = ids.next();
    declaration_with_id(id, ids.next, name, scope_id, kind)
}

impl Fixture {
    #[allow(clippy::too_many_arguments)]
    fn add_tag(
        &mut self,
        name: &str,
        declared_type: PrimitiveType,
        address_intent: AddressIntent,
        allocated_address: Option<Address>,
        start_value: Option<PlcValue>,
        retain_policy: RetainPolicy,
        kind: TagKind,
        hardware_channel_id: Option<ChannelId>,
        creation_ordinal: u64,
        comment: &str,
        display_format: &str,
    ) -> TagId {
        let declaration_kind = if matches!(&kind, TagKind::Constant(_)) {
            DeclarationKind::GlobalConstant
        } else {
            DeclarationKind::GlobalTag
        };
        let declaration = declaration(&mut self.ids, name, self.global_scope, declaration_kind);
        self.symbols
            .add_declaration(declaration.clone())
            .expect("tag declaration");
        let tag_id = self.ids.next();
        self.symbols
            .add_tag(Tag {
                id: tag_id,
                declaration_id: declaration.id,
                controller_id: self.controller_id,
                creation_ordinal,
                table_id: self.table_id,
                name: declaration.name,
                declared_type: CanonicalType::Primitive(declared_type),
                address_intent,
                allocated_address,
                comment: comment.to_owned(),
                start_value,
                retain_policy,
                display_format: display_format.to_owned(),
                kind,
                hardware_channel_id,
            })
            .expect("tag record");
        tag_id
    }

    fn add_variable(
        &mut self,
        name: &str,
        declared_type: PrimitiveType,
        address: Address,
        hardware_channel_id: Option<ChannelId>,
    ) -> TagId {
        self.add_tag(
            name,
            declared_type,
            AddressIntent::explicit(address.to_string()),
            Some(address),
            None,
            RetainPolicy::NonRetentive,
            TagKind::Variable,
            hardware_channel_id,
            1,
            "",
            "",
        )
    }

    fn diagnostic_codes_for(&self, tag_id: TagId) -> BTreeSet<DiagnosticCode> {
        self.symbols
            .validate_tags(&self.profile, &self.project, &self.artifact)
            .into_iter()
            .filter(|diagnostic| diagnostic.primary.id == tag_id.uuid())
            .map(|diagnostic| diagnostic.code)
            .collect()
    }

    fn channel_at(&self, address: Address) -> ChannelId {
        self.artifact
            .channel_bindings
            .values()
            .find(|binding| match (binding.address, address) {
                (
                    ChannelAddress::Bit {
                        area: AddressArea::Input,
                        byte: left_byte,
                        bit: left_bit,
                    },
                    Address::InputBit {
                        byte: right_byte,
                        bit: right_bit,
                    },
                )
                | (
                    ChannelAddress::Bit {
                        area: AddressArea::Output,
                        byte: left_byte,
                        bit: left_bit,
                    },
                    Address::OutputBit {
                        byte: right_byte,
                        bit: right_bit,
                    },
                ) => left_byte == right_byte && left_bit == right_bit,
                (
                    ChannelAddress::Word {
                        area: AddressArea::Input,
                        byte: left_byte,
                    },
                    Address::InputWord { byte: right_byte },
                )
                | (
                    ChannelAddress::Word {
                        area: AddressArea::Output,
                        byte: left_byte,
                    },
                    Address::OutputWord { byte: right_byte },
                ) => left_byte == right_byte,
                _ => false,
            })
            .map(|binding| binding.channel_id)
            .expect("fixture channel at canonical address")
    }
}

fn semantic_probe_fingerprint(comment: &str, display: &str, start: bool, bit: u8) -> Sha256Digest {
    let mut test = fixture();
    test.add_tag(
        "SemanticProbe",
        PrimitiveType::Bool,
        AddressIntent::explicit(format!("M0.{bit}")),
        Some(Address::MarkerBit { byte: 0, bit }),
        Some(PlcValue::Bool(start)),
        RetainPolicy::NonRetentive,
        TagKind::Variable,
        None,
        1,
        comment,
        display,
    );
    test.symbols.semantic_fingerprint()
}

#[test]
fn identifier_grammar_case_and_reserved_keyword_corpus() {
    let maximum = format!("A{}", "z".repeat(127));
    let maximum_folded = maximum.to_ascii_lowercase();
    let valid = [
        ("A", "a"),
        ("_", "_"),
        ("Motor_Start_21", "motor_start_21"),
        ("mIxEdCaSe", "mixedcase"),
        (maximum.as_str(), maximum_folded.as_str()),
    ];
    for (authored, folded) in valid {
        let parsed = Identifier::parse(authored).expect("positive identifier corpus");
        assert_eq!(parsed.as_str(), authored, "authored case: {authored}");
        assert_eq!(parsed.folded(), folded, "ASCII fold: {authored}");
    }

    let too_long = format!("A{}", "z".repeat(128));
    let invalid = [
        ("", IdentifierError::Empty),
        (too_long.as_str(), IdentifierError::TooLong),
        ("0Motor", IdentifierError::InvalidGrammar),
        ("Motor-Start", IdentifierError::InvalidGrammar),
        ("Motor Start", IdentifierError::InvalidGrammar),
        ("Motor.Start", IdentifierError::InvalidGrammar),
        ("\"Motor\"", IdentifierError::InvalidGrammar),
        ("[Motor]", IdentifierError::InvalidGrammar),
        ("Motor\\Start", IdentifierError::InvalidGrammar),
        ("Mötor", IdentifierError::NonAsciiUnsupported),
        ("Δ", IdentifierError::NonAsciiUnsupported),
    ];
    for (authored, expected) in invalid {
        assert_eq!(Identifier::parse(authored), Err(expected), "{authored:?}");
    }

    // Includes control-flow/type tokens and every declaration keyword accepted
    // by the SCL lexer. Comparison is deliberately ASCII-case-insensitive.
    for keyword in [
        "IF",
        "then",
        "BOOL",
        "end_function_block",
        "VAR_EXTERNAL",
        "var_stat",
        "VAR_CONFIG",
        "var_access",
        "DATA_BLOCK",
        "end_data_block",
        "PROGRAM",
        "end_program",
        "CONFIGURATION",
        "end_configuration",
        "RESOURCE",
        "end_resource",
    ] {
        assert_eq!(
            Identifier::parse(keyword),
            Err(IdentifierError::ReservedKeyword),
            "reserved keyword {keyword}"
        );
    }
}

#[test]
fn namespace_and_scope_shadowing_boundary_matrix() {
    let namespace_matrix = [
        (DeclarationKind::GlobalTag, Namespace::Value),
        (DeclarationKind::GlobalConstant, Namespace::Value),
        (DeclarationKind::GlobalDb, Namespace::Value),
        (DeclarationKind::InstanceDb, Namespace::Value),
        (
            DeclarationKind::BlockValue(BlockValueRole::Input),
            Namespace::Value,
        ),
        (
            DeclarationKind::BlockValue(BlockValueRole::Output),
            Namespace::Value,
        ),
        (
            DeclarationKind::BlockValue(BlockValueRole::InOut),
            Namespace::Value,
        ),
        (
            DeclarationKind::BlockValue(BlockValueRole::Static),
            Namespace::Value,
        ),
        (
            DeclarationKind::BlockValue(BlockValueRole::Temp),
            Namespace::Value,
        ),
        (
            DeclarationKind::BlockValue(BlockValueRole::Constant),
            Namespace::Value,
        ),
        (
            DeclarationKind::BlockValue(BlockValueRole::Return),
            Namespace::Value,
        ),
        (DeclarationKind::CallableBlock, Namespace::Callable),
        (DeclarationKind::NamedType, Namespace::Type),
        (DeclarationKind::Member, Namespace::Member),
        (DeclarationKind::Label, Namespace::Label),
        (DeclarationKind::Instruction, Namespace::Instruction),
        (DeclarationKind::HardwareChannel, Namespace::HardwareChannel),
    ];
    for (kind, namespace) in namespace_matrix {
        assert_eq!(kind.expected_namespace(), namespace, "{kind:?}");
    }

    let binding_kinds = BTreeSet::from([
        BindingKind::Declaration,
        BindingKind::Read,
        BindingKind::Write,
        BindingKind::ReadWrite,
        BindingKind::Call,
        BindingKind::Instantiate,
        BindingKind::TypeUse,
        BindingKind::AddressBind,
        BindingKind::HardwareBind,
        BindingKind::HmiBindReserved,
    ]);
    assert_eq!(binding_kinds.len(), 10);

    let profile = TrainingProfile::edu21();
    let mut ids = Ids::default();
    let controller_a = ids.next::<ControllerId>();
    let controller_b = ids.next::<ControllerId>();
    let global_a = ids.next::<ScopeId>();
    let global_b = ids.next::<ScopeId>();
    let block_a1 = ids.next::<ScopeId>();
    let block_a2 = ids.next::<ScopeId>();
    let block_b = ids.next::<ScopeId>();
    let mut universe = SymbolUniverse::new(profile.pin());
    for scope in [
        Scope {
            id: global_a,
            creation_ordinal: 1,
            kind: ScopeKind::ControllerGlobal(controller_a),
            parent_scope_id: None,
        },
        Scope {
            id: global_b,
            creation_ordinal: 2,
            kind: ScopeKind::ControllerGlobal(controller_b),
            parent_scope_id: None,
        },
        Scope {
            id: block_a1,
            creation_ordinal: 3,
            kind: ScopeKind::Block {
                controller_id: controller_a,
                block_id: ids.next(),
            },
            parent_scope_id: Some(global_a),
        },
        Scope {
            id: block_a2,
            creation_ordinal: 4,
            kind: ScopeKind::Block {
                controller_id: controller_a,
                block_id: ids.next(),
            },
            parent_scope_id: Some(global_a),
        },
        Scope {
            id: block_b,
            creation_ordinal: 5,
            kind: ScopeKind::Block {
                controller_id: controller_b,
                block_id: ids.next(),
            },
            parent_scope_id: Some(global_b),
        },
    ] {
        universe.add_scope(scope).expect("scope matrix");
    }

    let shared_value = declaration(&mut ids, "SharedName", global_a, DeclarationKind::GlobalTag);
    universe
        .add_declaration(shared_value.clone())
        .expect("global value");
    assert_eq!(
        universe.add_declaration(declaration(
            &mut ids,
            "sharedname",
            global_a,
            DeclarationKind::GlobalConstant,
        )),
        Err(SymbolError::DuplicateName),
        "same scope and namespace is case-folded"
    );
    assert_eq!(
        universe.add_declaration(declaration(
            &mut ids,
            "SHAREDNAME",
            block_a1,
            DeclarationKind::BlockValue(BlockValueRole::Temp),
        )),
        Err(SymbolError::ShadowingProhibited),
        "block cannot hide controller-global value"
    );

    let local_a1 = declaration(
        &mut ids,
        "ReusableLocal",
        block_a1,
        DeclarationKind::BlockValue(BlockValueRole::Input),
    );
    let local_a2 = declaration(
        &mut ids,
        "reusablelocal",
        block_a2,
        DeclarationKind::BlockValue(BlockValueRole::Return),
    );
    universe.add_declaration(local_a1).expect("first local");
    universe
        .add_declaration(local_a2)
        .expect("separate blocks may reuse local spelling");
    assert_eq!(
        universe.add_declaration(declaration(
            &mut ids,
            "REUSABLELOCAL",
            global_a,
            DeclarationKind::GlobalTag,
        )),
        Err(SymbolError::ShadowingProhibited),
        "global creation cannot retroactively shadow block locals"
    );

    universe
        .add_declaration(declaration(
            &mut ids,
            "sharedname",
            global_b,
            DeclarationKind::GlobalTag,
        ))
        .expect("other controllers have independent globals");
    let shared_type = declaration(&mut ids, "SHAREDNAME", global_a, DeclarationKind::NamedType);
    universe
        .add_declaration(shared_type.clone())
        .expect("different namespaces may share spelling");

    let mut mismatched = declaration(&mut ids, "Mismatch", global_a, DeclarationKind::GlobalTag);
    mismatched.namespace = Namespace::Type;
    assert_eq!(
        universe.add_declaration(mismatched),
        Err(SymbolError::NamespaceKindMismatch)
    );

    let value_reference = ids.next::<ReferenceId>();
    let value_resolution = universe
        .create_reference(
            value_reference,
            1,
            &["sharedname"],
            Namespace::Value,
            block_a1,
            SourceIdentity {
                object_id: ids.next(),
                location: "block-a1/value".to_owned(),
            },
            BindingKind::Read,
        )
        .expect("value resolution");
    let Resolution::Resolved(value_binding) = value_resolution else {
        panic!("value namespace should resolve");
    };
    assert_eq!(value_binding.target_id, shared_value.id);

    let type_resolution = universe
        .create_reference(
            ids.next(),
            2,
            &["sharedname"],
            Namespace::Type,
            block_a1,
            SourceIdentity {
                object_id: ids.next(),
                location: "block-a1/type".to_owned(),
            },
            BindingKind::TypeUse,
        )
        .expect("type resolution");
    let Resolution::Resolved(type_binding) = type_resolution else {
        panic!("type namespace should resolve");
    };
    assert_eq!(type_binding.target_id, shared_type.id);

    assert_eq!(
        universe.create_reference(
            ids.next(),
            3,
            &["sharedname"],
            Namespace::Value,
            block_b,
            SourceIdentity {
                object_id: ids.next(),
                location: "block-b/value".to_owned(),
            },
            BindingKind::HmiBindReserved,
        ),
        Err(SymbolError::ReservedHmiBinding)
    );
}

#[test]
fn qualified_resolution_and_rename_delete_identity_matrix() {
    let profile = TrainingProfile::edu21();
    let mut ids = Ids::default();
    let controller_id = ids.next::<ControllerId>();
    let global_scope = ids.next::<ScopeId>();
    let block_scope = ids.next::<ScopeId>();
    let db_id = ids.next::<DeclarationId>();
    let db_member_scope = ids.next::<ScopeId>();
    let aggregate_id = ids.next::<DeclarationId>();
    let aggregate_member_scope = ids.next::<ScopeId>();
    let leaf_id = ids.next::<DeclarationId>();
    let mut universe = SymbolUniverse::new(profile.pin());
    for scope in [
        Scope {
            id: global_scope,
            creation_ordinal: 1,
            kind: ScopeKind::ControllerGlobal(controller_id),
            parent_scope_id: None,
        },
        Scope {
            id: block_scope,
            creation_ordinal: 2,
            kind: ScopeKind::Block {
                controller_id,
                block_id: ids.next(),
            },
            parent_scope_id: Some(global_scope),
        },
        Scope {
            id: db_member_scope,
            creation_ordinal: 3,
            kind: ScopeKind::AggregateMember(db_id),
            parent_scope_id: None,
        },
        Scope {
            id: aggregate_member_scope,
            creation_ordinal: 4,
            kind: ScopeKind::AggregateMember(aggregate_id),
            parent_scope_id: None,
        },
    ] {
        universe.add_scope(scope).expect("qualified scope");
    }

    let mut db = declaration_with_id(
        db_id,
        1,
        "ProcessDb",
        global_scope,
        DeclarationKind::GlobalDb,
    );
    db.member_scope_id = Some(db_member_scope);
    universe.add_declaration(db).expect("DB declaration");
    let mut aggregate = declaration_with_id(
        aggregate_id,
        2,
        "Parameters",
        db_member_scope,
        DeclarationKind::Member,
    );
    aggregate.member_scope_id = Some(aggregate_member_scope);
    universe
        .add_declaration(aggregate)
        .expect("aggregate member");
    universe
        .add_declaration(declaration_with_id(
            leaf_id,
            3,
            "LimitValue",
            aggregate_member_scope,
            DeclarationKind::Member,
        ))
        .expect("leaf member");
    universe
        .add_declaration(declaration(
            &mut ids,
            "NoSuchMember",
            global_scope,
            DeclarationKind::GlobalTag,
        ))
        .expect("global fallback sentinel");

    let reference_id = ids.next::<ReferenceId>();
    let source = SourceIdentity {
        object_id: ids.next::<SourceObjectId>(),
        location: "network/7/box/3/input/1".to_owned(),
    };
    let resolution = universe
        .create_reference(
            reference_id,
            1,
            &["processdb", "PARAMETERS", "limitvalue"],
            Namespace::Member,
            block_scope,
            source.clone(),
            BindingKind::ReadWrite,
        )
        .expect("qualified binding");
    let Resolution::Resolved(binding) = resolution else {
        panic!("three-segment path should resolve");
    };
    assert_eq!(binding.target_path, vec![db_id, aggregate_id, leaf_id]);
    assert_eq!(binding.target_id, leaf_id);
    assert_eq!(binding.target_kind, DeclarationKind::Member);
    assert_eq!(binding.owning_scope_id, block_scope);
    assert_eq!(binding.source, source);
    assert_eq!(binding.binding_kind, BindingKind::ReadWrite);
    for target in [db_id, aggregate_id, leaf_id] {
        assert_eq!(
            universe.cross_reference_index().references_to(target),
            vec![reference_id]
        );
    }

    let fallback = universe
        .create_reference(
            ids.next(),
            2,
            &["ProcessDb", "NoSuchMember"],
            Namespace::Member,
            block_scope,
            SourceIdentity {
                object_id: ids.next(),
                location: "network/7/box/4".to_owned(),
            },
            BindingKind::Read,
        )
        .expect("unresolved qualified reference record");
    assert_eq!(
        fallback,
        Resolution::Unresolved,
        "member lookup never falls back"
    );

    for (target, new_name, segment) in [
        (db_id, "ProductionDb", 0_usize),
        (aggregate_id, "Settings", 1),
        (leaf_id, "Maximum", 2),
    ] {
        let preview = universe
            .preview_rename(target, new_name)
            .expect("rename preview");
        assert_eq!(preview.declaration_id, target);
        assert_eq!(preview.affected_reference_ids, vec![reference_id]);
        universe.commit_rename(&preview).expect("atomic rename");
        let ReferenceState::Resolved(current) = &universe.references()[&reference_id].state else {
            panic!("rename must retain resolved binding");
        };
        assert_eq!(current.target_path, vec![db_id, aggregate_id, leaf_id]);
        assert_eq!(current.display_path[segment].as_str(), new_name);
    }

    universe
        .delete_declaration(leaf_id)
        .expect("delete bound leaf");
    let ReferenceState::StaleDeleted(stale) = &universe.references()[&reference_id].state else {
        panic!("delete must create a UUID-preserving tombstone");
    };
    assert_eq!(stale.target_id, leaf_id);
    assert_eq!(stale.target_path, vec![db_id, aggregate_id, leaf_id]);
    assert_eq!(stale.binding_kind, BindingKind::ReadWrite);
    assert_eq!(
        universe.cross_reference_index().references_to(leaf_id),
        vec![reference_id]
    );

    let replacement = declaration(
        &mut ids,
        "Maximum",
        aggregate_member_scope,
        DeclarationKind::Member,
    );
    universe
        .add_declaration(replacement.clone())
        .expect("same-named replacement");
    assert!(matches!(
        universe.references()[&reference_id].state,
        ReferenceState::StaleDeleted(_)
    ));
    let rebound = universe
        .rebind_reference(reference_id)
        .expect("explicit rebind command");
    let Resolution::Resolved(rebound) = rebound else {
        panic!("explicit rebind should resolve replacement");
    };
    assert_eq!(rebound.target_id, replacement.id);
    assert!(
        universe
            .cross_reference_index()
            .references_to(leaf_id)
            .is_empty()
    );
    assert_eq!(
        universe
            .cross_reference_index()
            .references_to(replacement.id),
        vec![reference_id]
    );

    let never_id = ids.next::<ReferenceId>();
    assert_eq!(
        universe
            .create_reference(
                never_id,
                3,
                &["FutureValue"],
                Namespace::Value,
                block_scope,
                SourceIdentity {
                    object_id: ids.next(),
                    location: "network/8/contact/1".to_owned(),
                },
                BindingKind::Read,
            )
            .expect("never-resolved reference"),
        Resolution::Unresolved
    );
    let future = declaration(
        &mut ids,
        "FutureValue",
        global_scope,
        DeclarationKind::GlobalTag,
    );
    universe
        .add_declaration(future.clone())
        .expect("later declaration");
    assert!(matches!(
        universe.references()[&never_id].state,
        ReferenceState::NeverResolved { .. }
    ));
    let Resolution::Resolved(explicit) = universe
        .rebind_reference(never_id)
        .expect("explicit bind of later declaration")
    else {
        panic!("explicit rebind should resolve");
    };
    assert_eq!(explicit.target_id, future.id);

    let stale_preview = universe
        .preview_rename(future.id, "FutureRenamed")
        .expect("preview before unrelated mutation");
    universe
        .add_declaration(declaration(
            &mut ids,
            "Unrelated",
            global_scope,
            DeclarationKind::GlobalTag,
        ))
        .expect("unrelated mutation");
    assert_eq!(
        universe.commit_rename(&stale_preview),
        Err(SymbolError::StalePreview)
    );
    assert_eq!(
        universe.declarations()[&future.id].name.as_str(),
        "FutureValue"
    );
}

#[test]
fn canonical_address_parser_positive_and_negative_corpus() {
    let positive = [
        ("I0.0", "I0.0", SymbolAddressArea::Input, 1, 1),
        ("%i12.7", "I12.7", SymbolAddressArea::Input, 1, 1),
        ("Q3.4", "Q3.4", SymbolAddressArea::Output, 1, 1),
        ("%q0.0", "Q0.0", SymbolAddressArea::Output, 1, 1),
        ("IW0", "IW0", SymbolAddressArea::Input, 16, 2),
        ("%iw2", "IW2", SymbolAddressArea::Input, 16, 2),
        ("QW42", "QW42", SymbolAddressArea::Output, 16, 2),
        ("M0.0", "M0.0", SymbolAddressArea::Marker, 1, 1),
        ("%m8.7", "M8.7", SymbolAddressArea::Marker, 1, 1),
        ("MB9", "MB9", SymbolAddressArea::Marker, 8, 1),
        ("mw10", "MW10", SymbolAddressArea::Marker, 16, 2),
        ("MD12", "MD12", SymbolAddressArea::Marker, 32, 4),
        ("%ml16", "ML16", SymbolAddressArea::Marker, 64, 8),
    ];
    for (authored, canonical, area, width, alignment) in positive {
        let address = Address::parse(authored).expect("positive address corpus");
        assert_eq!(address.to_string(), canonical, "{authored}");
        assert_eq!(address.area(), area, "{authored}");
        assert_eq!(address.width_bits(), width, "{authored}");
        assert_eq!(address.alignment_bytes(), alignment, "{authored}");
    }

    let negative = [
        ("", AddressError::Malformed),
        ("%", AddressError::Malformed),
        (" I0.0", AddressError::Malformed),
        ("I0.0 ", AddressError::Malformed),
        ("I0", AddressError::Malformed),
        ("I0.8", AddressError::BitOutOfRange),
        ("I0.10", AddressError::Malformed),
        ("I00.0", AddressError::Malformed),
        ("I4294967296.0", AddressError::ByteOverflow),
        ("IB0", AddressError::Malformed),
        ("ID0", AddressError::Malformed),
        ("IL0", AddressError::Malformed),
        ("QB0", AddressError::Malformed),
        ("QD0", AddressError::Malformed),
        ("QL0", AddressError::Malformed),
        ("IW0.0", AddressError::Malformed),
        ("QW0.0", AddressError::Malformed),
        ("MW0.0", AddressError::Malformed),
        ("MD0.0", AddressError::Malformed),
        ("ML0.0", AddressError::Malformed),
        ("M-1.0", AddressError::Malformed),
        ("M1.2.3", AddressError::Malformed),
        ("DB1.DBX0.0", AddressError::Malformed),
        ("%%I0.0", AddressError::Malformed),
        ("Iø.0", AddressError::Malformed),
        ("http://I0.0", AddressError::Malformed),
        ("I0.0:80", AddressError::Malformed),
    ];
    for (authored, expected) in negative {
        assert_eq!(Address::parse(authored), Err(expected), "{authored}");
    }
}

#[test]
fn exact_io_channel_and_marker_width_type_matrix() {
    let positive = [
        (
            Address::InputBit { byte: 0, bit: 0 },
            PrimitiveType::Bool,
            true,
        ),
        (
            Address::OutputBit { byte: 0, bit: 0 },
            PrimitiveType::Bool,
            true,
        ),
        (Address::InputWord { byte: 2 }, PrimitiveType::Int, true),
        (Address::OutputWord { byte: 2 }, PrimitiveType::Int, true),
        (
            Address::MarkerBit { byte: 0, bit: 0 },
            PrimitiveType::Bool,
            false,
        ),
        (Address::MarkerByte { byte: 0 }, PrimitiveType::Sint, false),
        (Address::MarkerByte { byte: 0 }, PrimitiveType::Usint, false),
        (Address::MarkerByte { byte: 0 }, PrimitiveType::Byte, false),
        (Address::MarkerByte { byte: 0 }, PrimitiveType::Char, false),
        (Address::MarkerWord { byte: 0 }, PrimitiveType::Int, false),
        (Address::MarkerWord { byte: 0 }, PrimitiveType::Uint, false),
        (Address::MarkerWord { byte: 0 }, PrimitiveType::Word, false),
        (Address::MarkerDword { byte: 0 }, PrimitiveType::Dint, false),
        (
            Address::MarkerDword { byte: 0 },
            PrimitiveType::Udint,
            false,
        ),
        (
            Address::MarkerDword { byte: 0 },
            PrimitiveType::Dword,
            false,
        ),
        (Address::MarkerDword { byte: 0 }, PrimitiveType::Real, false),
        (Address::MarkerLword { byte: 0 }, PrimitiveType::Lint, false),
        (
            Address::MarkerLword { byte: 0 },
            PrimitiveType::Ulint,
            false,
        ),
        (
            Address::MarkerLword { byte: 0 },
            PrimitiveType::Lword,
            false,
        ),
        (
            Address::MarkerLword { byte: 0 },
            PrimitiveType::Lreal,
            false,
        ),
        (Address::MarkerLword { byte: 0 }, PrimitiveType::Time, false),
    ];
    for (index, (address, primitive, needs_channel)) in positive.into_iter().enumerate() {
        let mut test = fixture();
        let channel = needs_channel.then(|| test.channel_at(address));
        let tag_id = test.add_variable(&format!("Positive{index}"), primitive, address, channel);
        assert!(
            test.diagnostic_codes_for(tag_id).is_empty(),
            "positive binding {address} / {primitive:?}"
        );
    }

    let negative = [
        (
            "IoWithoutChannel",
            Address::InputBit { byte: 0, bit: 0 },
            PrimitiveType::Bool,
            None,
            DiagnosticCode::UnmappedIoAddress,
        ),
        (
            "WrongIoChannel",
            Address::InputBit { byte: 0, bit: 1 },
            PrimitiveType::Bool,
            Some(Address::InputBit { byte: 0, bit: 0 }),
            DiagnosticCode::UnmappedIoAddress,
        ),
        (
            "UnsignedIoWord",
            Address::InputWord { byte: 2 },
            PrimitiveType::Uint,
            Some(Address::InputWord { byte: 2 }),
            DiagnosticCode::TypeMismatch,
        ),
        (
            "BoolAtMarkerByte",
            Address::MarkerByte { byte: 0 },
            PrimitiveType::Bool,
            None,
            DiagnosticCode::TypeMismatch,
        ),
        (
            "ByteAtMarkerBit",
            Address::MarkerBit { byte: 0, bit: 0 },
            PrimitiveType::Byte,
            None,
            DiagnosticCode::TypeMismatch,
        ),
        (
            "DintAtMarkerWord",
            Address::MarkerWord { byte: 0 },
            PrimitiveType::Dint,
            None,
            DiagnosticCode::TypeMismatch,
        ),
        (
            "MisalignedInputWord",
            Address::InputWord { byte: 1 },
            PrimitiveType::Int,
            Some(Address::InputWord { byte: 1 }),
            DiagnosticCode::PlcAddressAlignment,
        ),
        (
            "MisalignedMarkerWord",
            Address::MarkerWord { byte: 1 },
            PrimitiveType::Int,
            None,
            DiagnosticCode::PlcAddressAlignment,
        ),
        (
            "MisalignedMarkerDword",
            Address::MarkerDword { byte: 2 },
            PrimitiveType::Dint,
            None,
            DiagnosticCode::PlcAddressAlignment,
        ),
        (
            "MisalignedMarkerLword",
            Address::MarkerLword { byte: 4 },
            PrimitiveType::Lint,
            None,
            DiagnosticCode::PlcAddressAlignment,
        ),
        (
            "MarkerCapacityOverflow",
            Address::MarkerLword { byte: 4096 },
            PrimitiveType::Lint,
            None,
            DiagnosticCode::PlcAddressCapacity,
        ),
        (
            "MarkerWithHardwareChannel",
            Address::MarkerBit { byte: 8, bit: 0 },
            PrimitiveType::Bool,
            Some(Address::InputBit { byte: 0, bit: 0 }),
            DiagnosticCode::TypeMismatch,
        ),
    ];
    for (name, address, primitive, channel_address, expected) in negative {
        let mut test = fixture();
        let channel = channel_address.map(|candidate| test.channel_at(candidate));
        let tag_id = test.add_variable(name, primitive, address, channel);
        assert!(
            test.diagnostic_codes_for(tag_id).contains(&expected),
            "negative binding {name}: expected {expected:?}"
        );
    }

    let malformed = [
        ("MalformedWideIo", "ID0"),
        ("MalformedHostLike", "http://127.0.0.1:80"),
    ];
    for (name, authored) in malformed {
        let mut test = fixture();
        let tag_id = test.add_tag(
            name,
            PrimitiveType::Bool,
            AddressIntent::explicit(authored),
            None,
            None,
            RetainPolicy::NonRetentive,
            TagKind::Variable,
            None,
            1,
            "",
            "",
        );
        let diagnostics = test
            .symbols
            .validate_tags(&test.profile, &test.project, &test.artifact);
        let diagnostic = diagnostics
            .iter()
            .find(|diagnostic| diagnostic.primary.id == tag_id.uuid())
            .expect("malformed address diagnostic");
        assert_eq!(diagnostic.code, DiagnosticCode::MalformedPlcAddress);
        assert_eq!(
            diagnostic
                .parameters
                .get("authoredText")
                .map(String::as_str),
            Some(authored)
        );
        assert_eq!(test.symbols.tags()[&tag_id].allocated_address, None);
        let AddressIntent::Explicit {
            authored_text,
            parsed,
        } = &test.symbols.tags()[&tag_id].address_intent
        else {
            panic!("explicit invalid text should be retained");
        };
        assert_eq!(authored_text, authored);
        assert_eq!(*parsed, None);
    }
}

#[test]
fn constant_storage_and_semantic_fingerprint_matrix() {
    enum ConstantCase {
        Valid,
        Overflow,
        WrongValueKind,
        RuntimeStart,
        Retentive,
        Addressed,
        HardwareBound,
    }
    let cases = [
        (ConstantCase::Valid, None),
        (
            ConstantCase::Overflow,
            Some(DiagnosticCode::ConstantRangeOrArithmetic),
        ),
        (
            ConstantCase::WrongValueKind,
            Some(DiagnosticCode::ConstantRangeOrArithmetic),
        ),
        (
            ConstantCase::RuntimeStart,
            Some(DiagnosticCode::TypeMismatch),
        ),
        (ConstantCase::Retentive, Some(DiagnosticCode::TypeMismatch)),
        (ConstantCase::Addressed, Some(DiagnosticCode::TypeMismatch)),
        (
            ConstantCase::HardwareBound,
            Some(DiagnosticCode::TypeMismatch),
        ),
    ];
    for (index, (case, expected)) in cases.into_iter().enumerate() {
        let mut test = fixture();
        let (
            declared_type,
            value,
            address_intent,
            allocated_address,
            start_value,
            retain_policy,
            channel,
        ) = match case {
            ConstantCase::Valid => (
                PrimitiveType::Bool,
                PlcValue::Bool(true),
                AddressIntent::None,
                None,
                None,
                RetainPolicy::NonRetentive,
                None,
            ),
            ConstantCase::Overflow => (
                PrimitiveType::Sint,
                PlcValue::Signed(128),
                AddressIntent::None,
                None,
                None,
                RetainPolicy::NonRetentive,
                None,
            ),
            ConstantCase::WrongValueKind => (
                PrimitiveType::Bool,
                PlcValue::Signed(1),
                AddressIntent::None,
                None,
                None,
                RetainPolicy::NonRetentive,
                None,
            ),
            ConstantCase::RuntimeStart => (
                PrimitiveType::Bool,
                PlcValue::Bool(true),
                AddressIntent::None,
                None,
                Some(PlcValue::Bool(false)),
                RetainPolicy::NonRetentive,
                None,
            ),
            ConstantCase::Retentive => (
                PrimitiveType::Bool,
                PlcValue::Bool(true),
                AddressIntent::None,
                None,
                None,
                RetainPolicy::Retentive,
                None,
            ),
            ConstantCase::Addressed => (
                PrimitiveType::Bool,
                PlcValue::Bool(true),
                AddressIntent::explicit("M0.0"),
                Some(Address::MarkerBit { byte: 0, bit: 0 }),
                None,
                RetainPolicy::NonRetentive,
                None,
            ),
            ConstantCase::HardwareBound => (
                PrimitiveType::Bool,
                PlcValue::Bool(true),
                AddressIntent::None,
                None,
                None,
                RetainPolicy::NonRetentive,
                Some(test.channels.input_bit_0),
            ),
        };
        let tag_id = test.add_tag(
            &format!("Constant{index}"),
            declared_type,
            address_intent,
            allocated_address,
            start_value,
            retain_policy,
            TagKind::Constant(value),
            channel,
            1,
            "Unicode metadata → 許可",
            "engineering display",
        );
        let codes = test.diagnostic_codes_for(tag_id);
        if let Some(expected) = expected {
            assert!(
                codes.contains(&expected),
                "constant case {index}: {codes:?}"
            );
        } else {
            assert!(codes.is_empty(), "valid constant: {codes:?}");
        }
    }

    let baseline = semantic_probe_fingerprint("ASCII", "binary", false, 0);
    assert_eq!(
        baseline,
        semantic_probe_fingerprint("Unicode αβγ 許可", "localized", false, 0),
        "comment and display format are nonsemantic"
    );
    assert_ne!(
        baseline,
        semantic_probe_fingerprint("ASCII", "binary", true, 0),
        "start value is semantic"
    );
    assert_ne!(
        baseline,
        semantic_probe_fingerprint("ASCII", "binary", false, 1),
        "address is semantic"
    );
}

#[test]
fn automatic_io_channel_priority_and_exhaustion_matrix() {
    #[derive(Clone, Copy, Debug)]
    enum PriorityField {
        ControllerCreationOrdinal,
        LocationRank,
        StationCreationOrdinal,
        SlotNumber,
        ChannelIndex,
        ChannelUuid,
    }

    for field in [
        PriorityField::ControllerCreationOrdinal,
        PriorityField::LocationRank,
        PriorityField::StationCreationOrdinal,
        PriorityField::SlotNumber,
        PriorityField::ChannelIndex,
        PriorityField::ChannelUuid,
    ] {
        let mut test = fixture();
        let left_id = test.channels.input_bit_0;
        let right_id = test.channels.input_bit_1;
        test.artifact
            .channel_bindings
            .retain(|channel_id, _| *channel_id == left_id || *channel_id == right_id);
        for channel_id in [left_id, right_id] {
            let binding = test
                .artifact
                .channel_bindings
                .get_mut(&channel_id)
                .expect("priority candidate");
            binding.controller_creation_ordinal = 1;
            binding.location_rank = 1;
            binding.station_creation_ordinal = 1;
            binding.slot_number = 1;
            binding.channel_index = 1;
        }

        let expected_channel = match field {
            PriorityField::ControllerCreationOrdinal => {
                test.artifact
                    .channel_bindings
                    .get_mut(&left_id)
                    .expect("left")
                    .controller_creation_ordinal = 2;
                right_id
            }
            PriorityField::LocationRank => {
                test.artifact
                    .channel_bindings
                    .get_mut(&left_id)
                    .expect("left")
                    .location_rank = 2;
                right_id
            }
            PriorityField::StationCreationOrdinal => {
                test.artifact
                    .channel_bindings
                    .get_mut(&left_id)
                    .expect("left")
                    .station_creation_ordinal = 2;
                right_id
            }
            PriorityField::SlotNumber => {
                test.artifact
                    .channel_bindings
                    .get_mut(&left_id)
                    .expect("left")
                    .slot_number = 2;
                right_id
            }
            PriorityField::ChannelIndex => {
                test.artifact
                    .channel_bindings
                    .get_mut(&left_id)
                    .expect("left")
                    .channel_index = 2;
                right_id
            }
            PriorityField::ChannelUuid => left_id.min(right_id),
        };
        let auto_tag = test.add_tag(
            "AutomaticInput",
            PrimitiveType::Bool,
            AddressIntent::Auto(SymbolAddressArea::Input),
            None,
            None,
            RetainPolicy::NonRetentive,
            TagKind::Variable,
            None,
            1,
            "",
            "",
        );
        let preview = test
            .symbols
            .preview_auto_allocate_tags(&test.profile, &test.project, &test.artifact)
            .expect("priority preview");
        let change = preview
            .changes
            .iter()
            .find(|change| change.tag_id == auto_tag)
            .expect("automatic input change");
        assert_eq!(
            change.proposed_channel_id,
            Some(expected_channel),
            "priority field {field:?}"
        );
        let expected_address = if expected_channel == left_id {
            Address::InputBit { byte: 0, bit: 0 }
        } else {
            Address::InputBit { byte: 0, bit: 1 }
        };
        assert_eq!(change.proposed_address, expected_address, "{field:?}");
    }

    let mut exhausted = fixture();
    let only_channel = exhausted.channels.input_bit_0;
    exhausted
        .artifact
        .channel_bindings
        .retain(|channel_id, _| *channel_id == only_channel);
    exhausted.add_tag(
        "FirstAutomaticInput",
        PrimitiveType::Bool,
        AddressIntent::Auto(SymbolAddressArea::Input),
        None,
        None,
        RetainPolicy::NonRetentive,
        TagKind::Variable,
        None,
        1,
        "",
        "",
    );
    let second = exhausted.add_tag(
        "SecondAutomaticInput",
        PrimitiveType::Bool,
        AddressIntent::Auto(SymbolAddressArea::Input),
        None,
        None,
        RetainPolicy::NonRetentive,
        TagKind::Variable,
        None,
        2,
        "",
        "",
    );
    let before = exhausted.symbols.semantic_fingerprint();
    assert_eq!(
        exhausted.symbols.preview_auto_allocate_tags(
            &exhausted.profile,
            &exhausted.project,
            &exhausted.artifact,
        ),
        Err(SymbolError::NoAutomaticAddressAvailable(second))
    );
    assert_eq!(
        exhausted.symbols.semantic_fingerprint(),
        before,
        "failed allocation preview is nonmutating"
    );
}

#[test]
fn deterministic_allocation_and_overlap_corpus() {
    let mut allocation = fixture();
    allocation.add_variable(
        "OccupiedByte",
        PrimitiveType::Byte,
        Address::MarkerByte { byte: 0 },
        None,
    );
    let first_input = allocation.add_tag(
        "FirstInput",
        PrimitiveType::Bool,
        AddressIntent::Auto(SymbolAddressArea::Input),
        None,
        None,
        RetainPolicy::NonRetentive,
        TagKind::Variable,
        None,
        1,
        "",
        "",
    );
    let second_input = allocation.add_tag(
        "SecondInput",
        PrimitiveType::Bool,
        AddressIntent::Auto(SymbolAddressArea::Input),
        None,
        None,
        RetainPolicy::NonRetentive,
        TagKind::Variable,
        None,
        2,
        "",
        "",
    );
    let marker_bit = allocation.add_tag(
        "AllocatedBit",
        PrimitiveType::Bool,
        AddressIntent::Auto(SymbolAddressArea::Marker),
        None,
        None,
        RetainPolicy::NonRetentive,
        TagKind::Variable,
        None,
        3,
        "",
        "",
    );
    let marker_word = allocation.add_tag(
        "AllocatedWord",
        PrimitiveType::Word,
        AddressIntent::Auto(SymbolAddressArea::Marker),
        None,
        None,
        RetainPolicy::NonRetentive,
        TagKind::Variable,
        None,
        4,
        "",
        "",
    );
    let preview = allocation
        .symbols
        .preview_auto_allocate_tags(
            &allocation.profile,
            &allocation.project,
            &allocation.artifact,
        )
        .expect("deterministic allocation preview");
    let changes: BTreeMap<_, _> = preview
        .changes
        .iter()
        .map(|change| {
            (
                change.tag_id,
                (change.proposed_address, change.proposed_channel_id),
            )
        })
        .collect();
    assert_eq!(
        changes[&first_input],
        (
            Address::InputBit { byte: 0, bit: 1 },
            Some(allocation.channels.input_bit_1)
        ),
        "first compatible channel follows hardware ordering, not lowest address"
    );
    assert_eq!(
        changes[&second_input],
        (
            Address::InputBit { byte: 0, bit: 0 },
            Some(allocation.channels.input_bit_0)
        )
    );
    assert_eq!(
        changes[&marker_bit],
        (Address::MarkerBit { byte: 1, bit: 0 }, None),
        "MB0 reserves every bit in byte zero"
    );
    assert_eq!(
        changes[&marker_word],
        (Address::MarkerWord { byte: 2 }, None),
        "marker first-fit respects occupied intervals and alignment"
    );
    allocation
        .symbols
        .commit_auto_allocate_tags(
            &allocation.profile,
            &allocation.project,
            &allocation.artifact,
            &preview,
        )
        .expect("allocation commit");
    assert!(
        allocation
            .symbols
            .validate_tags(
                &allocation.profile,
                &allocation.project,
                &allocation.artifact
            )
            .is_empty()
    );

    let marker_overlap = [
        (
            Address::MarkerBit { byte: 0, bit: 0 },
            PrimitiveType::Bool,
            Address::MarkerByte { byte: 0 },
            PrimitiveType::Byte,
            true,
        ),
        (
            Address::MarkerByte { byte: 0 },
            PrimitiveType::Byte,
            Address::MarkerByte { byte: 1 },
            PrimitiveType::Byte,
            false,
        ),
        (
            Address::MarkerWord { byte: 0 },
            PrimitiveType::Int,
            Address::MarkerByte { byte: 1 },
            PrimitiveType::Byte,
            true,
        ),
        (
            Address::MarkerWord { byte: 0 },
            PrimitiveType::Int,
            Address::MarkerWord { byte: 2 },
            PrimitiveType::Int,
            false,
        ),
        (
            Address::MarkerDword { byte: 0 },
            PrimitiveType::Dint,
            Address::MarkerWord { byte: 2 },
            PrimitiveType::Int,
            true,
        ),
        (
            Address::MarkerDword { byte: 0 },
            PrimitiveType::Dint,
            Address::MarkerDword { byte: 4 },
            PrimitiveType::Dint,
            false,
        ),
        (
            Address::MarkerLword { byte: 0 },
            PrimitiveType::Lint,
            Address::MarkerDword { byte: 4 },
            PrimitiveType::Dint,
            true,
        ),
        (
            Address::MarkerLword { byte: 0 },
            PrimitiveType::Lint,
            Address::MarkerLword { byte: 8 },
            PrimitiveType::Lint,
            false,
        ),
    ];
    for (index, (left, left_type, right, right_type, expected_overlap)) in
        marker_overlap.into_iter().enumerate()
    {
        let mut test = fixture();
        test.add_variable(&format!("Left{index}"), left_type, left, None);
        test.add_variable(&format!("Right{index}"), right_type, right, None);
        let has_overlap = test
            .symbols
            .validate_tags(&test.profile, &test.project, &test.artifact)
            .iter()
            .any(|diagnostic| diagnostic.code == DiagnosticCode::SymbolOverlap);
        assert_eq!(has_overlap, expected_overlap, "{left} versus {right}");
    }

    let mut io_overlap = fixture();
    let channel = io_overlap.channels.input_bit_0;
    io_overlap.add_variable(
        "InputAliasA",
        PrimitiveType::Bool,
        Address::InputBit { byte: 0, bit: 0 },
        Some(channel),
    );
    io_overlap.add_variable(
        "InputAliasB",
        PrimitiveType::Bool,
        Address::InputBit { byte: 0, bit: 0 },
        Some(channel),
    );
    assert!(
        io_overlap
            .symbols
            .validate_tags(
                &io_overlap.profile,
                &io_overlap.project,
                &io_overlap.artifact
            )
            .iter()
            .any(|diagnostic| diagnostic.code == DiagnosticCode::SymbolOverlap)
    );
}
