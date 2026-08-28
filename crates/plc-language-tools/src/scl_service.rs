use alloc::{
    collections::BTreeSet,
    string::{String, ToString},
    vec::Vec,
};

use plc_compiler::{
    LineColumn, ResourceLimits, SclSource, SourceAnchor, TextRange,
    scl::{
        SclAccessKind, SclOccurrenceKind, SclOccurrenceResolution, SclSemanticSnapshot,
        SclSemanticSymbol, SclSymbolOccurrence, TokenKind, analyze_scl, analyze_scl_with_program,
        lex_scl,
    },
};
use plc_program::{
    BlockId, ControllerProgram, DataType, InterfaceMemberId, InterfaceRole, ProgramBlock,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum CompletionKind {
    InterfaceSymbol,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompletionItem {
    pub label: String,
    pub kind: CompletionKind,
    pub member: InterfaceMemberId,
    pub data_type: DataType,
    pub role: InterfaceRole,
    pub replace_range: TextRange,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HoverInfo {
    Symbol {
        definition: SymbolDefinition,
        access: plc_compiler::scl::SclAccessKind,
        source: SourceAnchor,
    },
    ExpressionType {
        data_type: DataType,
        source: SourceAnchor,
    },
    Unresolved {
        spelling: String,
        resolution: SclOccurrenceResolution,
        source: SourceAnchor,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SymbolDefinition {
    pub owner: BlockId,
    pub member: InterfaceMemberId,
    pub name: String,
    pub data_type: DataType,
    pub role: InterfaceRole,
    pub declared_order: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SclNavigationTarget {
    Block(BlockId),
    Member {
        owner: BlockId,
        member: InterfaceMemberId,
    },
    UnresolvedOccurrence {
        owner: BlockId,
        source_revision: plc_compiler::Hash32,
        semantic_node: plc_compiler::SemanticNodeId,
        text_range: Option<TextRange>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SclNavigationRelationship {
    Definition,
    Use,
    Assignment,
    Call,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SclNavigationValidity {
    Valid,
    Unresolved,
    Ambiguous,
}

/// One read-only compiler-index projection for SCL definitions and
/// relationships. Every non-definition entry retains the exact immutable
/// source anchor; unresolved and ambiguous entries retain occurrence identity
/// without inventing a target.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SclNavigationEntry {
    pub target: SclNavigationTarget,
    pub source: Option<SourceAnchor>,
    pub relationship: SclNavigationRelationship,
    pub validity: SclNavigationValidity,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SourceEdit {
    /// Includes owner, content hash, language, stable semantic node, and exact
    /// zero-based UTF-8 byte range. A caller must reject stale hashes.
    pub source: SourceAnchor,
    pub replacement: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InterfaceRename {
    pub owner: BlockId,
    pub member: InterfaceMemberId,
    pub expected_name: String,
    pub replacement: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenamePlan {
    pub declaration: InterfaceRename,
    pub source_edits: Vec<SourceEdit>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RenameError {
    InvalidOffset,
    NoSymbol,
    UnresolvedSymbol,
    AmbiguousSymbol,
    InvalidIdentifier,
    NameCollision(InterfaceMemberId),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SignatureHelp {
    Call {
        target: BlockId,
        name: String,
        formals: Vec<SignatureFormal>,
    },
    NotAtCallable,
    ProgramContextRequired,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SignatureFormal {
    pub member: InterfaceMemberId,
    pub name: String,
    pub role: InterfaceRole,
    pub data_type: DataType,
    pub required: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SemanticToken {
    pub range: TextRange,
    pub kind: SemanticTokenKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SemanticTokenKind {
    Keyword,
    Literal,
    Operator,
    Punctuation,
    Symbol {
        member: InterfaceMemberId,
        role: InterfaceRole,
    },
    UnresolvedSymbol,
    AmbiguousSymbol,
    Identifier,
    Malformed,
}

/// Immutable language-service facade over the compiler-owned SCL semantic
/// snapshot. Every semantic answer comes from `analyze_scl`; token coloring
/// comes from the compiler lexer. This crate contains no parser or binder.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SclLanguageService {
    snapshot: SclSemanticSnapshot,
    limits: ResourceLimits,
    program: Option<ControllerProgram>,
}

impl SclLanguageService {
    #[must_use]
    pub fn analyze(source: &SclSource, block: &ProgramBlock, limits: ResourceLimits) -> Self {
        Self {
            snapshot: analyze_scl(source, block, limits),
            limits,
            program: None,
        }
    }

    #[must_use]
    pub fn analyze_with_program(
        source: &SclSource,
        block: &ProgramBlock,
        program: &ControllerProgram,
        limits: ResourceLimits,
    ) -> Self {
        Self {
            snapshot: analyze_scl_with_program(source, block, program, limits),
            limits,
            program: Some(program.clone()),
        }
    }

    #[must_use]
    pub const fn snapshot(&self) -> &SclSemanticSnapshot {
        &self.snapshot
    }

    #[must_use]
    pub fn diagnostics(&self) -> &[plc_compiler::scl::SclIssue] {
        self.snapshot.diagnostics()
    }

    #[must_use]
    pub fn folding_ranges(&self) -> &[TextRange] {
        self.snapshot.folding_ranges()
    }

    #[must_use]
    pub fn display_position(&self, byte_offset: u32) -> Option<LineColumn> {
        self.snapshot.source().line_column(byte_offset)
    }

    #[must_use]
    pub fn completions(&self, byte_offset: u32) -> Vec<CompletionItem> {
        if self.snapshot.resource_limit().is_some() {
            return Vec::new();
        }
        let Some(replace_range) = identifier_prefix_range(self.snapshot.source(), byte_offset)
        else {
            return Vec::new();
        };
        let Some(prefix) = self.snapshot.source().range_text(replace_range) else {
            return Vec::new();
        };
        let folded_prefix = prefix.to_ascii_lowercase();
        self.snapshot
            .symbols()
            .iter()
            .filter(|symbol| symbol.name.to_ascii_lowercase().starts_with(&folded_prefix))
            .map(|symbol| CompletionItem {
                label: symbol.name.clone(),
                kind: CompletionKind::InterfaceSymbol,
                member: symbol.member,
                data_type: symbol.data_type.clone(),
                role: symbol.role,
                replace_range,
            })
            .collect()
    }

    #[must_use]
    pub fn hover(&self, byte_offset: u32) -> Option<HoverInfo> {
        if let Some(occurrence) = occurrence_at(&self.snapshot, byte_offset) {
            return Some(match occurrence.resolution {
                SclOccurrenceResolution::Resolved => HoverInfo::Symbol {
                    definition: definition_for_occurrence(
                        &self.snapshot,
                        occurrence,
                        self.program.as_ref(),
                    )?,
                    access: occurrence.access,
                    source: occurrence.source.clone(),
                },
                SclOccurrenceResolution::Unresolved | SclOccurrenceResolution::Ambiguous => {
                    HoverInfo::Unresolved {
                        spelling: occurrence.spelling.clone(),
                        resolution: occurrence.resolution,
                        source: occurrence.source.clone(),
                    }
                }
            });
        }
        self.snapshot
            .type_facts()
            .iter()
            .filter(|fact| {
                fact.source
                    .text_range
                    .is_some_and(|range| range_contains(range, byte_offset))
            })
            .min_by_key(|fact| fact.source.text_range.map_or(u32::MAX, TextRange::len))
            .map(|fact| HoverInfo::ExpressionType {
                data_type: fact.data_type.clone(),
                source: fact.source.clone(),
            })
    }

    #[must_use]
    pub fn definition(&self, byte_offset: u32) -> Option<SymbolDefinition> {
        definition_for_occurrence(
            &self.snapshot,
            occurrence_at(&self.snapshot, byte_offset)?,
            self.program.as_ref(),
        )
    }

    #[must_use]
    pub fn references(&self, byte_offset: u32) -> Vec<SourceAnchor> {
        let Some(identity) = occurrence_at(&self.snapshot, byte_offset)
            .filter(|occurrence| occurrence.resolution == SclOccurrenceResolution::Resolved)
            .and_then(occurrence_member_identity)
        else {
            return Vec::new();
        };
        self.snapshot
            .occurrences()
            .iter()
            .filter(|occurrence| occurrence_member_identity(occurrence) == Some(identity))
            .map(|occurrence| occurrence.source.clone())
            .collect()
    }

    /// Returns the deterministic semantic relationships produced by the
    /// compiler binder. This is intentionally not a text-search result.
    #[must_use]
    pub fn navigation_entries(&self) -> Vec<SclNavigationEntry> {
        let mut entries = Vec::new();
        let mut definitions = BTreeSet::new();
        for symbol in self.snapshot.symbols() {
            definitions.insert(SclNavigationTarget::Member {
                owner: symbol.owner,
                member: symbol.member,
            });
        }
        for occurrence in self.snapshot.occurrences() {
            let target = navigation_target(occurrence);
            if occurrence.resolution == SclOccurrenceResolution::Resolved
                && !matches!(target, SclNavigationTarget::UnresolvedOccurrence { .. })
            {
                definitions.insert(target);
            }
            entries.push(SclNavigationEntry {
                target,
                source: Some(occurrence.source.clone()),
                relationship: navigation_relationship(occurrence),
                validity: navigation_validity(occurrence.resolution),
            });
        }
        entries.extend(definitions.into_iter().map(|target| SclNavigationEntry {
            target,
            source: None,
            relationship: SclNavigationRelationship::Definition,
            validity: SclNavigationValidity::Valid,
        }));
        entries.sort();
        entries.dedup();
        entries
    }

    pub fn rename(&self, byte_offset: u32, replacement: &str) -> Result<RenamePlan, RenameError> {
        if !valid_source_offset(self.snapshot.source(), byte_offset) {
            return Err(RenameError::InvalidOffset);
        }
        let occurrence = occurrence_at(&self.snapshot, byte_offset).ok_or(RenameError::NoSymbol)?;
        let member = match occurrence.resolution {
            SclOccurrenceResolution::Resolved => occurrence.member.ok_or(RenameError::NoSymbol)?,
            SclOccurrenceResolution::Unresolved => return Err(RenameError::UnresolvedSymbol),
            SclOccurrenceResolution::Ambiguous => return Err(RenameError::AmbiguousSymbol),
        };
        if !self.valid_identifier(replacement) {
            return Err(RenameError::InvalidIdentifier);
        }
        let definition =
            definition_for_occurrence(&self.snapshot, occurrence, self.program.as_ref())
                .ok_or(RenameError::NoSymbol)?;
        let collision = if definition.owner == self.snapshot.source().owner() {
            self.snapshot
                .symbols()
                .iter()
                .find(|candidate| {
                    candidate.member != member && candidate.name.eq_ignore_ascii_case(replacement)
                })
                .map(|candidate| candidate.member)
        } else {
            self.program
                .as_ref()
                .and_then(|program| program.block(definition.owner))
                .and_then(|block| {
                    block.interface.members.values().find(|candidate| {
                        candidate.id != member && candidate.name.eq_ignore_ascii_case(replacement)
                    })
                })
                .map(|candidate| candidate.id)
        };
        if let Some(collision) = collision {
            return Err(RenameError::NameCollision(collision));
        }
        let identity = (definition.owner, member);
        let source_edits = self
            .snapshot
            .occurrences()
            .iter()
            .filter(|candidate| occurrence_member_identity(candidate) == Some(identity))
            .map(|candidate| SourceEdit {
                source: candidate.source.clone(),
                replacement: replacement.to_string(),
            })
            .collect();
        Ok(RenamePlan {
            declaration: InterfaceRename {
                owner: definition.owner,
                member,
                expected_name: definition.name.clone(),
                replacement: replacement.to_string(),
            },
            source_edits,
        })
    }

    #[must_use]
    pub fn signature_help(&self, byte_offset: u32) -> SignatureHelp {
        let Some(program) = &self.program else {
            return SignatureHelp::ProgramContextRequired;
        };
        let Some(target) = occurrence_at(&self.snapshot, byte_offset)
            .filter(|occurrence| {
                matches!(
                    occurrence.kind,
                    SclOccurrenceKind::CallTarget | SclOccurrenceKind::CallFormal
                )
            })
            .and_then(|occurrence| occurrence.definition_owner)
            .and_then(|owner| program.block(owner))
        else {
            return SignatureHelp::NotAtCallable;
        };
        let formals = target
            .interface
            .ordered_member_ids
            .iter()
            .filter_map(|id| target.interface.member(*id))
            .filter(|member| {
                matches!(
                    member.role,
                    InterfaceRole::Input
                        | InterfaceRole::Output
                        | InterfaceRole::InOut
                        | InterfaceRole::Return
                )
            })
            .map(|member| SignatureFormal {
                member: member.id,
                name: member.name.clone(),
                role: member.role,
                data_type: member.data_type.clone(),
                required: match member.role {
                    InterfaceRole::Input => member.default_value.is_none(),
                    InterfaceRole::InOut => true,
                    InterfaceRole::Output | InterfaceRole::Return => member.required_output_binding,
                    InterfaceRole::Static | InterfaceRole::Temp | InterfaceRole::Constant => false,
                },
            })
            .collect();
        SignatureHelp::Call {
            target: target.id,
            name: target.display_name.clone(),
            formals,
        }
    }

    #[must_use]
    pub fn semantic_tokens(&self) -> Vec<SemanticToken> {
        let lexed = lex_scl(self.snapshot.source(), self.limits);
        lexed
            .tokens()
            .iter()
            .filter(|token| token.kind != TokenKind::Eof)
            .map(|token| SemanticToken {
                range: token.range,
                kind: self.semantic_token_kind(token.kind, token.range),
            })
            .collect()
    }

    fn semantic_token_kind(&self, token: TokenKind, range: TextRange) -> SemanticTokenKind {
        if token == TokenKind::Identifier {
            return self
                .snapshot
                .occurrences()
                .iter()
                .find(|occurrence| occurrence.source.text_range == Some(range))
                .map_or(
                    SemanticTokenKind::Identifier,
                    |occurrence| match occurrence.resolution {
                        SclOccurrenceResolution::Resolved => {
                            match (occurrence.member, occurrence.role) {
                                (Some(member), Some(role)) => {
                                    SemanticTokenKind::Symbol { member, role }
                                }
                                _ => SemanticTokenKind::Identifier,
                            }
                        }
                        SclOccurrenceResolution::Unresolved => SemanticTokenKind::UnresolvedSymbol,
                        SclOccurrenceResolution::Ambiguous => SemanticTokenKind::AmbiguousSymbol,
                    },
                );
        }
        token_class(token)
    }

    fn valid_identifier(&self, candidate: &str) -> bool {
        if candidate.is_empty() {
            return false;
        }
        let source = SclSource::new(self.snapshot.source().owner(), candidate);
        let lexed = lex_scl(&source, self.limits);
        lexed.issues().is_empty()
            && lexed.resource_limit().is_none()
            && lexed.tokens().len() == 2
            && lexed.tokens()[0].kind == TokenKind::Identifier
            && lexed.tokens()[0].range
                == TextRange {
                    start: 0,
                    end: u32::try_from(candidate.len()).unwrap_or(u32::MAX),
                }
            && lexed.tokens()[1].kind == TokenKind::Eof
    }
}

fn occurrence_at(snapshot: &SclSemanticSnapshot, byte_offset: u32) -> Option<&SclSymbolOccurrence> {
    snapshot
        .occurrences()
        .iter()
        .filter(|occurrence| {
            occurrence
                .source
                .text_range
                .is_some_and(|range| range_contains(range, byte_offset))
        })
        .min_by_key(|occurrence| {
            occurrence
                .source
                .text_range
                .map_or(u32::MAX, TextRange::len)
        })
}

fn definition_for_occurrence(
    snapshot: &SclSemanticSnapshot,
    occurrence: &SclSymbolOccurrence,
    program: Option<&ControllerProgram>,
) -> Option<SymbolDefinition> {
    let member = occurrence.member?;
    let owner = occurrence.definition_owner?;
    if owner == snapshot.source().owner() {
        return snapshot
            .symbols()
            .iter()
            .find(|symbol| symbol.owner == owner && symbol.member == member)
            .map(symbol_definition);
    }
    let member = program?.block(owner)?.interface.member(member)?;
    Some(SymbolDefinition {
        owner,
        member: member.id,
        name: member.name.clone(),
        data_type: member.data_type.clone(),
        role: member.role,
        declared_order: member.declared_order,
    })
}

fn occurrence_member_identity(
    occurrence: &SclSymbolOccurrence,
) -> Option<(BlockId, InterfaceMemberId)> {
    Some((occurrence.definition_owner?, occurrence.member?))
}

fn navigation_target(occurrence: &SclSymbolOccurrence) -> SclNavigationTarget {
    match (
        occurrence.resolution,
        occurrence.kind,
        occurrence.definition_owner,
        occurrence.member,
    ) {
        (SclOccurrenceResolution::Resolved, SclOccurrenceKind::CallTarget, Some(owner), None) => {
            SclNavigationTarget::Block(owner)
        }
        (SclOccurrenceResolution::Resolved, _, Some(owner), Some(member)) => {
            SclNavigationTarget::Member { owner, member }
        }
        _ => SclNavigationTarget::UnresolvedOccurrence {
            owner: occurrence.source.owner_object_id,
            source_revision: occurrence.source.source_revision_hash,
            semantic_node: occurrence.source.semantic_node_id,
            text_range: occurrence.source.text_range,
        },
    }
}

const fn navigation_relationship(occurrence: &SclSymbolOccurrence) -> SclNavigationRelationship {
    match (occurrence.kind, occurrence.access) {
        (SclOccurrenceKind::CallTarget, _) => SclNavigationRelationship::Call,
        (SclOccurrenceKind::MemberReference, SclAccessKind::Write) => {
            SclNavigationRelationship::Assignment
        }
        (SclOccurrenceKind::MemberReference | SclOccurrenceKind::CallFormal, _) => {
            SclNavigationRelationship::Use
        }
    }
}

const fn navigation_validity(resolution: SclOccurrenceResolution) -> SclNavigationValidity {
    match resolution {
        SclOccurrenceResolution::Resolved => SclNavigationValidity::Valid,
        SclOccurrenceResolution::Unresolved => SclNavigationValidity::Unresolved,
        SclOccurrenceResolution::Ambiguous => SclNavigationValidity::Ambiguous,
    }
}

fn symbol_definition(symbol: &SclSemanticSymbol) -> SymbolDefinition {
    SymbolDefinition {
        owner: symbol.owner,
        member: symbol.member,
        name: symbol.name.clone(),
        data_type: symbol.data_type.clone(),
        role: symbol.role,
        declared_order: symbol.declared_order,
    }
}

fn identifier_prefix_range(source: &SclSource, byte_offset: u32) -> Option<TextRange> {
    if !valid_source_offset(source, byte_offset) {
        return None;
    }
    let end = usize::try_from(byte_offset).ok()?;
    let bytes = source.text().as_bytes();
    let mut start = end;
    while start > 0 && is_identifier_continue(bytes[start - 1]) {
        start -= 1;
    }
    Some(TextRange {
        start: u32::try_from(start).ok()?,
        end: byte_offset,
    })
}

fn valid_source_offset(source: &SclSource, byte_offset: u32) -> bool {
    usize::try_from(byte_offset).ok().is_some_and(|offset| {
        offset <= source.text().len() && source.text().is_char_boundary(offset)
    })
}

const fn is_identifier_continue(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

const fn range_contains(range: TextRange, offset: u32) -> bool {
    range.start <= offset && offset < range.end
}

const fn token_class(token: TokenKind) -> SemanticTokenKind {
    match token {
        TokenKind::If
        | TokenKind::Then
        | TokenKind::Elsif
        | TokenKind::Else
        | TokenKind::EndIf
        | TokenKind::Return
        | TokenKind::And
        | TokenKind::Xor
        | TokenKind::Or
        | TokenKind::Not
        | TokenKind::Mod
        | TokenKind::UnsupportedKeyword => SemanticTokenKind::Keyword,
        TokenKind::IntegerLiteral
        | TokenKind::RealLiteral
        | TokenKind::QuotedLiteral
        | TokenKind::TimeLiteral
        | TokenKind::TypedLiteral
        | TokenKind::True
        | TokenKind::False => SemanticTokenKind::Literal,
        TokenKind::Assign
        | TokenKind::BindOutput
        | TokenKind::Equal
        | TokenKind::NotEqual
        | TokenKind::Less
        | TokenKind::LessEqual
        | TokenKind::Greater
        | TokenKind::GreaterEqual
        | TokenKind::Plus
        | TokenKind::Minus
        | TokenKind::Star
        | TokenKind::Slash => SemanticTokenKind::Operator,
        TokenKind::LeftParen
        | TokenKind::RightParen
        | TokenKind::LeftBracket
        | TokenKind::RightBracket
        | TokenKind::Comma
        | TokenKind::Dot
        | TokenKind::DotDot
        | TokenKind::Colon
        | TokenKind::Semicolon
        | TokenKind::Eof => SemanticTokenKind::Punctuation,
        TokenKind::Identifier => SemanticTokenKind::Identifier,
        TokenKind::Malformed => SemanticTokenKind::Malformed,
    }
}
