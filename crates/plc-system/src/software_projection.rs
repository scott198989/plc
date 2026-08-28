use std::collections::{BTreeMap, BTreeSet};

use plc_compiler::SclSource;
use plc_core::{
    Lifecycle, ObjectId, PayloadValue, Project, ProjectObject, ProjectObjectKind, Sha256Digest,
    Uuid,
};
use plc_hardware::{
    ArrayBound, CanonicalType, InstructionStateKind, PrimitiveType, StructMember, TypeDeclarationId,
};
use plc_program::{
    BlockId, BlockInterface, CanonicalF32, CanonicalF64, CanonicalValue, ControllerId,
    ControllerProgram, DataBlockKind, DataType, EngineeringNumber, InterfaceMember,
    InterfaceMemberId, InterfaceRole, ObDeclaration, ProgramBlock, ProgramUnitKind, RetainPolicy,
};

use crate::{ProjectDiagnostic, ProjectDiagnosticPhase};

pub const PROGRAM_BLOCK_PAYLOAD_SCHEMA: &str = "edu.program-block/1";
pub const DATA_BLOCK_PAYLOAD_SCHEMA: &str = "edu.data-block/1";
pub const TAG_PAYLOAD_SCHEMA: &str = "edu.tag/1";
pub const WATCH_TABLE_PAYLOAD_SCHEMA: &str = "edu.watch-table/1";
pub const TRACE_CONFIG_PAYLOAD_SCHEMA: &str = "edu.trace-configuration/1";
pub const NAMED_TYPE_PAYLOAD_SCHEMA: &str = "edu.named-type/1";

