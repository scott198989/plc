#![allow(
    clippy::missing_errors_doc,
    clippy::too_many_lines,
    clippy::type_complexity
)]

use std::collections::{BTreeMap, BTreeSet};

use crate::hash::Sha256Digest;
use crate::model::{
    CommandEnvelope, CommandOutcome, DependencyEdge, DependencyReason, Diagnostic, DomainCommand,
    DomainCommandResult, DomainEvent, Lifecycle, ObjectId, Project, ProjectObject,
    ProjectValidationError, ReferenceEdge, ReferenceKind, ResolutionState, TransactionId,
    UndoToken, Uuid,
};

const MAX_HISTORY: usize = 256;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EngineError {
    InvalidProject(ProjectValidationError),
    UndoConflict,
    RedoConflict,
}

#[derive(Clone, Debug)]
pub struct Engine {
    project: Project,
    undo: Vec<HistoryEntry>,
    redo: Vec<RedoEntry>,
    committed_command_ids: BTreeSet<Uuid>,
    committed_transaction_ids: BTreeSet<TransactionId>,
    history_limit: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CopyPreview {
    pub closure: Vec<ObjectId>,
    pub id_map: BTreeMap<ObjectId, ObjectId>,
    pub remapped_internal_references: usize,
    pub preserved_external_references: usize,
    pub diagnostics: Vec<Diagnostic>,
    pub can_commit: bool,
}

impl Engine {
    pub fn new(project: Project) -> Result<Self, EngineError> {
        project.validate().map_err(EngineError::InvalidProject)?;
        Ok(Self {
            project,
            undo: Vec::new(),
            redo: Vec::new(),
            committed_command_ids: BTreeSet::new(),
            committed_transaction_ids: BTreeSet::new(),
            history_limit: MAX_HISTORY,
        })
    }

    #[must_use]
    pub fn project(&self) -> &Project {
        &self.project
    }

    #[must_use]
    pub fn into_project(self) -> Project {
        self.project
    }

    pub fn set_history_limit(&mut self, limit: usize) {
        self.history_limit = limit.clamp(1, MAX_HISTORY);
        trim_history(&mut self.undo, self.history_limit);
    }

    /// Records a fully verified save checkpoint without creating a domain
    /// transaction or incrementing any revision.
    pub fn acknowledge_verified_save(
        &mut self,
        expected_content_hash: Sha256Digest,
        verified_package_hash: Sha256Digest,
    ) -> bool {
        if self.project.document_hash() != expected_content_hash {
            return false;
        }
        self.project.mark_saved_verified(verified_package_hash);
        true
    }

    #[must_use]
    pub fn preview_copy_closure(
        &self,
        roots: &[ObjectId],
        id_map: &BTreeMap<ObjectId, ObjectId>,
        destination_parent: ObjectId,
    ) -> CopyPreview {
        copy_preview(&self.project, roots, id_map, destination_parent)
    }

    #[must_use]
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    #[must_use]
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    #[must_use]
    pub fn next_undo_token(&self) -> Option<UndoToken> {
        self.undo.last().map(|entry| entry.undo_token)
    }

    pub fn execute(&mut self, envelope: &CommandEnvelope) -> DomainCommandResult {
        let before_hash = self.project.document_hash();
        if self.committed_command_ids.contains(&envelope.command_id)
            || self
                .committed_transaction_ids
                .contains(&envelope.transaction_id)
        {
            return rejected_result(
                envelope.transaction_id,
                before_hash,
                diagnostic(
                    "KRN_DUPLICATE_TRANSACTION_ID",
                    "The command or transaction identity was already committed in this session.",
                    Vec::new(),
                ),
            );
        }
        if let Some(diagnostic) = validate_envelope(&self.project, envelope) {
            return rejected_result(envelope.transaction_id, before_hash, diagnostic);
        }

        let before = self.project.clone();
        let mut candidate = before.clone();
        let events = match apply_command(&mut candidate, &envelope.command) {
            Ok(events) => events,
            Err(diagnostic) => {
                return rejected_result(envelope.transaction_id, before_hash, diagnostic);
            }
        };
        if let Err(error) = candidate.validate() {
            return rejected_result(
                envelope.transaction_id,
                before_hash,
                Diagnostic {
                    code: "KRN_INVALID_RESULT".to_owned(),
                    message: format!("Command would create an invalid project: {error:?}"),
                    object_ids: Vec::new(),
                },
            );
        }

        let history = HistoryEntry::between(
            envelope.transaction_id,
            UndoToken(envelope.transaction_id.0),
            &before,
            &candidate,
        );
        let affected = history.affected_ids();
        let after_hash = candidate.document_hash();
        self.project = candidate;
        self.committed_command_ids.insert(envelope.command_id);
        self.committed_transaction_ids
            .insert(envelope.transaction_id);
        self.undo.push(history);
        trim_history(&mut self.undo, self.history_limit);
        self.redo.clear();

        DomainCommandResult {
            outcome: CommandOutcome::Committed,
            transaction_id: envelope.transaction_id,
            affected_object_ids: affected,
            domain_events: events,
            diagnostics: Vec::new(),
            undo_token: Some(UndoToken(envelope.transaction_id.0)),
            before_project_hash: before_hash,
            after_project_hash: Some(after_hash),
        }
    }

    pub fn undo(&mut self, transaction_id: TransactionId, token: UndoToken) -> DomainCommandResult {
        let before_hash = self.project.document_hash();
        if transaction_id.0 == Uuid::NIL
            || !transaction_id.0.is_rfc9562_v4()
            || self.committed_transaction_ids.contains(&transaction_id)
        {
            return blocked_history_result(
                transaction_id,
                before_hash,
                "KRN_DUPLICATE_TRANSACTION_ID",
                "Undo requires a new non-nil transaction identity.",
            );
        }
        let Some(history) = self.undo.last().cloned() else {
            return blocked_history_result(
                transaction_id,
                before_hash,
                "KRN_UNDO_EMPTY",
                "No committed transaction is available to undo.",
            );
        };
        if history.undo_token != token
            || history.transaction_id.0 != token.0
            || !history.matches_after(&self.project)
        {
            return blocked_history_result(
                transaction_id,
                before_hash,
                "KRN_UNDO_CONFLICT",
                "Undo preconditions no longer match the committed transaction.",
            );
        }

        let mut candidate = self.project.clone();
        if let Err(error) =
            apply_history_direction(&mut candidate, &history, HistoryDirection::Undo)
        {
            return blocked_history_result(
                transaction_id,
                before_hash,
                &error.code,
                &error.message,
            );
        }
        if let Err(error) = candidate.validate() {
            return blocked_history_result(
                transaction_id,
                before_hash,
                "KRN_UNDO_INVALID",
                &format!("Undo would create an invalid project: {error:?}"),
            );
        }

        self.undo.pop();
        let expected_hash = candidate.document_hash();
        self.redo.push(RedoEntry {
            history: history.clone(),
            expected_hash,
        });
        let affected = history.affected_ids();
        let events = history.undo_events();
        self.project = candidate;
        self.committed_transaction_ids.insert(transaction_id);
        DomainCommandResult {
            outcome: CommandOutcome::Committed,
            transaction_id,
            affected_object_ids: affected,
            domain_events: events,
            diagnostics: Vec::new(),
            undo_token: None,
            before_project_hash: before_hash,
            after_project_hash: Some(expected_hash),
        }
    }

