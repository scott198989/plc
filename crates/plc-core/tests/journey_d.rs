use std::collections::BTreeMap;

use plc_core::{
    CommandContext, CommandEnvelope, CommandOutcome, DecodeLimits, DomainCommand, Engine,
    KernelSession, Lifecycle, NewObject, ObjectId, Payload, ProfilePin, Project, ProjectObjectKind,
    ReferenceEdge, ReferenceKind, ResolutionState, Sha256Digest, TransactionId, Uuid,
    decode_project_package, encode_project_package,
};

struct CommandIds {
    next: u64,
}

impl CommandIds {
    const fn new() -> Self {
        Self { next: 1 }
    }

    fn envelope(
        &mut self,
        engine: &Engine,
        command: DomainCommand,
        preconditions: &[ObjectId],
    ) -> CommandEnvelope {
        let ordinal = self.next;
        self.next += 1;
        CommandEnvelope {
            command_id: Uuid::deterministic_v4(b"journey-d-command", ordinal),
            transaction_id: TransactionId(Uuid::deterministic_v4(
                b"journey-d-transaction",
                ordinal,
            )),
            expected_document_revision: engine.project().document_revision(),
            expected_object_revisions: preconditions
                .iter()
                .map(|id| {
                    (
                        *id,
                        engine
                            .project()
                            .object(*id)
                            .expect("precondition object")
                            .object_revision,
                    )
                })
                .collect(),
            context: CommandContext {
                actor_id: "journey-d".to_owned(),
                can_mutate: true,
            },
            command,
        }
    }
}

fn object(id: ObjectId, kind: ProjectObjectKind, parent_id: ObjectId, name: &str) -> NewObject {
    NewObject {
        id,
        kind,
        parent_id,
        display_name: name.to_owned(),
        payload_schema: "edu.journey-d/1".to_owned(),
        payload: Payload::default(),
    }
}

fn commit(
    engine: &mut Engine,
    ids: &mut CommandIds,
    command: DomainCommand,
    preconditions: &[ObjectId],
) -> plc_core::DomainCommandResult {
    let envelope = ids.envelope(engine, command, preconditions);
    let result = engine.execute(&envelope);
    assert_eq!(
        result.outcome,
        CommandOutcome::Committed,
        "{:?}",
        result.diagnostics
    );
    result
}