const MAX_BLOCKS: usize = 4_096;
const MAX_MEMBERS_PER_BLOCK: usize = 4_096;
const MAX_TAGS: usize = 16_384;
const MAX_WATCH_ROWS: usize = 2_048;
const MAX_TRACE_CHANNELS: usize = 64;
const MAX_NAMED_TYPES: usize = 2_048;
const MAX_TYPE_DEPTH: u8 = 32;
const MAX_TYPE_MEMBERS: u32 = 4_096;
const MAX_ARRAY_DIMENSIONS: u8 = 6;
const MAX_ARRAY_ELEMENTS: u64 = 1_000_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum AuthoredLanguage {
    Scl,
    Lad,
    Fbd,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphicalBodyHook {
    pub owner_object_id: ObjectId,
    pub owner_block_id: BlockId,
    pub language: AuthoredLanguage,
    pub payload_schema: String,
    pub semantic_payload: BTreeMap<String, PayloadValue>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum CanonicalAddressArea {
    Input,
    Output,
    Memory,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum CanonicalAddressIntent {
    Auto,
    Explicit { byte_offset: u32, bit_offset: u8 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct CanonicalHardwareAddress {
    pub area: CanonicalAddressArea,
    pub intent: CanonicalAddressIntent,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct CanonicalTagTarget {
    pub block_object_id: ObjectId,
    pub member_id: InterfaceMemberId,
    pub hardware: Option<CanonicalHardwareAddress>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CanonicalTag {
    pub object_id: ObjectId,
    pub stable_identity: u128,
    pub display_name: String,
    pub data_type: DataType,
    pub target: CanonicalTagTarget,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CanonicalNamedType {
    object_id: ObjectId,
    declaration_id: TypeDeclarationId,
    display_name: String,
    canonical_type: CanonicalType,
    fingerprint: Sha256Digest,
}

impl CanonicalNamedType {
    #[must_use]
    pub const fn object_id(&self) -> ObjectId {
        self.object_id
    }

    #[must_use]
    pub const fn declaration_id(&self) -> TypeDeclarationId {
        self.declaration_id
    }

    #[must_use]
    pub fn display_name(&self) -> &str {
        &self.display_name
    }

    #[must_use]
    pub const fn canonical_type(&self) -> &CanonicalType {
        &self.canonical_type
    }

    #[must_use]
    pub const fn fingerprint(&self) -> Sha256Digest {
        self.fingerprint
    }

    #[must_use]
    pub fn canonical_reference(&self) -> String {
        format!("TYPE:{}", self.declaration_id)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum CanonicalProbeLayer {
    Natural,
    Effective,
    RawInput,
    CommittedOutput,
    DeliveredOutput,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum CanonicalDisplayBase {
    Automatic,
    Binary,
    Decimal,
    Hexadecimal,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CanonicalWatchRow {
    pub id: u128,
    pub target_tag: ObjectId,
    pub layer: CanonicalProbeLayer,
    pub display_base: CanonicalDisplayBase,
    pub order: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CanonicalWatchTable {
    pub object_id: ObjectId,
    pub name: String,
    pub rows: Vec<CanonicalWatchRow>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CanonicalTraceChannel {
    pub id: u128,
    pub alias: String,
    pub target_tag: ObjectId,
    pub layer: CanonicalProbeLayer,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CanonicalTraceConfig {
    pub object_id: ObjectId,
    pub name: String,
    pub channels: Vec<CanonicalTraceChannel>,
    pub every_scans: u32,
    pub pre_trigger_samples: usize,
    pub post_trigger_samples: usize,
    pub maximum_duration_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CanonicalSoftwareProjection {
    source_document_hash: Sha256Digest,
    source_semantic_fingerprint: Sha256Digest,
    controller_object_id: ObjectId,
    program: ControllerProgram,
    scl_sources: BTreeMap<BlockId, SclSource>,
    block_origins: BTreeMap<BlockId, ObjectId>,
    graphical_bodies: Vec<GraphicalBodyHook>,
    named_types: BTreeMap<TypeDeclarationId, CanonicalNamedType>,
    tags: Vec<CanonicalTag>,
    watch_tables: Vec<CanonicalWatchTable>,
    trace_configs: Vec<CanonicalTraceConfig>,
    diagnostics: Vec<ProjectDiagnostic>,
}

impl CanonicalSoftwareProjection {
    #[must_use]
    pub const fn source_document_hash(&self) -> Sha256Digest {
        self.source_document_hash
    }

    #[must_use]
    pub const fn source_semantic_fingerprint(&self) -> Sha256Digest {
        self.source_semantic_fingerprint
    }

    #[must_use]
    pub const fn controller_object_id(&self) -> ObjectId {
        self.controller_object_id
    }

    #[must_use]
    pub const fn program(&self) -> &ControllerProgram {
        &self.program
    }

    #[must_use]
    pub const fn scl_sources(&self) -> &BTreeMap<BlockId, SclSource> {
        &self.scl_sources
    }

    #[must_use]
    pub fn block_origin(&self, block: BlockId) -> Option<ObjectId> {
        self.block_origins.get(&block).copied()
    }

    #[must_use]
    pub fn graphical_bodies(&self) -> &[GraphicalBodyHook] {
        &self.graphical_bodies
    }

    #[must_use]
    pub const fn named_types(&self) -> &BTreeMap<TypeDeclarationId, CanonicalNamedType> {
        &self.named_types
    }

    #[must_use]
    pub fn named_type(&self, id: TypeDeclarationId) -> Option<&CanonicalNamedType> {
        self.named_types.get(&id)
    }

    #[must_use]
    pub fn named_type_for_object(&self, object_id: ObjectId) -> Option<&CanonicalNamedType> {
        self.named_type(TypeDeclarationId::new(object_id.0))
    }

    #[must_use]
    pub fn tags(&self) -> &[CanonicalTag] {
        &self.tags
    }

    #[must_use]
    pub fn watch_tables(&self) -> &[CanonicalWatchTable] {
        &self.watch_tables
    }

    #[must_use]
    pub fn trace_configs(&self) -> &[CanonicalTraceConfig] {
        &self.trace_configs
    }

    #[must_use]
    pub fn diagnostics(&self) -> &[ProjectDiagnostic] {
        &self.diagnostics
    }

    #[must_use]
    pub fn can_compile(&self) -> bool {
        !self
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.blocking)
    }
}

#[must_use]
#[allow(clippy::too_many_lines)]
pub fn project_software(
    project: &Project,
    controller_object_id: ObjectId,
) -> CanonicalSoftwareProjection {
    let mut diagnostics = Vec::new();
    let controller = project.object(controller_object_id);
    if controller.is_none_or(|object| {
        object.lifecycle != Lifecycle::Active || object.kind != ProjectObjectKind::Controller
    }) {
        diagnostics.push(diagnostic(
            "EDU-SYS-2000",
            "The selected canonical controller is absent, tombstoned, or has the wrong kind.",
            controller_object_id,
        ));
    }
    let mut program = ControllerProgram::new(ControllerId::new(object_u128(controller_object_id)));
    let mut scl_sources = BTreeMap::new();
    let mut block_origins = BTreeMap::new();
    let mut graphical_bodies = Vec::new();

    let authoring = objects_for_controller(project, controller_object_id);
    let (named_types, type_diagnostics) = project_named_types(project, &authoring);
    diagnostics.extend(type_diagnostics);

    let mut blocks = authoring
        .iter()
        .copied()
        .filter(|object| {
            matches!(
                object.kind,
                ProjectObjectKind::ProgramBlock | ProjectObjectKind::DataBlock
            )
        })
        .collect::<Vec<_>>();
    blocks.sort_by_key(|object| (object.creation_ordinal, object.id));
    if blocks.len() > MAX_BLOCKS {
        diagnostics.push(diagnostic(
            "EDU-SYS-2001",
            format!(
                "The selected controller has {} software objects, exceeding the canonical bound of {MAX_BLOCKS}.",
                blocks.len()
            ),
            controller_object_id,
        ));
    }
    for object in blocks.into_iter().take(MAX_BLOCKS) {
        match project_block(object, &named_types) {
            Ok((block, body)) => {
                let block_id = block.id;
                block_origins.insert(block_id, object.id);
                if let Err(error) = program.insert_block(block) {
                    diagnostics.push(diagnostic(
                        "EDU-SYS-2002",
                        format!("The canonical block aggregate rejected this object: {error:?}."),
                        object.id,
                    ));
                    continue;
                }
                match body {
                    Some(ProjectedBody::Scl(source)) => {
                        scl_sources.insert(block_id, SclSource::new(block_id, source));
                    }
                    Some(ProjectedBody::Graphical(language)) => {
                        graphical_bodies.push(GraphicalBodyHook {
                            owner_object_id: object.id,
                            owner_block_id: block_id,
                            language,
                            payload_schema: object.payload_schema.clone(),
                            semantic_payload: object.payload.semantic.clone(),
                        });
                    }
                    None => {}
                }
            }
            Err(message) => diagnostics.push(diagnostic("EDU-SYS-2003", message, object.id)),
        }
    }

    let mut tags = Vec::new();
    let mut watch_tables = Vec::new();
    let mut trace_configs = Vec::new();
    for object in &authoring {
        let parsed = match (object.kind, object.payload_schema.as_str()) {
            (ProjectObjectKind::Tag, TAG_PAYLOAD_SCHEMA) => {
                parse_tag(object, &named_types).map(|value| {
                    if tags.len() < MAX_TAGS {
                        tags.push(value);
                    }
                })
            }
            (ProjectObjectKind::Tag, _) => Err(format!(
                "Tag payload schema '{}' is unsupported; expected {TAG_PAYLOAD_SCHEMA}.",
                object.payload_schema
            )),
            (ProjectObjectKind::Generic, WATCH_TABLE_PAYLOAD_SCHEMA) => {
                parse_watch_table(object).map(|value| watch_tables.push(value))
            }
            (ProjectObjectKind::Generic, TRACE_CONFIG_PAYLOAD_SCHEMA) => {
                parse_trace_config(object).map(|value| trace_configs.push(value))
            }
            _ => continue,
        };
        if let Err(message) = parsed {
            diagnostics.push(diagnostic("EDU-SYS-2010", message, object.id));
        }
    }
    if tags.len() == MAX_TAGS
        && authoring
            .iter()
            .filter(|object| object.kind == ProjectObjectKind::Tag)
            .count()
            > MAX_TAGS
    {
        diagnostics.push(diagnostic(
            "EDU-SYS-2011",
            format!("The canonical tag bound of {MAX_TAGS} was exceeded."),
            controller_object_id,
        ));
    }
    tags.sort_by_key(|tag| tag.object_id);
    watch_tables.sort_by_key(|table| table.object_id);
    trace_configs.sort_by_key(|config| config.object_id);
    diagnostics.sort();
    diagnostics.dedup();

    CanonicalSoftwareProjection {
        source_document_hash: project.document_hash(),
        source_semantic_fingerprint: project.semantic_fingerprint(),
        controller_object_id,
        program,
        scl_sources,
        block_origins,
        graphical_bodies,
        named_types,
        tags,
        watch_tables,
        trace_configs,
        diagnostics,
    }
}

#[derive(Clone, Debug)]
struct RawNamedType {
    object_id: ObjectId,
    declaration_id: TypeDeclarationId,
    display_name: String,
    members: Vec<RawStructMember>,
}

#[derive(Clone, Debug)]
struct RawStructMember {
    id: Uuid,
    name: String,
    declared_order: u32,
    ty: RawTypeExpression,
    comment: String,
}

#[derive(Clone, Debug)]
enum RawTypeExpression {
    Primitive(PrimitiveType),
    InstructionState(InstructionStateKind),
    Named(TypeDeclarationId),
    Array {
        dimensions: Vec<ArrayBound>,
        element_type: Box<Self>,
    },
    AnonymousStruct(Vec<RawStructMember>),
}

fn project_named_types(
    project: &Project,
    objects: &[&ProjectObject],
) -> (
    BTreeMap<TypeDeclarationId, CanonicalNamedType>,
    Vec<ProjectDiagnostic>,
) {
    let mut project_type_objects = project
        .objects()
        .filter(|object| {
            object.lifecycle == Lifecycle::Active
                && object.kind == ProjectObjectKind::TypeDefinition
        })
        .collect::<Vec<_>>();
    project_type_objects.sort_by_key(|object| (object.creation_ordinal, object.id));
    let admitted = project_type_objects
        .iter()
        .take(MAX_NAMED_TYPES)
        .map(|object| object.id)
        .collect::<BTreeSet<_>>();
    let type_objects = objects
        .iter()
        .copied()
        .filter(|object| object.kind == ProjectObjectKind::TypeDefinition)
        .collect::<Vec<_>>();
    let mut diagnostics = Vec::new();
    let invalid = duplicate_type_name_diagnostics(&type_objects, &mut diagnostics);
    for object in project_type_objects.iter().skip(MAX_NAMED_TYPES) {
        diagnostics.push(diagnostic(
            "EDU-SYS-2023",
            format!(
                "Named-type capacity exceeds the canonical project limit of {MAX_NAMED_TYPES}."
            ),
            object.id,
        ));
    }

    let mut definitions = BTreeMap::new();
    for object in type_objects
        .into_iter()
        .filter(|object| admitted.contains(&object.id))
    {
        if object.payload_schema != NAMED_TYPE_PAYLOAD_SCHEMA {
            diagnostics.push(diagnostic(
                "EDU-SYS-2020",
                format!(
                    "Named-type payload schema '{}' is unsupported; expected {NAMED_TYPE_PAYLOAD_SCHEMA}.",
                    object.payload_schema
                ),
                object.id,
            ));
            continue;
        }
        match parse_raw_named_type(object) {
            Ok(definition) => {
                definitions.insert(definition.declaration_id, definition);
            }
            Err(message) => diagnostics.push(diagnostic("EDU-SYS-2020", message, object.id)),
        }
    }

    let mut resolved = BTreeMap::new();
    let mut cache = BTreeMap::new();
    for id in definitions.keys().copied() {
        if invalid.contains(&id) {
            continue;
        }
        let mut stack = Vec::new();
        match resolve_named_type(id, &definitions, &invalid, &mut cache, &mut stack, 1) {
            Ok(canonical_type) => {
                let definition = &definitions[&id];
                resolved.insert(
                    id,
                    CanonicalNamedType {
                        object_id: definition.object_id,
                        declaration_id: id,
                        display_name: definition.display_name.clone(),
                        fingerprint: canonical_type.fingerprint(),
                        canonical_type,
                    },
                );
            }
            Err(message) => diagnostics.push(diagnostic(
                "EDU-SYS-2022",
                message,
                definitions[&id].object_id,
            )),
        }
    }
    (resolved, diagnostics)
}

fn duplicate_type_name_diagnostics(
    objects: &[&ProjectObject],
    diagnostics: &mut Vec<ProjectDiagnostic>,
) -> BTreeSet<TypeDeclarationId> {
    let mut names = BTreeMap::<String, Vec<&ProjectObject>>::new();
    for object in objects {
        names
            .entry(object.display_name.to_ascii_lowercase())
            .or_default()
            .push(object);
    }
    let mut invalid = BTreeSet::new();
    for duplicates in names.values().filter(|values| values.len() > 1) {
        let identities = duplicates
            .iter()
            .map(|object| object.id.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        for object in duplicates {
            invalid.insert(TypeDeclarationId::new(object.id.0));
            diagnostics.push(diagnostic(
                "EDU-SYS-2021",
                format!(
                    "Named type '{}' duplicates another controller-global type name (case-insensitive identities: {identities}).",
                    object.display_name
                ),
                object.id,
            ));
        }
    }
    invalid
}

fn parse_raw_named_type(object: &ProjectObject) -> Result<RawNamedType, String> {
    validate_type_identifier(&object.display_name)
        .map_err(|message| format!("named type '{}': {message}", object.display_name))?;
    if !object.id.0.is_rfc9562_v4() {
        return Err("named type object identity must be an RFC 9562 UUIDv4".to_owned());
    }
    Ok(RawNamedType {
        object_id: object.id,
        declaration_id: TypeDeclarationId::new(object.id.0),
        display_name: object.display_name.clone(),
        members: parse_raw_members(required_list(object, "members")?, "members", 1)?,
    })
}

fn parse_raw_members(
    values: &[PayloadValue],
    context: &str,
    depth: u8,
) -> Result<Vec<RawStructMember>, String> {
    if values.len() > usize::try_from(MAX_TYPE_MEMBERS).unwrap_or(usize::MAX) {
        return Err(format!(
            "{context} has {} members, exceeding {MAX_TYPE_MEMBERS}",
            values.len()
        ));
    }
    let mut ids = BTreeSet::new();
    let mut names = BTreeSet::new();
    let mut orders = BTreeSet::new();
    let mut members = Vec::with_capacity(values.len());
    for (index, value) in values.iter().enumerate() {
        let member_context = format!("{context}[{index}]");
        let record = as_record(value, &member_context)?;
        let id = parse_uuid(record_text(record, "id")?)?;
        if !id.is_rfc9562_v4() {
            return Err(format!("{member_context}.id must be an RFC 9562 UUIDv4"));
        }
        let name = record_text(record, "name")?.to_owned();
        validate_type_identifier(&name)
            .map_err(|message| format!("{member_context}.name: {message}"))?;
        let order = u32::try_from(record_decimal_unsigned_any(
            record,
            &["declaredOrder", "order"],
        )?)
        .map_err(|_| format!("{member_context}.declaredOrder exceeds UInt32"))?;
        if !ids.insert(id) {
            return Err(format!(
                "{member_context}.id duplicates member identity {id}"
            ));
        }
        if !names.insert(name.to_ascii_lowercase()) {
            return Err(format!(
                "{member_context}.name duplicates member name '{name}' case-insensitively"
            ));
        }
        if !orders.insert(order) {
            return Err(format!(
                "{member_context}.declaredOrder duplicates member order {order}"
            ));
        }
        let type_value = record
            .get("typeId")
            .or_else(|| record.get("type"))
            .ok_or_else(|| format!("{member_context}.typeId or .type is required"))?;
        let comment = record
            .get("comment")
            .map(|value| payload_text(value, "comment").map(str::to_owned))
            .transpose()?
            .unwrap_or_default();
        members.push(RawStructMember {
            id,
            name,
            declared_order: order,
            ty: parse_raw_type_expression(
                type_value,
                &format!("{member_context}.type"),
                depth.saturating_add(1),
            )?,
            comment,
        });
    }
    members.sort_by_key(|member| (member.declared_order, member.id));
    Ok(members)
}

fn parse_raw_type_expression(
    value: &PayloadValue,
    context: &str,
    depth: u8,
) -> Result<RawTypeExpression, String> {
    if depth > MAX_TYPE_DEPTH {
        return Err(format!(
            "{context} exceeds the canonical type nesting depth of {MAX_TYPE_DEPTH}"
        ));
    }
    match value {
        PayloadValue::String(value) => parse_raw_type_token(value, context),
        PayloadValue::Record(record) => match record_text(record, "kind")? {
            "array" => {
                let dimensions = match record.get("dimensions") {
                    Some(PayloadValue::List(values)) => values,
                    Some(_) => return Err(format!("{context}.dimensions must be a list")),
                    None => return Err(format!("{context}.dimensions is required")),
                };
                if dimensions.is_empty() || dimensions.len() > usize::from(MAX_ARRAY_DIMENSIONS) {
                    return Err(format!(
                        "{context}.dimensions must contain 1..={MAX_ARRAY_DIMENSIONS} bounds"
                    ));
                }
                let mut bounds = Vec::with_capacity(dimensions.len());
                for (index, value) in dimensions.iter().enumerate() {
                    let bound_context = format!("{context}.dimensions[{index}]");
                    let bound = as_record(value, &bound_context)?;
                    bounds.push(ArrayBound {
                        lower: i32::try_from(record_decimal_signed(bound, "lower")?)
                            .map_err(|_| format!("{bound_context}.lower exceeds Int32"))?,
                        upper: i32::try_from(record_decimal_signed(bound, "upper")?)
                            .map_err(|_| format!("{bound_context}.upper exceeds Int32"))?,
                    });
                }
                let element = record
                    .get("elementType")
                    .ok_or_else(|| format!("{context}.elementType is required"))?;
                Ok(RawTypeExpression::Array {
                    dimensions: bounds,
                    element_type: Box::new(parse_raw_type_expression(
                        element,
                        &format!("{context}.elementType"),
                        depth.saturating_add(1),
                    )?),
                })
            }
            "anonymous-struct" | "struct" => {
                let members = match record.get("members") {
                    Some(PayloadValue::List(values)) => values,
                    Some(_) => return Err(format!("{context}.members must be a list")),
                    None => return Err(format!("{context}.members is required")),
                };
                Ok(RawTypeExpression::AnonymousStruct(parse_raw_members(
                    members,
                    &format!("{context}.members"),
                    depth,
                )?))
            }
            kind => Err(format!(
                "{context}.kind '{kind}' is not a canonical type expression"
            )),
        },
        _ => Err(format!(
            "{context} must be a canonical type token or expression record"
        )),
    }
}

fn parse_raw_type_token(value: &str, context: &str) -> Result<RawTypeExpression, String> {
    if value
        .get(..5)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("TYPE:"))
    {
        return Uuid::parse(&value[5..])
            .map(TypeDeclarationId::new)
            .map(RawTypeExpression::Named)
            .map_err(|_| format!("{context} named reference '{value}' has an invalid UUID"));
    }
    let upper = value.to_ascii_uppercase();
    let primitive = match upper.as_str() {
        "BOOL" => PrimitiveType::Bool,
        "SINT" => PrimitiveType::Sint,
        "INT" => PrimitiveType::Int,
        "DINT" => PrimitiveType::Dint,
        "LINT" => PrimitiveType::Lint,
        "USINT" => PrimitiveType::Usint,
        "UINT" => PrimitiveType::Uint,
        "UDINT" => PrimitiveType::Udint,
        "ULINT" => PrimitiveType::Ulint,
        "BYTE" => PrimitiveType::Byte,
        "WORD" => PrimitiveType::Word,
        "DWORD" => PrimitiveType::Dword,
        "LWORD" => PrimitiveType::Lword,
        "REAL" => PrimitiveType::Real,
        "LREAL" => PrimitiveType::Lreal,
        "CHAR" => PrimitiveType::Char,
        "TIME" => PrimitiveType::Time,
        "EDGESTATE" => {
            return Ok(RawTypeExpression::InstructionState(
                InstructionStateKind::Edge,
            ));
        }
        "TIMERSTATE" => {
            return Ok(RawTypeExpression::InstructionState(
                InstructionStateKind::Timer,
            ));
        }
        "COUNTERSTATE" => {
            return Ok(RawTypeExpression::InstructionState(
                InstructionStateKind::Counter,
            ));
        }
        _ if upper.starts_with("STRING[") && upper.ends_with(']') => {
            let capacity = upper[7..upper.len() - 1]
                .parse::<u8>()
                .map_err(|_| format!("{context} has invalid STRING capacity '{value}'"))?;
            PrimitiveType::String(capacity)
        }
        _ => return Err(format!("{context} canonical type '{value}' is unsupported")),
    };
    Ok(RawTypeExpression::Primitive(primitive))
}

fn resolve_named_type(
    id: TypeDeclarationId,
    definitions: &BTreeMap<TypeDeclarationId, RawNamedType>,
    invalid: &BTreeSet<TypeDeclarationId>,
    cache: &mut BTreeMap<TypeDeclarationId, CanonicalType>,
    stack: &mut Vec<TypeDeclarationId>,
    depth: u8,
) -> Result<CanonicalType, String> {
    if depth > MAX_TYPE_DEPTH {
        return Err(format!(
            "named type TYPE:{id} exceeds the canonical nesting depth of {MAX_TYPE_DEPTH}"
        ));
    }
    if let Some(value) = cache.get(&id) {
        return Ok(value.clone());
    }
    if let Some(index) = stack.iter().position(|candidate| *candidate == id) {
        let mut cycle = stack[index..]
            .iter()
            .map(|candidate| format!("TYPE:{candidate}"))
            .collect::<Vec<_>>();
        cycle.push(format!("TYPE:{id}"));
        return Err(format!("named type cycle detected: {}", cycle.join(" -> ")));
    }
    if invalid.contains(&id) {
        return Err(format!(
            "named type TYPE:{id} is unavailable because its declaration is invalid"
        ));
    }
    let definition = definitions
        .get(&id)
        .ok_or_else(|| format!("named type reference TYPE:{id} does not exist"))?;
    stack.push(id);
    let result = (|| {
        let members = definition
            .members
            .iter()
            .map(|member| {
                resolve_struct_member(
                    member,
                    definitions,
                    invalid,
                    cache,
                    stack,
                    depth.saturating_add(1),
                )
            })
            .collect::<Result<Vec<_>, _>>()?;
        let canonical = CanonicalType::Named { id, members };
        canonical
            .validate(
                MAX_TYPE_DEPTH,
                MAX_TYPE_MEMBERS,
                MAX_ARRAY_DIMENSIONS,
                MAX_ARRAY_ELEMENTS,
            )
            .map_err(|error| {
                format!("named type TYPE:{id} failed canonical validation: {error}")
            })?;
        Ok(canonical)
    })();
    stack.pop();
    if let Ok(value) = &result {
        cache.insert(id, value.clone());
    }
    result
}

fn resolve_struct_member(
    member: &RawStructMember,
    definitions: &BTreeMap<TypeDeclarationId, RawNamedType>,
    invalid: &BTreeSet<TypeDeclarationId>,
    cache: &mut BTreeMap<TypeDeclarationId, CanonicalType>,
    stack: &mut Vec<TypeDeclarationId>,
    depth: u8,
) -> Result<StructMember, String> {
    Ok(StructMember {
        id: member.id,
        name: member.name.clone(),
        declared_order: member.declared_order,
        ty: resolve_type_expression(&member.ty, definitions, invalid, cache, stack, depth)?,
        reusable_default: None,
        comment: member.comment.clone(),
    })
}

fn resolve_type_expression(
    expression: &RawTypeExpression,
    definitions: &BTreeMap<TypeDeclarationId, RawNamedType>,
    invalid: &BTreeSet<TypeDeclarationId>,
    cache: &mut BTreeMap<TypeDeclarationId, CanonicalType>,
    stack: &mut Vec<TypeDeclarationId>,
    depth: u8,
) -> Result<CanonicalType, String> {
    if depth > MAX_TYPE_DEPTH {
        return Err(format!(
            "type expression exceeds the canonical nesting depth of {MAX_TYPE_DEPTH}"
        ));
    }
    match expression {
        RawTypeExpression::Primitive(value) => Ok(CanonicalType::Primitive(*value)),
        RawTypeExpression::InstructionState(value) => Ok(CanonicalType::InstructionState(*value)),
        RawTypeExpression::Named(id) => {
            resolve_named_type(*id, definitions, invalid, cache, stack, depth)
        }
        RawTypeExpression::Array {
            dimensions,
            element_type,
        } => Ok(CanonicalType::Array {
            dimensions: dimensions.clone(),
            element_type: Box::new(resolve_type_expression(
                element_type,
                definitions,
                invalid,
                cache,
                stack,
                depth.saturating_add(1),
            )?),
        }),
        RawTypeExpression::AnonymousStruct(members) => Ok(CanonicalType::AnonymousStruct(
            members
                .iter()
                .map(|member| {
                    resolve_struct_member(
                        member,
                        definitions,
                        invalid,
                        cache,
                        stack,
                        depth.saturating_add(1),
                    )
                })
                .collect::<Result<Vec<_>, _>>()?,
        )),
    }
}

fn validate_type_identifier(value: &str) -> Result<(), &'static str> {
    if value.is_empty()
        || value.len() > 128
        || !value.is_ascii()
        || !value
            .bytes()
            .next()
            .is_some_and(|byte| byte.is_ascii_alphabetic() || byte == b'_')
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
    {
        Err(
            "identifier must be 1..=128 ASCII letters, digits, or underscores and start with a letter or underscore",
        )
    } else {
        Ok(())
    }
}

fn parse_uuid(value: &str) -> Result<Uuid, String> {
    Uuid::parse(value).map_err(|_| format!("'{value}' is not a canonical UUID"))
}

enum ProjectedBody {
    Scl(String),
    Graphical(AuthoredLanguage),
}

fn project_block(
    object: &ProjectObject,
    named_types: &BTreeMap<TypeDeclarationId, CanonicalNamedType>,
) -> Result<(ProgramBlock, Option<ProjectedBody>), String> {
    let (kind, body) = match object.kind {
        ProjectObjectKind::ProgramBlock => {
            if object.payload_schema != PROGRAM_BLOCK_PAYLOAD_SCHEMA {
                return Err(format!(
                    "Program block payload schema '{}' is unsupported; expected {PROGRAM_BLOCK_PAYLOAD_SCHEMA}.",
                    object.payload_schema
                ));
            }
            let kind = parse_program_kind(object)?;
            let language = required_text(object, "language")?;
            let body = match language.to_ascii_lowercase().as_str() {
                "scl" => Some(ProjectedBody::Scl(
                    required_text(object, "sourceText")?.to_owned(),
                )),
                "lad" => Some(ProjectedBody::Graphical(AuthoredLanguage::Lad)),
                "fbd" => Some(ProjectedBody::Graphical(AuthoredLanguage::Fbd)),
                _ => return Err(format!("Authored language '{language}' is not admitted.")),
            };
            (kind, body)
        }
        ProjectObjectKind::DataBlock => {
            if object.payload_schema != DATA_BLOCK_PAYLOAD_SCHEMA {
                return Err(format!(
                    "Data block payload schema '{}' is unsupported; expected {DATA_BLOCK_PAYLOAD_SCHEMA}.",
                    object.payload_schema
                ));
            }
            (parse_data_block_kind(object)?, None)
        }
        _ => {
            return Err(
                "Only ProgramBlock and DataBlock objects project into software.".to_owned(),
            );
        }
    };
    let number = optional_unsigned(object, "engineeringNumber")?
        .unwrap_or_else(|| object.creation_ordinal % u64::from(u16::MAX - 1) + 1);
    let number = u16::try_from(number)
        .map_err(|_| "engineeringNumber exceeds UInt16".to_owned())
        .and_then(|value| {
            EngineeringNumber::new(value)
                .ok_or_else(|| "engineeringNumber must be nonzero".to_owned())
        })?;
    let interface = parse_interface(object, named_types)?;
    Ok((
        ProgramBlock::new(
            BlockId::new(object_u128(object.id)),
            object.display_name.clone(),
            number,
            kind,
            interface,
        ),
        body,
    ))
}

fn parse_program_kind(object: &ProjectObject) -> Result<ProgramUnitKind, String> {
    match required_text(object, "blockKind")?
        .to_ascii_uppercase()
        .as_str()
    {
        "OB" => match required_text(object, "obRole")? {
            "CyclicMain" => Ok(ProgramUnitKind::OrganizationBlock(
                ObDeclaration::CyclicMain,
            )),
            "Startup" => Ok(ProgramUnitKind::OrganizationBlock(ObDeclaration::Startup)),
            "TimedCyclic" => Ok(ProgramUnitKind::OrganizationBlock(
                ObDeclaration::TimedCyclic {
                    period_milliseconds: u32::try_from(required_unsigned(
                        object,
                        "periodMilliseconds",
                    )?)
                    .map_err(|_| "periodMilliseconds exceeds UInt32".to_owned())?,
                    offset_milliseconds: u32::try_from(
                        optional_unsigned(object, "offsetMilliseconds")?.unwrap_or(0),
                    )
                    .map_err(|_| "offsetMilliseconds exceeds UInt32".to_owned())?,
                    priority: u16::try_from(optional_unsigned(object, "priority")?.unwrap_or(1))
                        .map_err(|_| "priority exceeds UInt16".to_owned())?,
                },
            )),
            value => Err(format!("Organization-block role '{value}' is unsupported.")),
        },
        "FC" => Ok(ProgramUnitKind::Function),
        "FB" => Ok(ProgramUnitKind::FunctionBlock),
        value => Err(format!("Program kind '{value}' is unsupported.")),
    }
}

fn parse_data_block_kind(object: &ProjectObject) -> Result<ProgramUnitKind, String> {
    match required_text(object, "dbKind")? {
        "GlobalDB" => Ok(ProgramUnitKind::DataBlock(DataBlockKind::Global)),
        "InstanceDB" => Ok(ProgramUnitKind::DataBlock(DataBlockKind::Instance {
            fb_type: BlockId::new(object_u128(parse_object_id(required_text(
                object,
                "instanceOf",
            )?)?)),
        })),
        value => Err(format!("Data-block kind '{value}' is unsupported.")),
    }
}

fn parse_interface(
    object: &ProjectObject,
    named_types: &BTreeMap<TypeDeclarationId, CanonicalNamedType>,
) -> Result<BlockInterface, String> {
    let Some(value) = object
        .payload
        .semantic
        .get("interface")
        .or_else(|| object.payload.semantic.get("members"))
    else {
        return Ok(BlockInterface::default());
    };
    let values = match value {
        PayloadValue::List(values) => values.iter().collect::<Vec<_>>(),
        PayloadValue::Record(groups) => {
            let mut values = Vec::new();
            for key in [
                "inputs",
                "outputs",
                "inOuts",
                "statics",
                "temps",
                "constants",
            ] {
                match groups.get(key) {
                    Some(PayloadValue::List(group)) => values.extend(group),
                    Some(_) => return Err(format!("interface.{key} must be a list")),
                    None => return Err(format!("interface.{key} is absent")),
                }
            }
            match groups.get("return") {
                Some(PayloadValue::Null) => {}
                Some(value @ PayloadValue::Record(_)) => values.push(value),
                Some(_) => return Err("interface.return must be a record or null".to_owned()),
                None => return Err("interface.return is absent".to_owned()),
            }
            values
        }
        _ => {
            return Err(
                "interface must be a member list or BlockInterfaceContract record".to_owned(),
            );
        }
    };
    if values.len() > MAX_MEMBERS_PER_BLOCK {
        return Err(format!(
            "interface has {} members, exceeding {MAX_MEMBERS_PER_BLOCK}",
            values.len()
        ));
    }
    let mut members = Vec::with_capacity(values.len());
    let mut identities = BTreeSet::new();
    for value in values {
        let record = as_record(value, "interface member")?;
        let id = InterfaceMemberId::new(parse_identity(record_text(record, "id")?)?);
        if !identities.insert(id) {
            return Err(format!("interface member identity {id} is duplicated"));
        }
        let role = parse_role(record_text(record, "role")?)?;
        let data_type =
            parse_project_data_type(record_text_any(record, &["typeId", "type"])?, named_types)?;
        let order = u32::try_from(record_decimal_unsigned_any(
            record,
            &["declaredOrder", "order"],
        )?)
        .map_err(|_| "interface order exceeds UInt32".to_owned())?;
        let mut member = InterfaceMember::plain(
            id,
            record_text(record, "name")?,
            role,
            data_type.clone(),
            order,
        );
        member.default_value = optional_record_value(record, "defaultValue", &data_type)?;
        member.start_value = optional_record_value(record, "startValue", &data_type)?;
        member.constant_value = optional_record_value(record, "constantValue", &data_type)?
            .or(optional_record_value(record, "value", &data_type)?);
        member.retain_policy = record
            .get("retainPolicy")
            .map(|value| payload_text(value, "retainPolicy"))
            .transpose()?
            .map(|policy| match policy {
                "Retentive" => Ok(RetainPolicy::Retentive),
                "NonRetentive" => Ok(RetainPolicy::NonRetentive),
                _ => Err(format!("retainPolicy '{policy}' is unsupported")),
            })
            .transpose()?
            .or(record_bool(record, "retentive")?
                .and_then(|retentive| retentive.then_some(RetainPolicy::Retentive)));
        member.required_output_binding = record_bool(record, "requiredOutputBinding")?
            .or(record_bool(record, "requiredOutput")?)
            .unwrap_or(false);
        members.push(member);
    }
    Ok(BlockInterface::from_members(members))
}

fn parse_role(value: &str) -> Result<InterfaceRole, String> {
    match value.to_ascii_lowercase().as_str() {
        "input" => Ok(InterfaceRole::Input),
        "output" => Ok(InterfaceRole::Output),
        "inout" | "in_out" => Ok(InterfaceRole::InOut),
        "static" => Ok(InterfaceRole::Static),
        "temp" => Ok(InterfaceRole::Temp),
        "constant" => Ok(InterfaceRole::Constant),
        "return" => Ok(InterfaceRole::Return),
        _ => Err(format!("interface role '{value}' is unsupported")),
    }
}

pub(crate) fn parse_data_type(value: &str) -> Result<DataType, String> {
    let upper = value.to_ascii_uppercase();
    match upper.as_str() {
        "BOOL" => Ok(DataType::Bool),
        "SINT" => Ok(DataType::SInt),
        "INT" => Ok(DataType::Int),
        "DINT" => Ok(DataType::DInt),
        "LINT" => Ok(DataType::LInt),
        "USINT" => Ok(DataType::USInt),
        "UINT" => Ok(DataType::UInt),
        "UDINT" => Ok(DataType::UDInt),
        "ULINT" => Ok(DataType::ULInt),
        "BYTE" => Ok(DataType::Byte),
        "WORD" => Ok(DataType::Word),
        "DWORD" => Ok(DataType::DWord),
        "LWORD" => Ok(DataType::LWord),
        "REAL" => Ok(DataType::Real),
        "LREAL" => Ok(DataType::LReal),
        "CHAR" => Ok(DataType::Char),
        "TIME" => Ok(DataType::Time),
        _ if upper.starts_with("STRING[") && upper.ends_with(']') => {
            let capacity = upper[7..upper.len() - 1]
                .parse::<u16>()
                .map_err(|_| format!("invalid STRING capacity in '{value}'"))?;
            if capacity > 254 {
                return Err(format!(
                    "STRING capacity {capacity} exceeds the canonical limit of 254"
                ));
            }
            Ok(DataType::String { capacity })
        }
        _ => Err(format!("canonical data type '{value}' is unsupported")),
    }
}

fn parse_project_data_type(
    value: &str,
    named_types: &BTreeMap<TypeDeclarationId, CanonicalNamedType>,
) -> Result<DataType, String> {
    let Some(prefix) = value.get(..5) else {
        return parse_data_type(value);
    };
    if !prefix.eq_ignore_ascii_case("TYPE:") {
        return parse_data_type(value);
    }
    let identity = Uuid::parse(&value[5..])
        .map(TypeDeclarationId::new)
        .map_err(|_| format!("named type reference '{value}' does not contain a canonical UUID"))?;
    if !named_types.contains_key(&identity) {
        return Err(format!(
            "named type reference 'TYPE:{identity}' is absent or invalid"
        ));
    }
    Ok(DataType::Named(format!("TYPE:{identity}")))
}

fn optional_record_value(
    record: &BTreeMap<String, PayloadValue>,
    key: &str,
    data_type: &DataType,
) -> Result<Option<CanonicalValue>, String> {
    record
        .get(key)
        .map(|value| parse_value(value, data_type))
        .transpose()
}

pub(crate) fn parse_value(
    value: &PayloadValue,
    data_type: &DataType,
) -> Result<CanonicalValue, String> {
    if let PayloadValue::Record(record) = value {
        return parse_contract_value(record, data_type);
    }
    match (value, data_type) {
        (PayloadValue::Bool(value), DataType::Bool) => Ok(CanonicalValue::Bool(*value)),
        (PayloadValue::Signed(value), DataType::SInt) => i8::try_from(*value)
            .map(CanonicalValue::SInt)
            .map_err(|_| "SINT value is out of range".to_owned()),
        (PayloadValue::Signed(value), DataType::Int) => i16::try_from(*value)
            .map(CanonicalValue::Int)
            .map_err(|_| "INT value is out of range".to_owned()),
        (PayloadValue::Signed(value), DataType::DInt) => i32::try_from(*value)
            .map(CanonicalValue::DInt)
            .map_err(|_| "DINT value is out of range".to_owned()),
        (PayloadValue::Signed(value), DataType::LInt) => Ok(CanonicalValue::LInt(*value)),
        (PayloadValue::Unsigned(value), DataType::USInt) => u8::try_from(*value)
            .map(CanonicalValue::USInt)
            .map_err(|_| "USINT value is out of range".to_owned()),
        (PayloadValue::Unsigned(value), DataType::UInt) => u16::try_from(*value)
            .map(CanonicalValue::UInt)
            .map_err(|_| "UINT value is out of range".to_owned()),
        (PayloadValue::Unsigned(value), DataType::UDInt) => u32::try_from(*value)
            .map(CanonicalValue::UDInt)
            .map_err(|_| "UDINT value is out of range".to_owned()),
        (PayloadValue::Unsigned(value), DataType::ULInt) => Ok(CanonicalValue::ULInt(*value)),
        (PayloadValue::Unsigned(value), DataType::Byte) => u8::try_from(*value)
            .map(CanonicalValue::Byte)
            .map_err(|_| "BYTE bit pattern is out of range".to_owned()),
        (PayloadValue::Unsigned(value), DataType::Word) => u16::try_from(*value)
            .map(CanonicalValue::Word)
            .map_err(|_| "WORD bit pattern is out of range".to_owned()),
        (PayloadValue::Unsigned(value), DataType::DWord) => u32::try_from(*value)
            .map(CanonicalValue::DWord)
            .map_err(|_| "DWORD bit pattern is out of range".to_owned()),
        (PayloadValue::Unsigned(value), DataType::LWord) => Ok(CanonicalValue::LWord(*value)),
        (PayloadValue::Unsigned(value), DataType::Real) => u32::try_from(*value)
            .map_err(|_| "REAL bit pattern exceeds UInt32".to_owned())
            .and_then(canonical_real_value),
        (PayloadValue::Unsigned(value), DataType::LReal) => canonical_lreal_value(*value),
        (PayloadValue::Unsigned(value), DataType::Char) => u8::try_from(*value)
            .map(CanonicalValue::Char)
            .map_err(|_| "CHAR code unit is out of range".to_owned()),
        (PayloadValue::Signed(value), DataType::Time) => {
            Ok(CanonicalValue::TimeMilliseconds(*value))
        }
        (PayloadValue::String(value), DataType::String { capacity })
            if value.len() <= usize::from(*capacity) =>
        {
            Ok(CanonicalValue::StringBytes(value.as_bytes().to_vec()))
        }
        _ => Err("typed canonical value does not match its declared data type".to_owned()),
    }
}

fn parse_contract_value(
    record: &BTreeMap<String, PayloadValue>,
    data_type: &DataType,
) -> Result<CanonicalValue, String> {
    let kind = record_text(record, "kind")?;
    let actual_type_id = record_text(record, "typeId")?;
    let expected_type_id = canonical_data_type_id(data_type).ok_or_else(|| {
        "canonical typed values require a primitive declared data type".to_owned()
    })?;
    if actual_type_id != expected_type_id {
        return Err(format!(
            "canonical typed value typeId '{actual_type_id}' does not match declared type '{expected_type_id}'"
        ));
    }
    match kind {
        "bool" => parse_contract_bool(record, data_type),
        "signed-integer" => parse_contract_signed_integer(record, data_type),
        "unsigned-integer" => parse_contract_unsigned_integer(record, data_type),
        "bit-string" => parse_contract_bit_string(record, data_type),
        "floating" => parse_contract_floating(record, data_type),
        "char" => parse_contract_char(record, data_type),
        "string" => parse_contract_string(record, data_type),
        "time" => parse_contract_time(record, data_type),
        _ => Err("canonical typed value kind does not match its declared data type".to_owned()),
    }
}

fn parse_contract_bool(
    record: &BTreeMap<String, PayloadValue>,
    data_type: &DataType,
) -> Result<CanonicalValue, String> {
    require_contract_fields(record, &["kind", "typeId", "value"])?;
    match (record.get("value"), data_type) {
        (Some(PayloadValue::Bool(value)), DataType::Bool) => Ok(CanonicalValue::Bool(*value)),
        _ => Err("canonical BOOL value must contain Boolean value".to_owned()),
    }
}

fn parse_contract_signed_integer(
    record: &BTreeMap<String, PayloadValue>,
    data_type: &DataType,
) -> Result<CanonicalValue, String> {
    require_contract_fields(record, &["kind", "typeId", "value"])?;
    let value = record_contract_signed(record, "value")?;
    match data_type {
        DataType::SInt => i8::try_from(value)
            .map(CanonicalValue::SInt)
            .map_err(|_| "SINT value is out of range".to_owned()),
        DataType::Int => i16::try_from(value)
            .map(CanonicalValue::Int)
            .map_err(|_| "INT value is out of range".to_owned()),
        DataType::DInt => i32::try_from(value)
            .map(CanonicalValue::DInt)
            .map_err(|_| "DINT value is out of range".to_owned()),
        DataType::LInt => Ok(CanonicalValue::LInt(value)),
        _ => Err("canonical typed value kind does not match its declared data type".to_owned()),
    }
}

fn parse_contract_unsigned_integer(
    record: &BTreeMap<String, PayloadValue>,
    data_type: &DataType,
) -> Result<CanonicalValue, String> {
    require_contract_fields(record, &["kind", "typeId", "value"])?;
    let value = record_contract_unsigned(record, "value")?;
    match data_type {
        DataType::USInt => u8::try_from(value)
            .map(CanonicalValue::USInt)
            .map_err(|_| "USINT value is out of range".to_owned()),
        DataType::UInt => u16::try_from(value)
            .map(CanonicalValue::UInt)
            .map_err(|_| "UINT value is out of range".to_owned()),
        DataType::UDInt => u32::try_from(value)
            .map(CanonicalValue::UDInt)
            .map_err(|_| "UDINT value is out of range".to_owned()),
        DataType::ULInt => Ok(CanonicalValue::ULInt(value)),
        _ => Err("canonical typed value kind does not match its declared data type".to_owned()),
    }
}

fn parse_contract_bit_string(
    record: &BTreeMap<String, PayloadValue>,
    data_type: &DataType,
) -> Result<CanonicalValue, String> {
    require_contract_fields(record, &["bitsHex", "kind", "typeId"])?;
    match data_type {
        DataType::Byte => u8::try_from(record_contract_hex(record, "bitsHex", 2)?)
            .map(CanonicalValue::Byte)
            .map_err(|_| "BYTE bit pattern is out of range".to_owned()),
        DataType::Word => u16::try_from(record_contract_hex(record, "bitsHex", 4)?)
            .map(CanonicalValue::Word)
            .map_err(|_| "WORD bit pattern is out of range".to_owned()),
        DataType::DWord => u32::try_from(record_contract_hex(record, "bitsHex", 8)?)
            .map(CanonicalValue::DWord)
            .map_err(|_| "DWORD bit pattern is out of range".to_owned()),
        DataType::LWord => record_contract_hex(record, "bitsHex", 16).map(CanonicalValue::LWord),
        _ => Err("canonical typed value kind does not match its declared data type".to_owned()),
    }
}

fn parse_contract_floating(
    record: &BTreeMap<String, PayloadValue>,
    data_type: &DataType,
) -> Result<CanonicalValue, String> {
    require_contract_fields(record, &["ieeeHex", "kind", "typeId"])?;
    match data_type {
        DataType::Real => u32::try_from(record_contract_hex(record, "ieeeHex", 8)?)
            .map_err(|_| "REAL bit pattern exceeds UInt32".to_owned())
            .and_then(canonical_real_value),
        DataType::LReal => canonical_lreal_value(record_contract_hex(record, "ieeeHex", 16)?),
        _ => Err("canonical typed value kind does not match its declared data type".to_owned()),
    }
}

fn parse_contract_char(
    record: &BTreeMap<String, PayloadValue>,
    data_type: &DataType,
) -> Result<CanonicalValue, String> {
    require_contract_fields(record, &["codeUnit", "kind", "typeId"])?;
    if !matches!(data_type, DataType::Char) {
        return Err("canonical typed value kind does not match its declared data type".to_owned());
    }
    u8::try_from(record_unsigned(record, "codeUnit")?)
        .map(CanonicalValue::Char)
        .map_err(|_| "CHAR code unit is out of range".to_owned())
}

fn parse_contract_string(
    record: &BTreeMap<String, PayloadValue>,
    data_type: &DataType,
) -> Result<CanonicalValue, String> {
    require_contract_fields(record, &["capacity", "codeUnits", "kind", "typeId"])?;
    let DataType::String { capacity } = data_type else {
        return Err("canonical typed value kind does not match its declared data type".to_owned());
    };
    let contract_capacity = u16::try_from(record_unsigned(record, "capacity")?)
        .map_err(|_| "STRING contract capacity exceeds UInt16".to_owned())?;
    if contract_capacity != *capacity {
        return Err(format!(
            "STRING contract capacity {contract_capacity} does not match declared capacity {capacity}"
        ));
    }
    let Some(PayloadValue::List(code_units)) = record.get("codeUnits") else {
        return Err("canonical STRING codeUnits must be a list".to_owned());
    };
    if code_units.len() > usize::from(*capacity) {
        return Err("canonical STRING value exceeds its declared capacity".to_owned());
    }
    code_units
        .iter()
        .enumerate()
        .map(|(index, value)| match value {
            PayloadValue::Unsigned(value) => u8::try_from(*value)
                .map_err(|_| format!("canonical STRING codeUnits[{index}] exceeds UInt8")),
            _ => Err(format!(
                "canonical STRING codeUnits[{index}] must be unsigned"
            )),
        })
        .collect::<Result<Vec<_>, _>>()
        .map(CanonicalValue::StringBytes)
}

fn parse_contract_time(
    record: &BTreeMap<String, PayloadValue>,
    data_type: &DataType,
) -> Result<CanonicalValue, String> {
    require_contract_fields(record, &["kind", "milliseconds", "typeId"])?;
    if !matches!(data_type, DataType::Time) {
        return Err("canonical typed value kind does not match its declared data type".to_owned());
    }
    record_contract_signed(record, "milliseconds").map(CanonicalValue::TimeMilliseconds)
}

fn canonical_data_type_id(data_type: &DataType) -> Option<&'static str> {
    match data_type {
        DataType::Bool => Some("BOOL"),
        DataType::SInt => Some("SINT"),
        DataType::Int => Some("INT"),
        DataType::DInt => Some("DINT"),
        DataType::LInt => Some("LINT"),
        DataType::USInt => Some("USINT"),
        DataType::UInt => Some("UINT"),
        DataType::UDInt => Some("UDINT"),
        DataType::ULInt => Some("ULINT"),
        DataType::Byte => Some("BYTE"),
        DataType::Word => Some("WORD"),
        DataType::DWord => Some("DWORD"),
        DataType::LWord => Some("LWORD"),
        DataType::Real => Some("REAL"),
        DataType::LReal => Some("LREAL"),
        DataType::Char => Some("CHAR"),
        DataType::Time => Some("TIME"),
        DataType::String { .. } => Some("STRING"),
        DataType::Named(_) | DataType::BlockInstance(_) | DataType::InstructionState(_) => None,
    }
}

fn require_contract_fields(
    record: &BTreeMap<String, PayloadValue>,
    expected: &[&str],
) -> Result<(), String> {
    if record.len() == expected.len() && expected.iter().all(|key| record.contains_key(*key)) {
        return Ok(());
    }
    Err(format!(
        "canonical typed value fields must be exactly [{}]",
        expected.join(", ")
    ))
}

fn record_contract_signed(
    record: &BTreeMap<String, PayloadValue>,
    key: &str,
) -> Result<i64, String> {
    match record.get(key) {
        Some(PayloadValue::Signed(value)) => Ok(*value),
        Some(PayloadValue::String(value)) if canonical_signed_decimal(value) => value
            .parse::<i64>()
            .map_err(|_| format!("record field '{key}' exceeds canonical DecimalInt64")),
        Some(_) => Err(format!(
            "record field '{key}' must be signed or canonical DecimalInt64 text"
        )),
        None => Err(format!("required record field '{key}' is absent")),
    }
}

fn record_contract_unsigned(
    record: &BTreeMap<String, PayloadValue>,
    key: &str,
) -> Result<u64, String> {
    match record.get(key) {
        Some(PayloadValue::Unsigned(value)) => Ok(*value),
        Some(PayloadValue::String(value)) if canonical_unsigned_decimal(value) => value
            .parse::<u64>()
            .map_err(|_| format!("record field '{key}' exceeds canonical DecimalUInt64")),
        Some(_) => Err(format!(
            "record field '{key}' must be unsigned or canonical DecimalUInt64 text"
        )),
        None => Err(format!("required record field '{key}' is absent")),
    }
}

fn canonical_signed_decimal(value: &str) -> bool {
    value == "0"
        || value
            .strip_prefix('-')
            .is_some_and(canonical_nonzero_unsigned_decimal)
        || canonical_nonzero_unsigned_decimal(value)
}

fn canonical_unsigned_decimal(value: &str) -> bool {
    value == "0" || canonical_nonzero_unsigned_decimal(value)
}

fn canonical_nonzero_unsigned_decimal(value: &str) -> bool {
    value
        .as_bytes()
        .first()
        .is_some_and(|first| matches!(first, b'1'..=b'9'))
        && value.as_bytes()[1..].iter().all(u8::is_ascii_digit)
}

fn record_contract_hex(
    record: &BTreeMap<String, PayloadValue>,
    key: &str,
    digits: usize,
) -> Result<u64, String> {
    let value = record_text(record, key)?;
    if value.len() != digits
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'A'..=b'F'))
    {
        return Err(format!(
            "record field '{key}' must contain exactly {digits} uppercase hexadecimal digits"
        ));
    }
    u64::from_str_radix(value, 16)
        .map_err(|_| format!("record field '{key}' is not canonical hexadecimal"))
}

fn canonical_real_value(bits: u32) -> Result<CanonicalValue, String> {
    if CanonicalF32::from_bits(bits).bits() != bits {
        return Err("REAL NaN must use canonical bit pattern 7FC00000".to_owned());
    }
    Ok(CanonicalValue::RealBits(bits))
}

fn canonical_lreal_value(bits: u64) -> Result<CanonicalValue, String> {
    if CanonicalF64::from_bits(bits).bits() != bits {
        return Err("LREAL NaN must use canonical bit pattern 7FF8000000000000".to_owned());
    }
    Ok(CanonicalValue::LRealBits(bits))
}

fn parse_tag(
    object: &ProjectObject,
    named_types: &BTreeMap<TypeDeclarationId, CanonicalNamedType>,
) -> Result<CanonicalTag, String> {
    let data_type = parse_project_data_type(required_text(object, "dataType")?, named_types)?;
    let area = match required_text(object, "addressArea")?
        .to_ascii_uppercase()
        .as_str()
    {
        "I" => CanonicalAddressArea::Input,
        "Q" => CanonicalAddressArea::Output,
        "M" => CanonicalAddressArea::Memory,
        value => return Err(format!("tag addressArea '{value}' is unsupported")),
    };
    let expected_kind = match area {
        CanonicalAddressArea::Input => "Input",
        CanonicalAddressArea::Output => "Output",
        CanonicalAddressArea::Memory => "Memory",
    };
    if required_text(object, "tagKind")? != expected_kind {
        return Err(format!(
            "tagKind must be '{expected_kind}' for addressArea '{}'.",
            required_text(object, "addressArea")?
        ));
    }
    let block_object_id = parse_object_id(required_text(object, "blockId")?)?;
    let member_id = InterfaceMemberId::new(parse_identity(required_text(object, "memberId")?)?);
    let hardware = match area {
        CanonicalAddressArea::Memory => None,
        CanonicalAddressArea::Input | CanonicalAddressArea::Output => {
            let intent = match required_text(object, "addressIntent")? {
                "auto" => CanonicalAddressIntent::Auto,
                "explicit" => CanonicalAddressIntent::Explicit {
                    byte_offset: u32::try_from(required_unsigned(object, "byteOffset")?)
                        .map_err(|_| "byteOffset exceeds UInt32".to_owned())?,
                    bit_offset: u8::try_from(optional_unsigned(object, "bitOffset")?.unwrap_or(0))
                        .map_err(|_| "bitOffset exceeds UInt8".to_owned())?,
                },
                value => return Err(format!("tag addressIntent '{value}' is unsupported")),
            };
            Some(CanonicalHardwareAddress { area, intent })
        }
    };
    Ok(CanonicalTag {
        object_id: object.id,
        stable_identity: object_u128(object.id),
        display_name: object.display_name.clone(),
        data_type,
        target: CanonicalTagTarget {
            block_object_id,
            member_id,
            hardware,
        },
    })
}

fn parse_watch_table(object: &ProjectObject) -> Result<CanonicalWatchTable, String> {
    let values = required_list(object, "rows")?;
    if values.len() > MAX_WATCH_ROWS {
        return Err(format!("watch table exceeds {MAX_WATCH_ROWS} rows"));
    }
    let mut rows = Vec::with_capacity(values.len());
    for value in values {
        let record = as_record(value, "watch row")?;
        rows.push(CanonicalWatchRow {
            id: parse_identity(record_text(record, "id")?)?,
            target_tag: parse_object_id(record_text(record, "targetTag")?)?,
            layer: parse_layer(record_text(record, "layer")?)?,
            display_base: record
                .get("displayBase")
                .map(|value| payload_text(value, "displayBase").and_then(parse_display_base))
                .transpose()?
                .unwrap_or(CanonicalDisplayBase::Automatic),
            order: u32::try_from(record_unsigned(record, "order")?)
                .map_err(|_| "watch row order exceeds UInt32".to_owned())?,
        });
    }
    rows.sort_by_key(|row| (row.order, row.id));
    Ok(CanonicalWatchTable {
        object_id: object.id,
        name: object.display_name.clone(),
        rows,
    })
}

fn parse_trace_config(object: &ProjectObject) -> Result<CanonicalTraceConfig, String> {
    if optional_text(object, "trigger")?.unwrap_or("immediate") != "immediate" {
        return Err(
            "only the canonical immediate trace trigger is admitted in this slice".to_owned(),
        );
    }
    let values = required_list(object, "channels")?;
    if values.len() > MAX_TRACE_CHANNELS {
        return Err(format!(
            "trace configuration exceeds {MAX_TRACE_CHANNELS} channels"
        ));
    }
    let mut channels = Vec::with_capacity(values.len());
    for value in values {
        let record = as_record(value, "trace channel")?;
        channels.push(CanonicalTraceChannel {
            id: parse_identity(record_text(record, "id")?)?,
            alias: record_text(record, "alias")?.to_owned(),
            target_tag: parse_object_id(record_text(record, "targetTag")?)?,
            layer: parse_layer(record_text(record, "layer")?)?,
        });
    }
    let every_scans = u32::try_from(optional_unsigned(object, "everyScans")?.unwrap_or(1))
        .map_err(|_| "everyScans exceeds UInt32".to_owned())?;
    let pre_trigger_samples =
        usize::try_from(optional_unsigned(object, "preSamples")?.unwrap_or(0))
            .map_err(|_| "preSamples exceeds usize".to_owned())?;
    let post_trigger_samples =
        usize::try_from(optional_unsigned(object, "postSamples")?.unwrap_or(64))
            .map_err(|_| "postSamples exceeds usize".to_owned())?;
    Ok(CanonicalTraceConfig {
        object_id: object.id,
        name: object.display_name.clone(),
        channels,
        every_scans,
        pre_trigger_samples,
        post_trigger_samples,
        maximum_duration_ms: optional_unsigned(object, "maximumDurationMs")?.unwrap_or(60_000),
    })
}

fn parse_layer(value: &str) -> Result<CanonicalProbeLayer, String> {
    match value.to_ascii_lowercase().as_str() {
        "natural" => Ok(CanonicalProbeLayer::Natural),
        "effective" => Ok(CanonicalProbeLayer::Effective),
        "raw-input" => Ok(CanonicalProbeLayer::RawInput),
        "committed-output" => Ok(CanonicalProbeLayer::CommittedOutput),
        "delivered-output" => Ok(CanonicalProbeLayer::DeliveredOutput),
        _ => Err(format!("probe layer '{value}' is unsupported")),
    }
}

fn parse_display_base(value: &str) -> Result<CanonicalDisplayBase, String> {
    match value.to_ascii_lowercase().as_str() {
        "automatic" => Ok(CanonicalDisplayBase::Automatic),
        "binary" => Ok(CanonicalDisplayBase::Binary),
        "decimal" => Ok(CanonicalDisplayBase::Decimal),
        "hexadecimal" => Ok(CanonicalDisplayBase::Hexadecimal),
        _ => Err(format!("display base '{value}' is unsupported")),
    }
}

fn objects_for_controller(project: &Project, controller: ObjectId) -> Vec<&ProjectObject> {
    let mut values = project
        .objects()
        .filter(|object| {
            object.lifecycle == Lifecycle::Active
                && object.id != controller
                && belongs_to_controller(project, object, controller)
        })
        .collect::<Vec<_>>();
    values.sort_by_key(|object| (object.creation_ordinal, object.id));
    values
}

fn belongs_to_controller(project: &Project, object: &ProjectObject, controller: ObjectId) -> bool {
    let mut parent = object.parent_id;
    let mut remaining = project.objects().count();
    while let Some(id) = parent {
        if id == controller {
            return true;
        }
        let Some(candidate) = project.object(id) else {
            return false;
        };
        if candidate.kind == ProjectObjectKind::Controller {
            return false;
        }
        parent = candidate.parent_id;
        if remaining == 0 {
            return false;
        }
        remaining -= 1;
    }
    false
}

fn required_text<'a>(object: &'a ProjectObject, key: &str) -> Result<&'a str, String> {
    object
        .payload
        .semantic
        .get(key)
        .ok_or_else(|| format!("required semantic field '{key}' is absent"))
        .and_then(|value| payload_text(value, key))
}

fn optional_text<'a>(object: &'a ProjectObject, key: &str) -> Result<Option<&'a str>, String> {
    object
        .payload
        .semantic
        .get(key)
        .map(|value| payload_text(value, key))
        .transpose()
}

fn payload_text<'a>(value: &'a PayloadValue, key: &str) -> Result<&'a str, String> {
    match value {
        PayloadValue::String(value) => Ok(value),
        _ => Err(format!("semantic field '{key}' must be text")),
    }
}

fn required_unsigned(object: &ProjectObject, key: &str) -> Result<u64, String> {
    optional_unsigned(object, key)?
        .ok_or_else(|| format!("required unsigned semantic field '{key}' is absent"))
}

fn optional_unsigned(object: &ProjectObject, key: &str) -> Result<Option<u64>, String> {
    object
        .payload
        .semantic
        .get(key)
        .map(|value| match value {
            PayloadValue::Unsigned(value) => Ok(*value),
            _ => Err(format!("semantic field '{key}' must be unsigned")),
        })
        .transpose()
}

fn required_list<'a>(object: &'a ProjectObject, key: &str) -> Result<&'a [PayloadValue], String> {
    match object.payload.semantic.get(key) {
        Some(PayloadValue::List(values)) => Ok(values),
        Some(_) => Err(format!("semantic field '{key}' must be a list")),
        None => Err(format!("required list semantic field '{key}' is absent")),
    }
}

pub(crate) fn as_record<'a>(
    value: &'a PayloadValue,
    context: &str,
) -> Result<&'a BTreeMap<String, PayloadValue>, String> {
    match value {
        PayloadValue::Record(record) => Ok(record),
        _ => Err(format!("{context} must be a record")),
    }
}

pub(crate) fn record_text<'a>(
    record: &'a BTreeMap<String, PayloadValue>,
    key: &str,
) -> Result<&'a str, String> {
    record
        .get(key)
        .ok_or_else(|| format!("required record field '{key}' is absent"))
        .and_then(|value| payload_text(value, key))
}