    pub fn redo(&mut self, transaction_id: TransactionId) -> DomainCommandResult {
        let before_hash = self.project.document_hash();
        if transaction_id.0 == Uuid::NIL
            || !transaction_id.0.is_rfc9562_v4()
            || self.committed_transaction_ids.contains(&transaction_id)
        {
            return blocked_history_result(
                transaction_id,
                before_hash,
                "KRN_DUPLICATE_TRANSACTION_ID",
                "Redo requires a new non-nil transaction identity.",
            );
        }
        let Some(entry) = self.redo.last().cloned() else {
            return blocked_history_result(
                transaction_id,
                before_hash,
                "KRN_REDO_EMPTY",
                "No transaction is available to redo.",
            );
        };
        if entry.expected_hash != before_hash
            || !entry.history.matches_before_semantics(&self.project)
        {
            return blocked_history_result(
                transaction_id,
                before_hash,
                "KRN_REDO_CONFLICT",
                "Redo preconditions were invalidated by an intervening mutation.",
            );
        }

        let before = self.project.clone();
        let mut candidate = before.clone();
        if let Err(error) =
            apply_history_direction(&mut candidate, &entry.history, HistoryDirection::Redo)
        {
            return blocked_history_result(
                transaction_id,
                before_hash,
                &error.code,
                &error.message,
            );
        }
        if let Err(error) = candidate.validate() {
            return blocked_history_result(
                transaction_id,
                before_hash,
                "KRN_REDO_INVALID",
                &format!("Redo would create an invalid project: {error:?}"),
            );
        }
        self.redo.pop();
        let redo_history = HistoryEntry::between(
            transaction_id,
            UndoToken(transaction_id.0),
            &before,
            &candidate,
        );
        let affected = redo_history.affected_ids();
        let events = entry.history.redo_events();
        let after_hash = candidate.document_hash();
        self.project = candidate;
        self.committed_transaction_ids.insert(transaction_id);
        self.undo.push(redo_history);
        trim_history(&mut self.undo, self.history_limit);

        DomainCommandResult {
            outcome: CommandOutcome::Committed,
            transaction_id,
            affected_object_ids: affected,
            domain_events: events,
            diagnostics: Vec::new(),
            undo_token: Some(UndoToken(transaction_id.0)),
            before_project_hash: before_hash,
            after_project_hash: Some(after_hash),
        }
    }
}

fn validate_envelope(project: &Project, envelope: &CommandEnvelope) -> Option<Diagnostic> {
    if envelope.command_id == Uuid::NIL
        || !envelope.command_id.is_rfc9562_v4()
        || envelope.transaction_id.0 == Uuid::NIL
        || !envelope.transaction_id.0.is_rfc9562_v4()
    {
        return Some(diagnostic(
            "KRN_INVALID_COMMAND_ID",
            "Command and transaction identities must be non-nil.",
            Vec::new(),
        ));
    }
    if !envelope.context.can_mutate || envelope.context.actor_id.is_empty() {
        return Some(diagnostic(
            "KRN_UNAUTHORIZED",
            "The command context does not authorize mutation.",
            Vec::new(),
        ));
    }
    if envelope.expected_document_revision != project.document_revision {
        return Some(diagnostic(
            "KRN_STALE_DOCUMENT_REVISION",
            "The expected document revision is stale.",
            Vec::new(),
        ));
    }
    let required = required_precondition_ids(&envelope.command);
    for id in &required {
        let Some(expected) = envelope.expected_object_revisions.get(id) else {
            return Some(diagnostic(
                "KRN_MISSING_OBJECT_PRECONDITION",
                "A required object revision precondition is missing.",
                vec![*id],
            ));
        };
        let Some(actual) = project.objects.get(id) else {
            return Some(diagnostic(
                "KRN_UNKNOWN_OBJECT",
                "A command precondition names an unknown object.",
                vec![*id],
            ));
        };
        if actual.object_revision != *expected {
            return Some(diagnostic(
                "KRN_STALE_OBJECT_REVISION",
                "An expected object revision is stale.",
                vec![*id],
            ));
        }
    }
    for (id, expected) in &envelope.expected_object_revisions {
        if project
            .objects
            .get(id)
            .is_none_or(|object| object.object_revision != *expected)
        {
            return Some(diagnostic(
                "KRN_STALE_OBJECT_REVISION",
                "An expected object revision is stale or unknown.",
                vec![*id],
            ));
        }
    }
    None
}

fn required_precondition_ids(command: &DomainCommand) -> BTreeSet<ObjectId> {
    match command {
        DomainCommand::Create(spec) => BTreeSet::from([spec.parent_id]),
        DomainCommand::Rename { object_id, .. }
        | DomainCommand::SetSemanticField { object_id, .. }
        | DomainCommand::SetPresentationField { object_id, .. }
        | DomainCommand::Delete { object_id } => BTreeSet::from([*object_id]),
        DomainCommand::Move {
            object_id,
            parent_id,
        } => BTreeSet::from([*object_id, *parent_id]),
        DomainCommand::CopyClosure {
            roots,
            destination_parent,
            ..
        } => roots.iter().copied().chain([*destination_parent]).collect(),
        DomainCommand::AddReference(edge) | DomainCommand::RemoveReference(edge) => {
            BTreeSet::from([edge.source_id])
        }
        DomainCommand::AddDependency(edge) | DomainCommand::RemoveDependency(edge) => {
            BTreeSet::from([edge.source_id, edge.target_id])
        }
    }
}

fn apply_command(
    project: &mut Project,
    command: &DomainCommand,
) -> Result<Vec<DomainEvent>, Diagnostic> {
    let document_before = project.document_revision;
    let semantic_before = project.semantic_revision;
    let semantic_fingerprint_before = project.semantic_fingerprint();
    let mut events = Vec::new();
    let _declared_semantic_change = match command {
        DomainCommand::Create(spec) => {
            validate_new_object(project, spec)?;
            let object = ProjectObject {
                id: spec.id,
                kind: spec.kind,
                object_revision: 1,
                semantic_revision: 1,
                creation_ordinal: project.next_creation_ordinal,
                parent_id: Some(spec.parent_id),
                display_name: spec.display_name.clone(),
                payload_schema: spec.payload_schema.clone(),
                payload: spec.payload.clone(),
                lifecycle: Lifecycle::Active,
            };
            project.next_creation_ordinal = project
                .next_creation_ordinal
                .checked_add(1)
                .ok_or_else(|| {
                    diagnostic(
                        "KRN_ORDINAL_EXHAUSTED",
                        "Creation ordinal exhausted.",
                        vec![spec.id],
                    )
                })?;
            project.objects.insert(spec.id, object);
            events.push(DomainEvent::Created(spec.id));
            true
        }
        DomainCommand::Rename {
            object_id,
            display_name,
        } => {
            validate_name(display_name, *object_id)?;
            let object = active_object_mut(project, *object_id)?;
            if object.display_name == *display_name {
                return Err(diagnostic(
                    "KRN_NO_CHANGE",
                    "Rename does not change the object.",
                    vec![*object_id],
                ));
            }
            object.display_name.clone_from(display_name);
            let semantic = object.kind.name_is_semantic();
            bump_object(object, semantic)?;
            events.push(DomainEvent::Renamed(*object_id));
            semantic
        }
        DomainCommand::Move {
            object_id,
            parent_id,
        } => {
            if object_id == parent_id {
                return Err(diagnostic(
                    "KRN_CONTAINMENT_CYCLE",
                    "An object cannot contain itself.",
                    vec![*object_id],
                ));
            }
            let parent_kind = active_object(project, *parent_id)?.kind;
            let object = active_object_mut(project, *object_id)?;
            if object.kind == crate::model::ProjectObjectKind::Project {
                return Err(diagnostic(
                    "KRN_ROOT_MOVE",
                    "The project root cannot be moved.",
                    vec![*object_id],
                ));
            }
            if !parent_kind.can_contain(object.kind) {
                return Err(diagnostic(
                    "KRN_ILLEGAL_CONTAINMENT",
                    "The destination object kind cannot contain this object kind.",
                    vec![*parent_id, *object_id],
                ));
            }
            if object.parent_id == Some(*parent_id) {
                return Err(diagnostic(
                    "KRN_NO_CHANGE",
                    "Move does not change the parent object.",
                    vec![*object_id],
                ));
            }
            let semantic = object.kind.containment_is_semantic();
            object.parent_id = Some(*parent_id);
            bump_object(object, semantic)?;
            events.push(DomainEvent::Moved(*object_id));
            semantic
        }
        DomainCommand::SetSemanticField {
            object_id,
            key,
            value,
        } => {
            validate_field_key(key, *object_id)?;
            let object = active_object_mut(project, *object_id)?;
            object.payload.semantic.insert(key.clone(), value.clone());
            bump_object(object, true)?;
            events.push(DomainEvent::Changed(*object_id));
            true
        }
        DomainCommand::SetPresentationField {
            object_id,
            key,
            value,
        } => {
            validate_field_key(key, *object_id)?;
            let object = active_object_mut(project, *object_id)?;
            object
                .payload
                .presentation
                .insert(key.clone(), value.clone());
            bump_object(object, false)?;
            events.push(DomainEvent::Changed(*object_id));
            false
        }
        DomainCommand::Delete { object_id } => {
            if *object_id == project.root_id {
                return Err(diagnostic(
                    "KRN_ROOT_DELETE",
                    "The project root cannot be deleted.",
                    vec![*object_id],
                ));
            }
            active_object(project, *object_id)?;
            let closure = containment_closure(project, &[*object_id]);
            for id in &closure {
                let object = project
                    .objects
                    .get_mut(id)
                    .expect("closure is derived from objects");
                object.lifecycle = Lifecycle::Tombstoned;
                bump_object(object, true)?;
                events.push(DomainEvent::Deleted(*id));
            }
            refresh_reference_resolution(project);
            true
        }
        DomainCommand::CopyClosure {
            roots,
            id_map,
            destination_parent,
        } => {
            let preview = copy_preview(project, roots, id_map, *destination_parent);
            if let Some(diagnostic) = preview.diagnostics.into_iter().next() {
                return Err(diagnostic);
            }
            let closure: BTreeSet<_> = preview.closure.into_iter().collect();
            let mut ordered: Vec<_> = closure.iter().copied().collect();
            ordered.sort_by_key(|id| project.objects[id].creation_ordinal);
            for source_id in ordered {
                let source = project.objects[&source_id].clone();
                let copy_id = id_map[&source_id];
                let parent_id = source
                    .parent_id
                    .and_then(|parent| id_map.get(&parent).copied())
                    .or_else(|| roots.contains(&source_id).then_some(*destination_parent))
                    .or(source.parent_id);
                let copy = ProjectObject {
                    id: copy_id,
                    kind: source.kind,
                    object_revision: 1,
                    semantic_revision: 1,
                    creation_ordinal: project.next_creation_ordinal,
                    parent_id,
                    display_name: source.display_name,
                    payload_schema: source.payload_schema,
                    payload: source.payload,
                    lifecycle: Lifecycle::Active,
                };
                project.next_creation_ordinal = project
                    .next_creation_ordinal
                    .checked_add(1)
                    .ok_or_else(|| {
                        diagnostic(
                            "KRN_ORDINAL_EXHAUSTED",
                            "Creation ordinal exhausted.",
                            vec![copy_id],
                        )
                    })?;
                project.objects.insert(copy_id, copy);
                events.push(DomainEvent::Copied {
                    source: source_id,
                    copy: copy_id,
                });
            }
            let copied_references: Vec<_> = project
                .references
                .iter()
                .filter(|edge| closure.contains(&edge.source_id))
                .cloned()
                .map(|mut edge| {
                    edge.source_id = id_map[&edge.source_id];
                    if let Some(target) = id_map.get(&edge.target_id) {
                        edge.target_id = *target;
                    }
                    edge
                })
                .collect();
            project.references.extend(copied_references);
            let copied_dependencies: Vec<_> = project
                .dependencies
                .iter()
                .filter(|edge| closure.contains(&edge.source_id))
                .cloned()
                .map(|mut edge| {
                    edge.source_id = id_map[&edge.source_id];
                    if let Some(target) = id_map.get(&edge.target_id) {
                        edge.target_id = *target;
                    }
                    edge
                })
                .collect();
            project.dependencies.extend(copied_dependencies);
            refresh_reference_resolution(project);
            true
        }
        DomainCommand::AddReference(edge) => {
            if edge.kind == ReferenceKind::HmiBindReserved {
                return Err(diagnostic(
                    "KRN_RESERVED_REFERENCE_KIND",
                    "HMI bindings are reserved and cannot be created in Phase 2.",
                    vec![edge.source_id],
                ));
            }
            active_object(project, edge.source_id)?;
            let mut normalized = edge.clone();
            normalized.resolution = resolved_state(project, edge);
            if !project.references.insert(normalized) {
                return Err(diagnostic(
                    "KRN_DUPLICATE_REFERENCE",
                    "The reference already exists.",
                    vec![edge.source_id, edge.target_id],
                ));
            }
            bump_by_id(project, edge.source_id, true)?;
            events.push(DomainEvent::ReferenceChanged {
                source: edge.source_id,
                target: edge.target_id,
            });
            true
        }
        DomainCommand::RemoveReference(edge) => {
            let found = project
                .references
                .iter()
                .find(|candidate| {
                    candidate.source_id == edge.source_id
                        && candidate.source_location == edge.source_location
                        && candidate.target_id == edge.target_id
                        && candidate.expected_target_kind == edge.expected_target_kind
                        && candidate.kind == edge.kind
                })
                .cloned();
            let Some(found) = found else {
                return Err(diagnostic(
                    "KRN_UNKNOWN_REFERENCE",
                    "The reference does not exist.",
                    vec![edge.source_id, edge.target_id],
                ));
            };
            project.references.remove(&found);
            bump_by_id(project, edge.source_id, true)?;
            events.push(DomainEvent::ReferenceChanged {
                source: edge.source_id,
                target: edge.target_id,
            });
            true
        }
        DomainCommand::AddDependency(edge) => {
            if edge.reason == DependencyReason::HmiBindingReserved {
                return Err(diagnostic(
                    "KRN_RESERVED_DEPENDENCY_REASON",
                    "HMI dependency creation is reserved for a later phase.",
                    vec![edge.source_id],
                ));
            }
            active_object(project, edge.source_id)?;
            active_object(project, edge.target_id)?;
            if !project.dependencies.insert(edge.clone()) {
                return Err(diagnostic(
                    "KRN_DUPLICATE_DEPENDENCY",
                    "The dependency already exists.",
                    vec![edge.source_id, edge.target_id],
                ));
            }
            bump_by_id(project, edge.source_id, true)?;
            events.push(DomainEvent::DependencyChanged {
                source: edge.source_id,
                target: edge.target_id,
            });
            true
        }
        DomainCommand::RemoveDependency(edge) => {
            if !project.dependencies.remove(edge) {
                return Err(diagnostic(
                    "KRN_UNKNOWN_DEPENDENCY",
                    "The dependency does not exist.",
                    vec![edge.source_id, edge.target_id],
                ));
            }
            bump_by_id(project, edge.source_id, true)?;
            events.push(DomainEvent::DependencyChanged {
                source: edge.source_id,
                target: edge.target_id,
            });
            true
        }
    };

    project.document_revision = document_before.checked_add(1).ok_or_else(|| {
        diagnostic(
            "KRN_REVISION_EXHAUSTED",
            "Document revision exhausted.",
            Vec::new(),
        )
    })?;
    if project.semantic_fingerprint() != semantic_fingerprint_before {
        project.semantic_revision = semantic_before.checked_add(1).ok_or_else(|| {
            diagnostic(
                "KRN_REVISION_EXHAUSTED",
                "Semantic revision exhausted.",
                Vec::new(),
            )
        })?;
    }
    Ok(events)
}

fn validate_new_object(
    project: &Project,
    spec: &crate::model::NewObject,
) -> Result<(), Diagnostic> {
    if spec.id.0 == Uuid::NIL || !spec.id.0.is_rfc9562_v4() {
        return Err(diagnostic(
            "KRN_INVALID_UUID",
            "New object identity must be a non-nil RFC 9562 UUIDv4.",
            vec![spec.id],
        ));
    }
    if project.objects.contains_key(&spec.id) {
        return Err(diagnostic(
            "KRN_UUID_COLLISION",
            "The new object UUID already exists.",
            vec![spec.id],
        ));
    }
    if spec.kind == crate::model::ProjectObjectKind::Project {
        return Err(diagnostic(
            "KRN_DUPLICATE_ROOT",
            "A project can contain only one root object.",
            vec![spec.id],
        ));
    }
    let parent = active_object(project, spec.parent_id)?;
    if !parent.kind.can_contain(spec.kind) {
        return Err(diagnostic(
            "KRN_ILLEGAL_CONTAINMENT",
            "The parent object kind cannot contain the new object kind.",
            vec![spec.parent_id, spec.id],
        ));
    }
    validate_name(&spec.display_name, spec.id)?;
    if spec.payload_schema.is_empty() || spec.payload_schema.len() > 128 {
        return Err(diagnostic(
            "KRN_INVALID_PAYLOAD_SCHEMA",
            "Payload schema identity must contain 1 through 128 bytes.",
            vec![spec.id],
        ));
    }
    Ok(())
}

fn validate_copy_map(
    project: &Project,
    closure: &BTreeSet<ObjectId>,
    id_map: &BTreeMap<ObjectId, ObjectId>,
) -> Result<(), Diagnostic> {
    if closure.is_empty()
        || closure.len() != id_map.len()
        || !closure.iter().all(|id| id_map.contains_key(id))
    {
        return Err(diagnostic(
            "KRN_INCOMPLETE_COPY_MAP",
            "Copy identity map must cover the complete copied closure exactly.",
            closure.iter().copied().collect(),
        ));
    }
    let mut targets = BTreeSet::new();
    for (source, target) in id_map {
        if source == target
            || target.0 == Uuid::NIL
            || !target.0.is_rfc9562_v4()
            || project.objects.contains_key(target)
            || !targets.insert(*target)
        {
            return Err(diagnostic(
                "KRN_COPY_UUID_COLLISION",
                "Copied objects require unique new RFC 9562 UUIDv4 identities.",
                vec![*source, *target],
            ));
        }
    }
    Ok(())
}

fn copy_preview(
    project: &Project,
    roots: &[ObjectId],
    id_map: &BTreeMap<ObjectId, ObjectId>,
    destination_parent: ObjectId,
) -> CopyPreview {
    let closure = containment_closure(project, roots);
    let mut diagnostics = Vec::new();
    let destination = match active_object(project, destination_parent) {
        Ok(destination) => Some(destination),
        Err(diagnostic) => {
            diagnostics.push(diagnostic);
            None
        }
    };
    if let Err(diagnostic) = validate_copy_map(project, &closure, id_map) {
        diagnostics.push(diagnostic);
    }
    if let Some(destination) = destination {
        for root in roots {
            if let Some(source) = project.objects.get(root) {
                if !destination.kind.can_contain(source.kind) {
                    diagnostics.push(diagnostic(
                        "KRN_ILLEGAL_CONTAINMENT",
                        "The destination object kind cannot contain the copied root kind.",
                        vec![destination_parent, *root],
                    ));
                }
                if project.objects.values().any(|candidate| {
                    candidate.lifecycle == Lifecycle::Active
                        && candidate.parent_id == Some(destination_parent)
                        && candidate.kind == source.kind
                        && candidate.display_name == source.display_name
                }) {
                    diagnostics.push(diagnostic(
                        "KRN_COPY_NAME_CONFLICT",
                        "A sibling with the same kind and display name already exists at the destination.",
                        vec![destination_parent, *root],
                    ));
                }
            }
        }
    }
    for source_id in &closure {
        let Some(source) = project.objects.get(source_id) else {
            continue;
        };
        for (field, code, message) in [
            (
                "engineeringNumber",
                "KRN_COPY_NUMBER_CONFLICT",
                "The copy would duplicate an engineering number.",
            ),
            (
                "address",
                "KRN_COPY_ADDRESS_CONFLICT",
                "The copy would duplicate an engineering address.",
            ),
        ] {
            let Some(value) = source.payload.semantic.get(field) else {
                continue;
            };
            if let Some(conflict) = project.objects.values().find(|candidate| {
                candidate.lifecycle == Lifecycle::Active
                    && candidate.payload.semantic.get(field) == Some(value)
            }) {
                diagnostics.push(diagnostic(code, message, vec![*source_id, conflict.id]));
            }
        }
        if source.payload.semantic.contains_key("requiredCapability") {
            diagnostics.push(diagnostic(
                "KRN_COPY_CAPABILITY_PREVIEW_REQUIRED",
                "A higher-layer TrainingProfile capability preview is required before copying this object.",
                vec![*source_id],
            ));
        }
    }
    let mut remapped_internal_references = 0;
    let mut preserved_external_references = 0;
    for edge in project
        .references
        .iter()
        .filter(|edge| closure.contains(&edge.source_id))
    {
        if closure.contains(&edge.target_id) {
            remapped_internal_references += 1;
        } else {
            preserved_external_references += 1;
            if edge.resolution == ResolutionState::Unresolved {
                diagnostics.push(diagnostic(
                    "KRN_COPY_UNRESOLVED_EXTERNAL_DEPENDENCY",
                    "The copied closure has an unresolved dependency outside the closure.",
                    vec![edge.source_id, edge.target_id],
                ));
            }
        }
    }
    let mut ordered: Vec<_> = closure.into_iter().collect();
    ordered.sort_by_key(|id| project.objects[id].creation_ordinal);
    CopyPreview {
        closure: ordered,
        id_map: id_map.clone(),
        remapped_internal_references,
        preserved_external_references,
        can_commit: diagnostics.is_empty(),
        diagnostics,
    }
}

fn containment_closure(project: &Project, roots: &[ObjectId]) -> BTreeSet<ObjectId> {
    let mut closure: BTreeSet<_> = roots
        .iter()
        .copied()
        .filter(|id| {
            project
                .objects
                .get(id)
                .is_some_and(|item| item.lifecycle == Lifecycle::Active)
        })
        .collect();
    loop {
        let before = closure.len();
        for object in project.objects.values() {
            if object.lifecycle == Lifecycle::Active
                && object
                    .parent_id
                    .is_some_and(|parent| closure.contains(&parent))
            {
                closure.insert(object.id);
            }
        }
        if closure.len() == before {
            break;
        }
    }
    closure
}

fn active_object(project: &Project, id: ObjectId) -> Result<&ProjectObject, Diagnostic> {
    project
        .objects
        .get(&id)
        .filter(|object| object.lifecycle == Lifecycle::Active)
        .ok_or_else(|| {
            diagnostic(
                "KRN_UNKNOWN_ACTIVE_OBJECT",
                "The active object does not exist.",
                vec![id],
            )
        })
}

fn active_object_mut(
    project: &mut Project,
    id: ObjectId,
) -> Result<&mut ProjectObject, Diagnostic> {
    project
        .objects
        .get_mut(&id)
        .filter(|object| object.lifecycle == Lifecycle::Active)
        .ok_or_else(|| {
            diagnostic(
                "KRN_UNKNOWN_ACTIVE_OBJECT",
                "The active object does not exist.",
                vec![id],
            )
        })
}

fn bump_by_id(project: &mut Project, id: ObjectId, semantic: bool) -> Result<(), Diagnostic> {
    bump_object(active_object_mut(project, id)?, semantic)
}

fn bump_object(object: &mut ProjectObject, semantic: bool) -> Result<(), Diagnostic> {
    object.object_revision = object.object_revision.checked_add(1).ok_or_else(|| {
        diagnostic(
            "KRN_REVISION_EXHAUSTED",
            "Object revision exhausted.",
            vec![object.id],
        )
    })?;
    if semantic {
        object.semantic_revision = object.semantic_revision.checked_add(1).ok_or_else(|| {
            diagnostic(
                "KRN_REVISION_EXHAUSTED",
                "Object semantic revision exhausted.",
                vec![object.id],
            )
        })?;
    }
    Ok(())
}

fn validate_name(name: &str, id: ObjectId) -> Result<(), Diagnostic> {
    if name.is_empty() || name.len() > 256 || name.chars().any(char::is_control) {
        return Err(diagnostic(
            "KRN_INVALID_DISPLAY_NAME",
            "Display name must contain 1 through 256 non-control UTF-8 bytes.",
            vec![id],
        ));
    }
    Ok(())
}

fn validate_field_key(key: &str, id: ObjectId) -> Result<(), Diagnostic> {
    if key.is_empty()
        || key.len() > 128
        || !key
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
    {
        return Err(diagnostic(
            "KRN_INVALID_FIELD_KEY",
            "Payload field key is outside the closed baseline grammar.",
            vec![id],
        ));
    }
    Ok(())
}

fn resolved_state(project: &Project, edge: &ReferenceEdge) -> ResolutionState {
    if project.objects.get(&edge.target_id).is_some_and(|target| {
        target.lifecycle == Lifecycle::Active && target.kind == edge.expected_target_kind
    }) {
        ResolutionState::Resolved
    } else {
        ResolutionState::Unresolved
    }
}

fn refresh_reference_resolution(project: &mut Project) {
    project.references = project
        .references
        .iter()
        .cloned()
        .map(|mut edge| {
            edge.resolution = resolved_state(project, &edge);
            edge
        })
        .collect();
}

#[derive(Clone, Debug)]
struct ObjectDelta {
    id: ObjectId,
    before: Option<ProjectObject>,
    after: Option<ProjectObject>,
    semantic_changed: bool,
}

#[derive(Clone, Debug)]
struct HistoryEntry {
    transaction_id: TransactionId,
    undo_token: UndoToken,
    object_deltas: Vec<ObjectDelta>,
    references_added: BTreeSet<ReferenceEdge>,
    references_removed: BTreeSet<ReferenceEdge>,
    dependencies_added: BTreeSet<DependencyEdge>,
    dependencies_removed: BTreeSet<DependencyEdge>,
    semantic_changed: bool,
}

impl HistoryEntry {
    fn between(
        transaction_id: TransactionId,
        undo_token: UndoToken,
        before: &Project,
        after: &Project,
    ) -> Self {
        let ids: BTreeSet<_> = before
            .objects
            .keys()
            .chain(after.objects.keys())
            .copied()
            .collect();
        let object_deltas = ids
            .into_iter()
            .filter_map(|id| {
                let left = before.objects.get(&id).cloned();
                let right = after.objects.get(&id).cloned();
                (left != right).then(|| ObjectDelta {
                    id,
                    semantic_changed: semantic_object_state(left.as_ref())
                        != semantic_object_state(right.as_ref()),
                    before: left,
                    after: right,
                })
            })
            .collect();
        Self {
            transaction_id,
            undo_token,
            object_deltas,
            references_added: after
                .references
                .difference(&before.references)
                .cloned()
                .collect(),
            references_removed: before
                .references
                .difference(&after.references)
                .cloned()
                .collect(),
            dependencies_added: after
                .dependencies
                .difference(&before.dependencies)
                .cloned()
                .collect(),
            dependencies_removed: before
                .dependencies
                .difference(&after.dependencies)
                .cloned()
                .collect(),
            semantic_changed: before.semantic_fingerprint() != after.semantic_fingerprint(),
        }
    }