#[test]
#[allow(clippy::too_many_lines)]
fn journey_d_identity_persistence_and_corruption_are_end_to_end() {
    let document_id = Uuid::deterministic_v4(b"journey-d-document", 1);
    let root = ObjectId(Uuid::deterministic_v4(b"journey-d-root", 1));
    let source_folder = ObjectId(Uuid::deterministic_v4(b"journey-d-object", 1));
    let destination_folder = ObjectId(Uuid::deterministic_v4(b"journey-d-object", 2));
    let symbol_table = ObjectId(Uuid::deterministic_v4(b"journey-d-object", 3));
    let tag = ObjectId(Uuid::deterministic_v4(b"journey-d-object", 4));
    let block = ObjectId(Uuid::deterministic_v4(b"journey-d-object", 5));
    let project = Project::new(
        document_id,
        root,
        "Journey D",
        ProfilePin {
            id: "EDU-21-Core".to_owned(),
            version: "1.0".to_owned(),
            manifest_hash: Sha256Digest([0x21; 32]),
        },
    );
    let mut engine = Engine::new(project).expect("valid project");
    let mut ids = CommandIds::new();

    for spec in [
        object(source_folder, ProjectObjectKind::Folder, root, "Source"),
        object(
            destination_folder,
            ProjectObjectKind::Folder,
            root,
            "Destination",
        ),
        object(
            symbol_table,
            ProjectObjectKind::SymbolTable,
            source_folder,
            "Symbols",
        ),
        object(tag, ProjectObjectKind::Tag, symbol_table, "Start"),
        object(
            block,
            ProjectObjectKind::ProgramBlock,
            source_folder,
            "Main",
        ),
    ] {
        let parent = spec.parent_id;
        commit(
            &mut engine,
            &mut ids,
            DomainCommand::Create(spec),
            &[parent],
        );
    }

    let internal_reference = ReferenceEdge {
        source_id: block,
        source_location: "network:1/contact:1".to_owned(),
        target_id: tag,
        expected_target_kind: ProjectObjectKind::Tag,
        kind: ReferenceKind::Read,
        resolution: ResolutionState::Resolved,
    };
    let external_reference = ReferenceEdge {
        source_id: block,
        source_location: "metadata/project".to_owned(),
        target_id: root,
        expected_target_kind: ProjectObjectKind::Project,
        kind: ReferenceKind::Generic,
        resolution: ResolutionState::Resolved,
    };
    for edge in [internal_reference.clone(), external_reference.clone()] {
        commit(
            &mut engine,
            &mut ids,
            DomainCommand::AddReference(edge),
            &[block],
        );
    }

    commit(
        &mut engine,
        &mut ids,
        DomainCommand::Rename {
            object_id: tag,
            display_name: "RunRequest".to_owned(),
        },
        &[tag],
    );
    assert!(engine.project().references().any(|edge| {
        edge.source_id == block
            && edge.target_id == tag
            && edge.resolution == ResolutionState::Resolved
    }));

    let unrelated_block = engine.project().object(block).expect("block").clone();
    let delete = commit(
        &mut engine,
        &mut ids,
        DomainCommand::Delete { object_id: tag },
        &[tag],
    );
    assert_eq!(
        engine.project().object(tag).expect("tombstone").lifecycle,
        Lifecycle::Tombstoned
    );
    assert!(
        engine.project().references().any(|edge| {
            edge.target_id == tag && edge.resolution == ResolutionState::Unresolved
        })
    );
    let undo_token = delete.undo_token.expect("undo token");
    let undo = engine.undo(
        TransactionId(Uuid::deterministic_v4(b"journey-d-undo", 1)),
        undo_token,
    );
    assert_eq!(undo.outcome, CommandOutcome::Committed);
    assert_eq!(
        engine.project().object(tag).expect("restored").lifecycle,
        Lifecycle::Active
    );
    assert_eq!(engine.project().object(block), Some(&unrelated_block));
    assert!(
        engine
            .project()
            .references()
            .any(|edge| { edge.target_id == tag && edge.resolution == ResolutionState::Resolved })
    );

    let copy_folder = ObjectId(Uuid::deterministic_v4(b"journey-d-copy", 1));
    let copy_table = ObjectId(Uuid::deterministic_v4(b"journey-d-copy", 2));
    let copy_tag = ObjectId(Uuid::deterministic_v4(b"journey-d-copy", 3));
    let copy_block = ObjectId(Uuid::deterministic_v4(b"journey-d-copy", 4));
    let id_map = BTreeMap::from([
        (source_folder, copy_folder),
        (symbol_table, copy_table),
        (tag, copy_tag),
        (block, copy_block),
    ]);
    let preview = engine.preview_copy_closure(&[source_folder], &id_map, destination_folder);
    assert!(preview.can_commit, "{:?}", preview.diagnostics);
    assert_eq!(preview.remapped_internal_references, 1);
    assert_eq!(preview.preserved_external_references, 1);
    commit(
        &mut engine,
        &mut ids,
        DomainCommand::CopyClosure {
            roots: vec![source_folder],
            id_map,
            destination_parent: destination_folder,
        },
        &[source_folder, destination_folder],
    );
    assert!(engine.project().references().any(|edge| {
        edge.source_id == copy_block
            && edge.target_id == copy_tag
            && edge.resolution == ResolutionState::Resolved
    }));
    assert!(engine.project().references().any(|edge| {
        edge.source_id == copy_block
            && edge.target_id == root
            && edge.resolution == ResolutionState::Resolved
    }));

    let package_one = encode_project_package(engine.project(), "journey-d/1").expect("first save");
    let (opened, manifest) =
        decode_project_package(&package_one, DecodeLimits::default()).expect("open");
    assert!(!opened.is_document_dirty());
    assert_eq!(opened.saved_document_hash(), Some(manifest.package_hash));
    let package_two = encode_project_package(&opened, "journey-d/1").expect("second save");
    assert_eq!(package_one, package_two);
    assert_eq!(opened.document_hash(), engine.project().document_hash());

    let new_document_id = Uuid::deterministic_v4(b"journey-d-save-as", 1);
    let save_as = opened.for_save_as(new_document_id).expect("Save As");
    assert_eq!(save_as.document_id(), new_document_id);
    assert_eq!(save_as.root_id(), opened.root_id());
    assert_eq!(save_as.document_revision(), opened.document_revision());
    assert_eq!(
        save_as
            .objects()
            .map(|object| object.id)
            .collect::<Vec<_>>(),
        opened.objects().map(|object| object.id).collect::<Vec<_>>()
    );

    let mut session = KernelSession::from_project(opened).expect("session");
    let before_corrupt_open = session.project().document_hash();
    let mut corrupt = package_one;
    let middle = corrupt.len() / 2;
    corrupt[middle] ^= 0x80;
    assert!(
        session
            .replace_from_package(&corrupt, DecodeLimits::default())
            .is_err()
    );
    assert_eq!(session.project().document_hash(), before_corrupt_open);
}