fn record_text_any<'a>(
    record: &'a BTreeMap<String, PayloadValue>,
    keys: &[&str],
) -> Result<&'a str, String> {
    keys.iter()
        .find_map(|key| record.get(*key).map(|value| payload_text(value, key)))
        .unwrap_or_else(|| {
            Err(format!(
                "required record field '{}' is absent",
                keys.join(" or ")
            ))
        })
}

fn record_decimal_unsigned_any(
    record: &BTreeMap<String, PayloadValue>,
    keys: &[&str],
) -> Result<u64, String> {
    for key in keys {
        if let Some(value) = record.get(*key) {
            return match value {
                PayloadValue::Unsigned(value) => Ok(*value),
                PayloadValue::String(value) => value
                    .parse::<u64>()
                    .map_err(|_| format!("record field '{key}' is not canonical DecimalUInt64")),
                _ => Err(format!(
                    "record field '{key}' must be unsigned or canonical DecimalUInt64 text"
                )),
            };
        }
    }
    Err(format!(
        "required record field '{}' is absent",
        keys.join(" or ")
    ))
}

fn record_decimal_signed(
    record: &BTreeMap<String, PayloadValue>,
    key: &str,
) -> Result<i64, String> {
    match record.get(key) {
        Some(PayloadValue::Signed(value)) => Ok(*value),
        Some(PayloadValue::String(value)) => value
            .parse::<i64>()
            .map_err(|_| format!("record field '{key}' is not canonical DecimalInt64")),
        Some(_) => Err(format!(
            "record field '{key}' must be signed or canonical DecimalInt64 text"
        )),
        None => Err(format!("required record field '{key}' is absent")),
    }
}