    fn affected_ids(&self) -> Vec<ObjectId> {
        self.object_deltas.iter().map(|delta| delta.id).collect()
    }

    fn matches_after(&self, project: &Project) -> bool {
        self.object_deltas
            .iter()
            .all(|delta| project.objects.get(&delta.id) == delta.after.as_ref())
            && self
                .references_added
                .iter()
                .all(|edge| project.references.contains(edge))
            && self
                .references_removed
                .iter()
                .all(|edge| !project.references.contains(edge))
            && self
                .dependencies_added
                .iter()
                .all(|edge| project.dependencies.contains(edge))
            && self
                .dependencies_removed
                .iter()
                .all(|edge| !project.dependencies.contains(edge))
    }

    fn matches_before_semantics(&self, project: &Project) -> bool {
        self.object_deltas.iter().all(|delta| {
            semantic_object_state(project.objects.get(&delta.id))
                == semantic_object_state(delta.before.as_ref())
        })
    }

    fn undo_events(&self) -> Vec<DomainEvent> {
        self.object_deltas
            .iter()
            .map(|delta| match (&delta.before, &delta.after) {
                (Some(before), Some(after))
                    if before.lifecycle == Lifecycle::Active
                        && after.lifecycle == Lifecycle::Tombstoned =>
                {
                    DomainEvent::Restored(delta.id)
                }
                (None, Some(_)) => DomainEvent::Deleted(delta.id),
                _ => DomainEvent::Changed(delta.id),
            })
            .collect()
    }

    fn redo_events(&self) -> Vec<DomainEvent> {
        self.object_deltas
            .iter()
            .map(|delta| match (&delta.before, &delta.after) {
                (Some(before), Some(after))
                    if before.lifecycle == Lifecycle::Active
                        && after.lifecycle == Lifecycle::Tombstoned =>
                {
                    DomainEvent::Deleted(delta.id)
                }
                (None, Some(_)) => DomainEvent::Created(delta.id),
                _ => DomainEvent::Changed(delta.id),
            })
            .collect()
    }
}

#[derive(Clone, Debug)]
struct RedoEntry {
    history: HistoryEntry,
    expected_hash: Sha256Digest,
}

#[derive(Clone, Copy)]
enum HistoryDirection {
    Undo,
    Redo,
}

fn apply_history_direction(
    project: &mut Project,
    history: &HistoryEntry,
    direction: HistoryDirection,
) -> Result<(), Diagnostic> {
    for delta in &history.object_deltas {
        let desired = match direction {
            HistoryDirection::Undo => delta.before.as_ref(),
            HistoryDirection::Redo => delta.after.as_ref(),
        };
        match desired {
            None => {
                project.objects.remove(&delta.id);
            }
            Some(desired) => {
                let mut restored = desired.clone();
                if let Some(current) = project.objects.get(&delta.id) {
                    restored.object_revision =
                        current.object_revision.checked_add(1).ok_or_else(|| {
                            diagnostic(
                                "KRN_REVISION_EXHAUSTED",
                                "Undo or redo object revision exhausted.",
                                vec![delta.id],
                            )
                        })?;
                    restored.semantic_revision = current
                        .semantic_revision
                        .checked_add(u64::from(delta.semantic_changed))
                        .ok_or_else(|| {
                            diagnostic(
                                "KRN_REVISION_EXHAUSTED",
                                "Undo or redo semantic revision exhausted.",
                                vec![delta.id],
                            )
                        })?;
                } else if matches!(direction, HistoryDirection::Redo) && delta.before.is_none() {
                    restored.object_revision =
                        restored.object_revision.checked_add(1).ok_or_else(|| {
                            diagnostic(
                                "KRN_REVISION_EXHAUSTED",
                                "Redo object revision exhausted.",
                                vec![delta.id],
                            )
                        })?;
                    restored.semantic_revision = restored
                        .semantic_revision
                        .checked_add(u64::from(delta.semantic_changed))
                        .ok_or_else(|| {
                            diagnostic(
                                "KRN_REVISION_EXHAUSTED",
                                "Redo semantic revision exhausted.",
                                vec![delta.id],
                            )
                        })?;
                }
                project.objects.insert(delta.id, restored);
            }
        }
    }
    match direction {
        HistoryDirection::Undo => {
            for edge in &history.references_added {
                project.references.remove(edge);
            }
            project
                .references
                .extend(history.references_removed.iter().cloned());
            for edge in &history.dependencies_added {
                project.dependencies.remove(edge);
            }
            project
                .dependencies
                .extend(history.dependencies_removed.iter().cloned());
        }
        HistoryDirection::Redo => {
            for edge in &history.references_removed {
                project.references.remove(edge);
            }
            project
                .references
                .extend(history.references_added.iter().cloned());
            for edge in &history.dependencies_removed {
                project.dependencies.remove(edge);
            }
            project
                .dependencies
                .extend(history.dependencies_added.iter().cloned());
        }
    }
    refresh_reference_resolution(project);
    project.document_revision = project.document_revision.checked_add(1).ok_or_else(|| {
        diagnostic(
            "KRN_REVISION_EXHAUSTED",
            "Undo or redo document revision exhausted.",
            Vec::new(),
        )
    })?;
    if history.semantic_changed {
        project.semantic_revision = project.semantic_revision.checked_add(1).ok_or_else(|| {
            diagnostic(
                "KRN_REVISION_EXHAUSTED",
                "Undo or redo semantic revision exhausted.",
                Vec::new(),
            )
        })?;
    }
    Ok(())
}

fn semantic_object_state(
    object: Option<&ProjectObject>,
) -> Option<(
    crate::model::ProjectObjectKind,
    Option<ObjectId>,
    &str,
    &str,
    &BTreeMap<String, crate::model::PayloadValue>,
    Lifecycle,
)> {
    object.and_then(|value| {
        if matches!(
            value.kind,
            crate::model::ProjectObjectKind::Project
                | crate::model::ProjectObjectKind::Folder
                | crate::model::ProjectObjectKind::BuildRecord
                | crate::model::ProjectObjectKind::SnapshotReference
        ) {
            return None;
        }
        Some((
            value.kind,
            value
                .kind
                .containment_is_semantic()
                .then_some(value.parent_id)
                .flatten(),
            if value.kind.name_is_semantic() {
                value.display_name.as_str()
            } else {
                ""
            },
            value.payload_schema.as_str(),
            &value.payload.semantic,
            value.lifecycle,
        ))
    })
}

fn trim_history(history: &mut Vec<HistoryEntry>, limit: usize) {
    if history.len() > limit {
        history.drain(..history.len() - limit);
    }
}

fn diagnostic(code: &str, message: &str, object_ids: Vec<ObjectId>) -> Diagnostic {
    Diagnostic {
        code: code.to_owned(),
        message: message.to_owned(),
        object_ids,
    }
}

fn rejected_result(
    transaction_id: TransactionId,
    before_hash: Sha256Digest,
    diagnostic: Diagnostic,
) -> DomainCommandResult {
    DomainCommandResult {
        outcome: CommandOutcome::Rejected,
        transaction_id,
        affected_object_ids: Vec::new(),
        domain_events: Vec::new(),
        diagnostics: vec![diagnostic],
        undo_token: None,
        before_project_hash: before_hash,
        after_project_hash: None,
    }
}

fn blocked_history_result(
    transaction_id: TransactionId,
    before_hash: Sha256Digest,
    code: &str,
    message: &str,
) -> DomainCommandResult {
    DomainCommandResult {
        outcome: CommandOutcome::Blocked,
        transaction_id,
        affected_object_ids: Vec::new(),
        domain_events: Vec::new(),
        diagnostics: vec![diagnostic(code, message, Vec::new())],
        undo_token: None,
        before_project_hash: before_hash,
        after_project_hash: None,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::Engine;
    use crate::hash::sha256;
    use crate::model::{
        CommandContext, CommandEnvelope, CommandOutcome, DomainCommand, NewObject, ObjectId,
        Payload, ProfilePin, Project, ProjectObjectKind, TransactionId, Uuid,
    };

    fn id(n: u64) -> Uuid {
        Uuid::deterministic_v4(b"engine-test", n)
    }

    fn envelope(project: &Project, command: DomainCommand, n: u64) -> CommandEnvelope {
        let mut expected = BTreeMap::new();
        let root = project.root_id();
        expected.insert(root, project.object(root).unwrap().object_revision);
        CommandEnvelope {
            command_id: id(n + 100),
            transaction_id: TransactionId(id(n + 200)),
            expected_document_revision: project.document_revision(),
            expected_object_revisions: expected,
            context: CommandContext {
                actor_id: "test".to_owned(),
                can_mutate: true,
            },
            command,
        }
    }

    fn fixture() -> (Project, ObjectId) {
        let root = ObjectId(id(1));
        (
            Project::new(
                id(2),
                root,
                "Project",
                ProfilePin {
                    id: "EDU-21".to_owned(),
                    version: "1.0".to_owned(),
                    manifest_hash: sha256(b"profile"),
                },
            ),
            root,
        )
    }

    #[test]
    fn presentation_edit_does_not_change_semantic_fingerprint() {
        let (project, root) = fixture();
        let mut engine = Engine::new(project).unwrap();
        let before = engine.project().semantic_fingerprint();
        let object = NewObject {
            id: ObjectId(id(3)),
            kind: ProjectObjectKind::Generic,
            parent_id: root,
            display_name: "Folder".to_owned(),
            payload_schema: "edu.folder/1".to_owned(),
            payload: Payload::default(),
        };
        let create = envelope(engine.project(), DomainCommand::Create(object), 1);
        let result = engine.execute(&create);
        assert_eq!(result.outcome, CommandOutcome::Committed);
        let semantic_after_create = engine.project().semantic_fingerprint();
        assert_ne!(before, semantic_after_create);

        let folder = ObjectId(id(3));
        let command = DomainCommand::SetPresentationField {
            object_id: folder,
            key: "x".to_owned(),
            value: crate::model::PayloadValue::Unsigned(12),
        };
        let mut expected = BTreeMap::new();
        expected.insert(
            folder,
            engine.project().object(folder).unwrap().object_revision,
        );
        let envelope = CommandEnvelope {
            command_id: id(110),
            transaction_id: TransactionId(id(210)),
            expected_document_revision: engine.project().document_revision(),
            expected_object_revisions: expected,
            context: CommandContext {
                actor_id: "test".to_owned(),
                can_mutate: true,
            },
            command,
        };
        let document_before = engine.project().document_hash();
        assert_eq!(engine.execute(&envelope).outcome, CommandOutcome::Committed);
        assert_ne!(document_before, engine.project().document_hash());
        assert_eq!(
            semantic_after_create,
            engine.project().semantic_fingerprint()
        );
    }

    #[test]
    fn organizational_folder_creation_and_rename_are_nonsemantic() {
        let (project, root) = fixture();
        let mut engine = Engine::new(project).expect("engine");
        let baseline = engine.project().semantic_fingerprint();
        let baseline_revision = engine.project().semantic_revision();
        let folder = ObjectId(id(50));
        let create = envelope(
            engine.project(),
            DomainCommand::Create(NewObject {
                id: folder,
                kind: ProjectObjectKind::Folder,
                parent_id: root,
                display_name: "Organization".to_owned(),
                payload_schema: "edu.folder/1".to_owned(),
                payload: Payload::default(),
            }),
            50,
        );
        assert_eq!(engine.execute(&create).outcome, CommandOutcome::Committed);
        assert_eq!(engine.project().semantic_fingerprint(), baseline);
        assert_eq!(engine.project().semantic_revision(), baseline_revision);

        let rename = CommandEnvelope {
            command_id: id(151),
            transaction_id: TransactionId(id(251)),
            expected_document_revision: engine.project().document_revision(),
            expected_object_revisions: BTreeMap::from([(
                folder,
                engine
                    .project()
                    .object(folder)
                    .expect("folder")
                    .object_revision,
            )]),
            context: CommandContext {
                actor_id: "test".to_owned(),
                can_mutate: true,
            },
            command: DomainCommand::Rename {
                object_id: folder,
                display_name: "Organization 2".to_owned(),
            },
        };
        assert_eq!(engine.execute(&rename).outcome, CommandOutcome::Committed);
        assert_eq!(engine.project().semantic_fingerprint(), baseline);
        assert_eq!(engine.project().semantic_revision(), baseline_revision);
    }

    #[test]
    fn stale_or_invalid_commands_leave_no_partial_mutation() {
        let (project, root) = fixture();
        let mut engine = Engine::new(project).expect("engine");
        let first = envelope(
            engine.project(),
            DomainCommand::Rename {
                object_id: root,
                display_name: "First".to_owned(),
            },
            1,
        );
        let stale = CommandEnvelope {
            command_id: id(102),
            transaction_id: TransactionId(id(202)),
            command: DomainCommand::Rename {
                object_id: root,
                display_name: "Stale".to_owned(),
            },
            ..first.clone()
        };
        assert_eq!(engine.execute(&first).outcome, CommandOutcome::Committed);
        let committed_hash = engine.project().document_hash();
        let rejected = engine.execute(&stale);
        assert_eq!(rejected.outcome, CommandOutcome::Rejected);
        assert!(rejected.domain_events.is_empty());
        assert_eq!(engine.project().document_hash(), committed_hash);

        let invalid = envelope(
            engine.project(),
            DomainCommand::Move {
                object_id: root,
                parent_id: root,
            },
            3,
        );
        let rejected = engine.execute(&invalid);
        assert_eq!(rejected.outcome, CommandOutcome::Rejected);
        assert_eq!(engine.project().document_hash(), committed_hash);
    }

    #[test]
    fn undo_redo_are_revisioned_and_intervening_commit_invalidates_redo() {
        let (project, root) = fixture();
        let mut engine = Engine::new(project).expect("engine");
        let rename = envelope(
            engine.project(),
            DomainCommand::Rename {
                object_id: root,
                display_name: "Renamed".to_owned(),
            },
            1,
        );
        let committed = engine.execute(&rename);
        let token = committed.undo_token.expect("undo token");
        let revision_after_commit = engine.project().document_revision();
        let undone = engine.undo(TransactionId(id(300)), token);
        assert_eq!(undone.outcome, CommandOutcome::Committed);
        assert!(engine.project().document_revision() > revision_after_commit);
        assert_eq!(
            engine.project().object(root).expect("root").display_name,
            "Project"
        );
        let redone = engine.redo(TransactionId(id(301)));
        assert_eq!(redone.outcome, CommandOutcome::Committed);
        assert_eq!(
            engine.project().object(root).expect("root").display_name,
            "Renamed"
        );

        let second_token = redone.undo_token.expect("redo undo token");
        assert_eq!(
            engine.undo(TransactionId(id(302)), second_token).outcome,
            CommandOutcome::Committed
        );
        let intervening = envelope(
            engine.project(),
            DomainCommand::Rename {
                object_id: root,
                display_name: "Intervening".to_owned(),
            },
            4,
        );
        assert_eq!(
            engine.execute(&intervening).outcome,
            CommandOutcome::Committed
        );
        let before_blocked_redo = engine.project().document_hash();
        assert_eq!(
            engine.redo(TransactionId(id(303))).outcome,
            CommandOutcome::Blocked
        );
        assert_eq!(engine.project().document_hash(), before_blocked_redo);
    }

    #[test]
    fn deterministic_generated_command_corpus_replays_exactly() {
        let (project, root) = fixture();
        let mut left = Engine::new(project.clone()).expect("left");
        let mut right = Engine::new(project).expect("right");
        let mut object_ids = Vec::new();
        for ordinal in 1..=24_u64 {
            let object_id = ObjectId(Uuid::deterministic_v4(b"generated-object", ordinal));
            object_ids.push(object_id);
            let create = CommandEnvelope {
                command_id: Uuid::deterministic_v4(b"generated-command", ordinal),
                transaction_id: TransactionId(Uuid::deterministic_v4(
                    b"generated-transaction",
                    ordinal,
                )),
                expected_document_revision: left.project().document_revision(),
                expected_object_revisions: BTreeMap::from([(
                    root,
                    left.project().object(root).expect("root").object_revision,
                )]),
                context: CommandContext {
                    actor_id: "generated-corpus".to_owned(),
                    can_mutate: true,
                },
                command: DomainCommand::Create(NewObject {
                    id: object_id,
                    kind: ProjectObjectKind::Generic,
                    parent_id: root,
                    display_name: format!("Object {ordinal}"),
                    payload_schema: "edu.generated/1".to_owned(),
                    payload: Payload::default(),
                }),
            };
            assert_eq!(left.execute(&create), right.execute(&create));
        }
        let mut state = 0x9e37_79b9_u64;
        for ordinal in 25..=96_u64 {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            let index = usize::try_from(
                state % u64::try_from(object_ids.len()).expect("small generated corpus"),
            )
            .expect("index fits usize");
            let target = object_ids[index];
            let command = if state & 1 == 0 {
                DomainCommand::SetPresentationField {
                    object_id: target,
                    key: "layout.x".to_owned(),
                    value: crate::model::PayloadValue::Unsigned(state),
                }
            } else {
                DomainCommand::SetSemanticField {
                    object_id: target,
                    key: "value".to_owned(),
                    value: crate::model::PayloadValue::Unsigned(state),
                }
            };
            let edit = CommandEnvelope {
                command_id: Uuid::deterministic_v4(b"generated-command", ordinal),
                transaction_id: TransactionId(Uuid::deterministic_v4(
                    b"generated-transaction",
                    ordinal,
                )),
                expected_document_revision: left.project().document_revision(),
                expected_object_revisions: BTreeMap::from([(
                    target,
                    left.project()
                        .object(target)
                        .expect("generated target")
                        .object_revision,
                )]),
                context: CommandContext {
                    actor_id: "generated-corpus".to_owned(),
                    can_mutate: true,
                },
                command,
            };
            assert_eq!(left.execute(&edit), right.execute(&edit));
            assert_eq!(left.project().validate(), Ok(()));
            assert_eq!(
                left.project().document_hash(),
                right.project().document_hash()
            );
        }
    }
}