pub(crate) fn record_unsigned(
    record: &BTreeMap<String, PayloadValue>,
    key: &str,
) -> Result<u64, String> {
    match record.get(key) {
        Some(PayloadValue::Unsigned(value)) => Ok(*value),
        Some(_) => Err(format!("record field '{key}' must be unsigned")),
        None => Err(format!("required record field '{key}' is absent")),
    }
}

fn record_bool(record: &BTreeMap<String, PayloadValue>, key: &str) -> Result<Option<bool>, String> {
    match record.get(key) {
        Some(PayloadValue::Bool(value)) => Ok(Some(*value)),
        Some(_) => Err(format!("record field '{key}' must be Boolean")),
        None => Ok(None),
    }
}

fn parse_object_id(value: &str) -> Result<ObjectId, String> {
    Uuid::parse(value)
        .map(ObjectId)
        .map_err(|_| format!("'{value}' is not a canonical UUID"))
}

pub(crate) fn parse_identity(value: &str) -> Result<u128, String> {
    Uuid::parse(value)
        .map(|id| u128::from_be_bytes(id.into_bytes()))
        .map_err(|_| format!("'{value}' is not a canonical UUID identity"))
}

#[must_use]
pub(crate) fn object_u128(value: ObjectId) -> u128 {
    u128::from_be_bytes(value.0.into_bytes())
}

fn diagnostic(
    code: &str,
    message: impl Into<String>,
    primary_object_id: ObjectId,
) -> ProjectDiagnostic {
    ProjectDiagnostic {
        blocking: true,
        code: code.to_owned(),
        message: message.into(),
        phase: ProjectDiagnosticPhase::SoftwareProjection,
        primary_object_id,
        related_object_ids: Vec::new(),
    }
}
