use alloc::{
    collections::{BTreeMap, BTreeSet},
    string::{String, ToString},
    vec::Vec,
};

use plc_program::{
    BlockId, CALL_FC, CanonicalValue, ControllerProgram, DataType, InterfaceMember,
    InterfaceMemberId, InterfaceRole, ProgramBlock, ProgramUnitKind,
};

use crate::{
    DiagnosticCode, ResourceLimit, ResourceLimits, SclSource, SemanticNodeId, SourceAnchor,
    TextRange,
};

use super::{
    BinaryOp, CallActual, CallArgument, Expr, ExprKind, Literal, MissingToken, ParsedOnlyStatement,
    SclIssue, Statement, StatementKind, SyntaxTree, UnaryOp, parse_scl,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SclAccessKind {
    Read,
    Write,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SclOccurrenceResolution {
    Resolved,
    Unresolved,
    Ambiguous,
}

/// Compiler-owned semantic role for a textual identifier occurrence.
/// Consumers must use this role instead of inferring calls or assignments from
/// spelling or punctuation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SclOccurrenceKind {
    MemberReference,
    CallTarget,
    CallFormal,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SclSemanticSymbol {
    pub owner: BlockId,
    pub member: InterfaceMemberId,
    pub name: String,
    pub data_type: DataType,
    pub role: InterfaceRole,
    pub declared_order: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SclSymbolOccurrence {
    pub source: SourceAnchor,
    pub spelling: String,
    pub kind: SclOccurrenceKind,
    pub access: SclAccessKind,
    pub resolution: SclOccurrenceResolution,
    pub member: Option<InterfaceMemberId>,
    pub data_type: Option<DataType>,
    pub role: Option<InterfaceRole>,
    pub definition_owner: Option<BlockId>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SclTypeFact {
    pub source: SourceAnchor,
    pub data_type: DataType,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SclSemanticSnapshot {
    source: SclSource,
    symbols: Vec<SclSemanticSymbol>,
    occurrences: Vec<SclSymbolOccurrence>,
    type_facts: Vec<SclTypeFact>,
    diagnostics: Vec<SclIssue>,
    missing_tokens: Vec<MissingToken>,
    folding_ranges: Vec<TextRange>,
    resource_limit: Option<ResourceLimit>,
}

impl SclSemanticSnapshot {
    #[must_use]
    pub const fn source(&self) -> &SclSource {
        &self.source
    }

    #[must_use]
    pub fn symbols(&self) -> &[SclSemanticSymbol] {
        &self.symbols
    }

    #[must_use]
    pub fn occurrences(&self) -> &[SclSymbolOccurrence] {
        &self.occurrences
    }

    #[must_use]
    pub fn type_facts(&self) -> &[SclTypeFact] {
        &self.type_facts
    }

    #[must_use]
    pub fn diagnostics(&self) -> &[SclIssue] {
        &self.diagnostics
    }

    #[must_use]
    pub fn missing_tokens(&self) -> &[MissingToken] {
        &self.missing_tokens
    }

    #[must_use]
    pub fn folding_ranges(&self) -> &[TextRange] {
        &self.folding_ranges
    }

    #[must_use]
    pub const fn resource_limit(&self) -> Option<&ResourceLimit> {
        self.resource_limit.as_ref()
    }
}

#[must_use]
pub fn analyze_scl(
    source: &SclSource,
    block: &ProgramBlock,
    limits: ResourceLimits,
) -> SclSemanticSnapshot {
    analyze_scl_inner(source, block, None, limits)
}

#[must_use]
pub fn analyze_scl_with_program(
    source: &SclSource,
    block: &ProgramBlock,
    program: &ControllerProgram,
    limits: ResourceLimits,
) -> SclSemanticSnapshot {
    analyze_scl_inner(source, block, Some(program), limits)
}

fn analyze_scl_inner(
    source: &SclSource,
    block: &ProgramBlock,
    program: Option<&ControllerProgram>,
    limits: ResourceLimits,
) -> SclSemanticSnapshot {
    let tree = parse_scl(source, limits);
    let symbols = semantic_symbols(block);
    let mut diagnostics = tree.issues().to_vec();
    let mut folding_ranges = Vec::new();
    collect_folding_ranges(&tree.statements, &mut folding_ranges);
    folding_ranges.sort_unstable();
    folding_ranges.dedup();
    let missing_tokens = tree.missing_tokens().to_vec();
    let resource_limit = tree.resource_limit().cloned();
    if resource_limit.is_some() {
        return SclSemanticSnapshot {
            source: source.clone(),
            symbols,
            occurrences: Vec::new(),
            type_facts: Vec::new(),
            diagnostics,
            missing_tokens,
            folding_ranges,
            resource_limit,
        };
    }
    let (typed, semantic_diagnostics, mut occurrences) =
        bind_and_typecheck_with_occurrences(&tree, block, program);
    diagnostics.extend(semantic_diagnostics);
    sort_issues(&mut diagnostics);
    occurrences.sort_by(|left, right| {
        (
            left.source.text_range,
            left.source.semantic_node_id,
            left.access,
            &left.spelling,
        )
            .cmp(&(
                right.source.text_range,
                right.source.semantic_node_id,
                right.access,
                &right.spelling,
            ))
    });
    occurrences.dedup();
    let mut type_facts = Vec::new();
    collect_type_facts(&typed.statements, source, &mut type_facts);
    type_facts.sort_by_key(|fact| (fact.source.text_range, fact.source.semantic_node_id));
    type_facts.dedup();
    SclSemanticSnapshot {
        source: source.clone(),
        symbols,
        occurrences,
        type_facts,
        diagnostics,
        missing_tokens,
        folding_ranges,
        resource_limit,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TypedBlock {
    pub owner: BlockId,
    pub kind: ProgramUnitKind,
    pub statements: Vec<TypedStatement>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TypedStatement {
    pub id: SemanticNodeId,
    pub range: TextRange,
    pub kind: TypedStatementKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum TypedStatementKind {
    Assignment {
        target: TypedMember,
        value: TypedExpr,
    },
    If {
        branches: Vec<(TypedExpr, Vec<TypedStatement>)>,
        else_body: Vec<TypedStatement>,
    },
    Case {
        selector: TypedExpr,
        arms: Vec<TypedCaseArm>,
        else_body: Vec<TypedStatement>,
    },
    For {
        iterator: TypedMember,
        initial: alloc::boxed::Box<TypedExpr>,
        limit: alloc::boxed::Box<TypedExpr>,
        step: alloc::boxed::Box<TypedExpr>,
        body: Vec<TypedStatement>,
    },
    While {
        condition: TypedExpr,
        body: Vec<TypedStatement>,
    },
    Repeat {
        body: Vec<TypedStatement>,
        condition: TypedExpr,
    },
    Exit,
    Continue,
    Call(TypedCall),
    Return,
    Error,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TypedCaseArm {
    pub range: TextRange,
    pub labels: Vec<TypedCaseLabel>,
    pub body: Vec<TypedStatement>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TypedCaseLabel {
    pub range: TextRange,
    pub lower: TypedExpr,
    pub upper: Option<TypedExpr>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TypedCall {
    pub instruction: plc_program::InstructionCode,
    pub target: BlockId,
    pub inputs: Vec<TypedCallInput>,
    pub outputs: Vec<TypedCallOutput>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TypedCallInput {
    pub formal: InterfaceMemberId,
    pub value: TypedExpr,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TypedCallOutput {
    pub formal: InterfaceMemberId,
    pub target: TypedMember,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TypedMember {
    pub id: InterfaceMemberId,
    pub data_type: DataType,
    pub role: InterfaceRole,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TypedExpr {
    pub id: SemanticNodeId,
    pub range: TextRange,
    pub data_type: DataType,
    pub kind: TypedExprKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum TypedExprKind {
    Constant(CanonicalValue),
    Member(TypedMember),
    Unary {
        operator: UnaryOp,
        operand: alloc::boxed::Box<TypedExpr>,
    },
    Binary {
        operator: BinaryOp,
        left: alloc::boxed::Box<TypedExpr>,
        right: alloc::boxed::Box<TypedExpr>,
    },
    Convert {
        source: alloc::boxed::Box<TypedExpr>,
    },
    Error,
}

#[derive(Clone, Debug)]
struct BoundBlock {
    owner: BlockId,
    kind: ProgramUnitKind,
    statements: Vec<BoundStatement>,
}

#[derive(Clone, Debug)]
struct BoundStatement {
    id: SemanticNodeId,
    range: TextRange,
    kind: BoundStatementKind,
}

#[derive(Clone, Debug)]
enum BoundStatementKind {
    Assignment {
        target: Option<TypedMember>,
        value: BoundExpr,
    },
    If {
        branches: Vec<(BoundExpr, Vec<BoundStatement>)>,
        else_body: Vec<BoundStatement>,
    },
    Case {
        selector: BoundExpr,
        arms: Vec<BoundCaseArm>,
        else_body: Vec<BoundStatement>,
    },
    For {
        iterator: Option<TypedMember>,
        initial: alloc::boxed::Box<BoundExpr>,
        limit: alloc::boxed::Box<BoundExpr>,
        step: Option<alloc::boxed::Box<BoundExpr>>,
        body: Vec<BoundStatement>,
    },
    While {
        condition: BoundExpr,
        body: Vec<BoundStatement>,
    },
    Repeat {
        body: Vec<BoundStatement>,
        condition: BoundExpr,
    },
    Exit,
    Continue,
    Call(BoundCall),
    Return,
    Error,
}

#[derive(Clone, Debug)]
struct BoundCaseArm {
    range: TextRange,
    labels: Vec<BoundCaseLabel>,
    body: Vec<BoundStatement>,
}

#[derive(Clone, Debug)]
struct BoundCaseLabel {
    range: TextRange,
    lower: BoundExpr,
    upper: Option<BoundExpr>,
}

#[derive(Clone, Debug)]
struct BoundCall {
    target: Option<ProgramBlock>,
    arguments: Vec<BoundCallArgument>,
}

#[derive(Clone, Debug)]
struct BoundCallArgument {
    range: TextRange,
    formal: Option<TypedMember>,
    actual: BoundCallActual,
}

#[derive(Clone, Debug)]
enum BoundCallActual {
    Input(BoundExpr),
    Output(Option<TypedMember>),
}

#[derive(Clone, Debug)]
struct BoundExpr {
    id: SemanticNodeId,
    range: TextRange,
    kind: BoundExprKind,
}

#[derive(Clone, Debug)]
enum BoundExprKind {
    Literal(Literal),
    Member {
        member: TypedMember,
        constant_value: Option<CanonicalValue>,
    },
    Unary {
        operator: UnaryOp,
        operand: alloc::boxed::Box<BoundExpr>,
    },
    Binary {
        operator: BinaryOp,
        left: alloc::boxed::Box<BoundExpr>,
        right: alloc::boxed::Box<BoundExpr>,
    },
    Error,
}

#[allow(dead_code)]
pub(crate) fn bind_and_typecheck(
    tree: &SyntaxTree,
    block: &ProgramBlock,
) -> (TypedBlock, Vec<SclIssue>) {
    let (typed, issues, _) = bind_and_typecheck_with_occurrences(tree, block, None);
    (typed, issues)
}

pub(crate) fn bind_and_typecheck_with_program(
    tree: &SyntaxTree,
    block: &ProgramBlock,
    program: &ControllerProgram,
) -> (TypedBlock, Vec<SclIssue>) {
    let (typed, issues, _) = bind_and_typecheck_with_occurrences(tree, block, Some(program));
    (typed, issues)
}

fn bind_and_typecheck_with_occurrences(
    tree: &SyntaxTree,
    block: &ProgramBlock,
    program: Option<&ControllerProgram>,
) -> (TypedBlock, Vec<SclIssue>, Vec<SclSymbolOccurrence>) {
    let (bound, mut issues, occurrences) = bind(tree, block, program);
    let (typed, type_issues) = typecheck(&bound, block);
    issues.extend(type_issues);
    sort_issues(&mut issues);
    (typed, issues, occurrences)
}

fn bind(
    tree: &SyntaxTree,
    block: &ProgramBlock,
    program: Option<&ControllerProgram>,
) -> (BoundBlock, Vec<SclIssue>, Vec<SclSymbolOccurrence>) {
    let mut names = BTreeMap::<String, Vec<&InterfaceMember>>::new();
    for member in block.interface.members.values() {
        names
            .entry(member.name.to_ascii_lowercase())
            .or_default()
            .push(member);
    }
    let mut binder = Binder {
        names,
        constant_values: block
            .interface
            .members
            .values()
            .filter_map(|member| {
                member
                    .constant_value
                    .clone()
                    .map(|value| (member.id, value))
            })
            .collect(),
        issues: Vec::new(),
        occurrences: Vec::new(),
        source_owner: tree.source().owner(),
        source_revision: tree.source().revision_hash(),
        caller: block.id,
        program,
    };
    let statements = binder.bind_statements(&tree.statements);
    (
        BoundBlock {
            owner: block.id,
            kind: block.kind,
            statements,
        },
        binder.issues,
        binder.occurrences,
    )
}

struct Binder<'a> {
    names: BTreeMap<String, Vec<&'a InterfaceMember>>,
    constant_values: BTreeMap<InterfaceMemberId, CanonicalValue>,
    issues: Vec<SclIssue>,
    occurrences: Vec<SclSymbolOccurrence>,
    source_owner: BlockId,
    source_revision: crate::Hash32,
    caller: BlockId,
    program: Option<&'a ControllerProgram>,
}

impl Binder<'_> {
    fn bind_statements(&mut self, statements: &[Statement]) -> Vec<BoundStatement> {
        statements
            .iter()
            .map(|statement| self.bind_statement(statement))
            .collect()
    }

    fn bind_statement(&mut self, statement: &Statement) -> BoundStatement {
        let kind = match &statement.parsed_only {
            Some(ParsedOnlyStatement::Case {
                selector,
                arms,
                else_body,
            }) => BoundStatementKind::Case {
                selector: self.bind_expr(selector),
                arms: arms
                    .iter()
                    .map(|arm| BoundCaseArm {
                        range: arm.range,
                        labels: arm
                            .labels
                            .iter()
                            .map(|label| BoundCaseLabel {
                                range: label.range,
                                lower: self.bind_expr(&label.lower),
                                upper: label.upper.as_ref().map(|value| self.bind_expr(value)),
                            })
                            .collect(),
                        body: self.bind_statements(&arm.body),
                    })
                    .collect(),
                else_body: self.bind_statements(else_body),
            },
            Some(ParsedOnlyStatement::For {
                iterator,
                initial,
                limit,
                step,
                body,
            }) => BoundStatementKind::For {
                iterator: self.resolve(
                    &iterator.spelling,
                    iterator.range,
                    statement.id,
                    SclAccessKind::Write,
                ),
                initial: alloc::boxed::Box::new(self.bind_expr(initial)),
                limit: alloc::boxed::Box::new(self.bind_expr(limit)),
                step: step
                    .as_ref()
                    .map(|value| alloc::boxed::Box::new(self.bind_expr(value))),
                body: self.bind_statements(body),
            },
            Some(ParsedOnlyStatement::While { condition, body }) => BoundStatementKind::While {
                condition: self.bind_expr(condition),
                body: self.bind_statements(body),
            },
            Some(ParsedOnlyStatement::Repeat { body, condition }) => BoundStatementKind::Repeat {
                body: self.bind_statements(body),
                condition: self.bind_expr(condition),
            },
            Some(ParsedOnlyStatement::Exit) => BoundStatementKind::Exit,
            Some(ParsedOnlyStatement::Continue) => BoundStatementKind::Continue,
            Some(
                ParsedOnlyStatement::Assignment { .. }
                | ParsedOnlyStatement::Call { .. }
                | ParsedOnlyStatement::UnsupportedDeclaration { .. },
            ) => BoundStatementKind::Error,
            None => match &statement.kind {
                StatementKind::Assignment { target, value } => BoundStatementKind::Assignment {
                    target: self.resolve(
                        &target.spelling,
                        target.range,
                        statement.id,
                        SclAccessKind::Write,
                    ),
                    value: self.bind_expr(value),
                },
                StatementKind::If {
                    branches,
                    else_body,
                } => BoundStatementKind::If {
                    branches: branches
                        .iter()
                        .map(|(condition, body)| {
                            (self.bind_expr(condition), self.bind_statements(body))
                        })
                        .collect(),
                    else_body: self.bind_statements(else_body),
                },
                StatementKind::Call { callee, arguments } => {
                    BoundStatementKind::Call(self.bind_call(callee, arguments, statement.id))
                }
                StatementKind::Return => BoundStatementKind::Return,
                StatementKind::Error => BoundStatementKind::Error,
            },
        };
        BoundStatement {
            id: statement.id,
            range: statement.range,
            kind,
        }
    }

    fn bind_call(
        &mut self,
        callee: &super::parser::Name,
        arguments: &[CallArgument],
        node: SemanticNodeId,
    ) -> BoundCall {
        let candidates: Vec<_> = self.program.map_or_else(Vec::new, |program| {
            program
                .blocks()
                .values()
                .filter(|block| block.display_name.eq_ignore_ascii_case(&callee.spelling))
                .cloned()
                .collect()
        });
        let target = match candidates.as_slice() {
            [target] => {
                self.external_occurrence(
                    &callee.spelling,
                    callee.range,
                    node,
                    SclOccurrenceKind::CallTarget,
                    SclAccessKind::Read,
                    SclOccurrenceResolution::Resolved,
                    Some(target.id),
                    None,
                );
                Some(target.clone())
            }
            [] => {
                self.external_occurrence(
                    &callee.spelling,
                    callee.range,
                    node,
                    SclOccurrenceKind::CallTarget,
                    SclAccessKind::Read,
                    SclOccurrenceResolution::Unresolved,
                    None,
                    None,
                );
                self.issues.push(SclIssue {
                    code: DiagnosticCode::UNRESOLVED_REFERENCE,
                    range: callee.range,
                    semantic_node: Some(node),
                    cause: alloc::format!("unresolved callable block '{}'", callee.spelling),
                });
                None
            }
            _ => {
                self.external_occurrence(
                    &callee.spelling,
                    callee.range,
                    node,
                    SclOccurrenceKind::CallTarget,
                    SclAccessKind::Read,
                    SclOccurrenceResolution::Ambiguous,
                    None,
                    None,
                );
                self.issues.push(SclIssue {
                    code: DiagnosticCode::AMBIGUOUS_REFERENCE,
                    range: callee.range,
                    semantic_node: Some(node),
                    cause: alloc::format!(
                        "case-insensitive callable block '{}' has multiple candidates",
                        callee.spelling
                    ),
                });
                None
            }
        };
        let bound_arguments = arguments
            .iter()
            .map(|argument| self.bind_call_argument(argument, target.as_ref(), node))
            .collect();
        BoundCall {
            target,
            arguments: bound_arguments,
        }
    }

    fn bind_call_argument(
        &mut self,
        argument: &CallArgument,
        target: Option<&ProgramBlock>,
        node: SemanticNodeId,
    ) -> BoundCallArgument {
        let formal = target.and_then(|target| {
            let candidates: Vec<_> = target
                .interface
                .members
                .values()
                .filter(|member| member.name.eq_ignore_ascii_case(&argument.formal.spelling))
                .collect();
            match candidates.as_slice() {
                [member] => {
                    let typed = TypedMember {
                        id: member.id,
                        data_type: member.data_type.clone(),
                        role: member.role,
                    };
                    self.external_occurrence(
                        &argument.formal.spelling,
                        argument.formal.range,
                        node,
                        SclOccurrenceKind::CallFormal,
                        SclAccessKind::Read,
                        SclOccurrenceResolution::Resolved,
                        Some(target.id),
                        Some(&typed),
                    );
                    Some(typed)
                }
                [] => {
                    self.external_occurrence(
                        &argument.formal.spelling,
                        argument.formal.range,
                        node,
                        SclOccurrenceKind::CallFormal,
                        SclAccessKind::Read,
                        SclOccurrenceResolution::Unresolved,
                        Some(target.id),
                        None,
                    );
                    self.issues.push(SclIssue {
                        code: DiagnosticCode::UNRESOLVED_REFERENCE,
                        range: argument.formal.range,
                        semantic_node: Some(node),
                        cause: alloc::format!(
                            "call target has no formal '{}'",
                            argument.formal.spelling
                        ),
                    });
                    None
                }
                _ => {
                    self.external_occurrence(
                        &argument.formal.spelling,
                        argument.formal.range,
                        node,
                        SclOccurrenceKind::CallFormal,
                        SclAccessKind::Read,
                        SclOccurrenceResolution::Ambiguous,
                        Some(target.id),
                        None,
                    );
                    self.issues.push(SclIssue {
                        code: DiagnosticCode::AMBIGUOUS_REFERENCE,
                        range: argument.formal.range,
                        semantic_node: Some(node),
                        cause: alloc::format!(
                            "call formal '{}' is ambiguous",
                            argument.formal.spelling
                        ),
                    });
                    None
                }
            }
        });
        let actual = match &argument.actual {
            CallActual::Input(expression) => BoundCallActual::Input(self.bind_expr(expression)),
            CallActual::Output(name) => BoundCallActual::Output(self.resolve(
                &name.spelling,
                name.range,
                node,
                SclAccessKind::Write,
            )),
        };
        BoundCallArgument {
            range: argument.range,
            formal,
            actual,
        }
    }

    fn bind_expr(&mut self, expression: &Expr) -> BoundExpr {
        let kind = match &expression.kind {
            ExprKind::Literal(literal) => BoundExprKind::Literal(literal.clone()),
            ExprKind::Name(name) => match self.resolve(
                &name.spelling,
                name.range,
                expression.id,
                SclAccessKind::Read,
            ) {
                Some(member) => BoundExprKind::Member {
                    constant_value: self.constant_values.get(&member.id).cloned(),
                    member,
                },
                None => BoundExprKind::Error,
            },
            ExprKind::Unary { operator, operand } => BoundExprKind::Unary {
                operator: *operator,
                operand: alloc::boxed::Box::new(self.bind_expr(operand)),
            },
            ExprKind::Binary {
                operator,
                left,
                right,
            } => BoundExprKind::Binary {
                operator: *operator,
                left: alloc::boxed::Box::new(self.bind_expr(left)),
                right: alloc::boxed::Box::new(self.bind_expr(right)),
            },
            ExprKind::Error => BoundExprKind::Error,
        };
        BoundExpr {
            id: expression.id,
            range: expression.range,
            kind,
        }
    }

    fn resolve(
        &mut self,
        spelling: &str,
        range: TextRange,
        node: SemanticNodeId,
        access: SclAccessKind,
    ) -> Option<TypedMember> {
        match self
            .names
            .get(&spelling.to_ascii_lowercase())
            .map(Vec::as_slice)
        {
            None | Some([]) => {
                self.occurrence(
                    spelling,
                    range,
                    node,
                    access,
                    SclOccurrenceResolution::Unresolved,
                    None,
                );
                self.issues.push(SclIssue {
                    code: DiagnosticCode::UNRESOLVED_REFERENCE,
                    range,
                    semantic_node: Some(node),
                    cause: alloc::format!("unresolved interface member '{spelling}'"),
                });
                None
            }
            Some([member]) => {
                let resolved = TypedMember {
                    id: member.id,
                    data_type: member.data_type.clone(),
                    role: member.role,
                };
                self.occurrence(
                    spelling,
                    range,
                    node,
                    access,
                    SclOccurrenceResolution::Resolved,
                    Some(&resolved),
                );
                Some(resolved)
            }
            Some(_) => {
                self.occurrence(
                    spelling,
                    range,
                    node,
                    access,
                    SclOccurrenceResolution::Ambiguous,
                    None,
                );
                self.issues.push(SclIssue {
                    code: DiagnosticCode::AMBIGUOUS_REFERENCE,
                    range,
                    semantic_node: Some(node),
                    cause: alloc::format!(
                        "case-insensitive interface member '{spelling}' has multiple candidates"
                    ),
                });
                None
            }
        }
    }

    fn occurrence(
        &mut self,
        spelling: &str,
        range: TextRange,
        node: SemanticNodeId,
        access: SclAccessKind,
        resolution: SclOccurrenceResolution,
        member: Option<&TypedMember>,
    ) {
        self.occurrences.push(SclSymbolOccurrence {
            source: SourceAnchor::scl(self.source_owner, self.source_revision, node, range),
            spelling: spelling.into(),
            kind: SclOccurrenceKind::MemberReference,
            access,
            resolution,
            member: member.map(|value| value.id),
            data_type: member.map(|value| value.data_type.clone()),
            role: member.map(|value| value.role),
            definition_owner: member.map(|_| self.caller),
        });
    }

    #[allow(clippy::too_many_arguments)]
    fn external_occurrence(
        &mut self,
        spelling: &str,
        range: TextRange,
        node: SemanticNodeId,
        kind: SclOccurrenceKind,
        access: SclAccessKind,
        resolution: SclOccurrenceResolution,
        definition_owner: Option<BlockId>,
        member: Option<&TypedMember>,
    ) {
        self.occurrences.push(SclSymbolOccurrence {
            source: SourceAnchor::scl(self.source_owner, self.source_revision, node, range),
            spelling: spelling.into(),
            kind,
            access,
            resolution,
            member: member.map(|value| value.id),
            data_type: member.map(|value| value.data_type.clone()),
            role: member.map(|value| value.role),
            definition_owner,
        });
    }
}

fn semantic_symbols(block: &ProgramBlock) -> Vec<SclSemanticSymbol> {
    block
        .interface
        .members
        .values()
        .map(|member| SclSemanticSymbol {
            owner: block.id,
            member: member.id,
            name: member.name.clone(),
            data_type: member.data_type.clone(),
            role: member.role,
            declared_order: member.declared_order,
        })
        .collect()
}

fn sort_issues(issues: &mut Vec<SclIssue>) {
    issues.sort_by(|left, right| {
        (left.range, left.code, left.semantic_node, &left.cause).cmp(&(
            right.range,
            right.code,
            right.semantic_node,
            &right.cause,
        ))
    });
    issues.dedup();
}

fn collect_folding_ranges(statements: &[Statement], ranges: &mut Vec<TextRange>) {
    for statement in statements {
        match &statement.parsed_only {
            Some(ParsedOnlyStatement::Case {
                arms, else_body, ..
            }) => {
                ranges.push(statement.range);
                for arm in arms {
                    collect_folding_ranges(&arm.body, ranges);
                }
                collect_folding_ranges(else_body, ranges);
                continue;
            }
            Some(
                ParsedOnlyStatement::For { body, .. }
                | ParsedOnlyStatement::While { body, .. }
                | ParsedOnlyStatement::Repeat { body, .. },
            ) => {
                ranges.push(statement.range);
                collect_folding_ranges(body, ranges);
                continue;
            }
            Some(
                ParsedOnlyStatement::Assignment { .. }
                | ParsedOnlyStatement::Call { .. }
                | ParsedOnlyStatement::Exit
                | ParsedOnlyStatement::Continue
                | ParsedOnlyStatement::UnsupportedDeclaration { .. },
            )
            | None => {}
        }
        match &statement.kind {
            StatementKind::If {
                branches,
                else_body,
            } => {
                ranges.push(statement.range);
                for (_, body) in branches {
                    collect_folding_ranges(body, ranges);
                }
                collect_folding_ranges(else_body, ranges);
            }
            StatementKind::Assignment { .. }
            | StatementKind::Call { .. }
            | StatementKind::Return
            | StatementKind::Error => {}
        }
    }
}

fn collect_type_facts(
    statements: &[TypedStatement],
    source: &SclSource,
    facts: &mut Vec<SclTypeFact>,
) {
    for statement in statements {
        match &statement.kind {
            TypedStatementKind::Assignment { value, .. } => {
                collect_expr_type_facts(value, source, facts);
            }
            TypedStatementKind::If {
                branches,
                else_body,
            } => {
                for (condition, body) in branches {
                    collect_expr_type_facts(condition, source, facts);
                    collect_type_facts(body, source, facts);
                }
                collect_type_facts(else_body, source, facts);
            }
            TypedStatementKind::Case {
                selector,
                arms,
                else_body,
            } => {
                collect_expr_type_facts(selector, source, facts);
                for arm in arms {
                    for label in &arm.labels {
                        collect_expr_type_facts(&label.lower, source, facts);
                        if let Some(upper) = &label.upper {
                            collect_expr_type_facts(upper, source, facts);
                        }
                    }
                    collect_type_facts(&arm.body, source, facts);
                }
                collect_type_facts(else_body, source, facts);
            }
            TypedStatementKind::For {
                initial,
                limit,
                step,
                body,
                ..
            } => {
                collect_expr_type_facts(initial, source, facts);
                collect_expr_type_facts(limit, source, facts);
                collect_expr_type_facts(step, source, facts);
                collect_type_facts(body, source, facts);
            }
            TypedStatementKind::While { condition, body } => {
                collect_expr_type_facts(condition, source, facts);
                collect_type_facts(body, source, facts);
            }
            TypedStatementKind::Repeat { body, condition } => {
                collect_type_facts(body, source, facts);
                collect_expr_type_facts(condition, source, facts);
            }
            TypedStatementKind::Call(call) => {
                for input in &call.inputs {
                    collect_expr_type_facts(&input.value, source, facts);
                }
            }
            TypedStatementKind::Exit
            | TypedStatementKind::Continue
            | TypedStatementKind::Return
            | TypedStatementKind::Error => {}
        }
    }
}

fn collect_expr_type_facts(
    expression: &TypedExpr,
    source: &SclSource,
    facts: &mut Vec<SclTypeFact>,
) {
    if expression.kind == TypedExprKind::Error {
        return;
    }
    facts.push(SclTypeFact {
        source: SourceAnchor::scl(
            source.owner(),
            source.revision_hash(),
            expression.id,
            expression.range,
        ),
        data_type: expression.data_type.clone(),
    });
    match &expression.kind {
        TypedExprKind::Unary { operand, .. } => {
            collect_expr_type_facts(operand, source, facts);
        }
        TypedExprKind::Binary { left, right, .. } => {
            collect_expr_type_facts(left, source, facts);
            collect_expr_type_facts(right, source, facts);
        }
        TypedExprKind::Convert { source: converted } => {
            collect_expr_type_facts(converted, source, facts);
        }
        TypedExprKind::Constant(_) | TypedExprKind::Member(_) | TypedExprKind::Error => {}
    }
}

fn typecheck(bound: &BoundBlock, block: &ProgramBlock) -> (TypedBlock, Vec<SclIssue>) {
    let return_member = block
        .interface
        .members
        .values()
        .find(|member| member.role == InterfaceRole::Return)
        .map(|member| member.id);
    let mut checker = TypeChecker {
        issues: Vec::new(),
        return_member,
        loop_depth: 0,
        loop_iterators: Vec::new(),
    };
    let initial = FlowState {
        assigned_temps: BTreeSet::new(),
        return_assigned: false,
        reachable: true,
    };
    let (statements, final_flow) = checker.check_statements(&bound.statements, initial);
    if final_flow.reachable && return_member.is_some() && !final_flow.return_assigned {
        checker.issues.push(SclIssue {
            code: DiagnosticCode::INVALID_CONTROL_FLOW,
            range: TextRange::empty(0),
            semantic_node: None,
            cause: "FC Return storage is not definitely assigned on the fallthrough path".into(),
        });
    }
    (
        TypedBlock {
            owner: bound.owner,
            kind: bound.kind,
            statements,
        },
        checker.issues,
    )
}

#[derive(Clone)]
struct FlowState {
    assigned_temps: BTreeSet<InterfaceMemberId>,
    return_assigned: bool,
    reachable: bool,
}

struct TypeChecker {
    issues: Vec<SclIssue>,
    return_member: Option<InterfaceMemberId>,
    loop_depth: usize,
    loop_iterators: Vec<InterfaceMemberId>,
}

impl TypeChecker {
    fn check_statements(
        &mut self,
        statements: &[BoundStatement],
        mut flow: FlowState,
    ) -> (Vec<TypedStatement>, FlowState) {
        let mut typed = Vec::with_capacity(statements.len());
        for statement in statements {
            let (next, next_flow) = self.check_statement(statement, flow);
            typed.push(next);
            flow = next_flow;
        }
        (typed, flow)
    }

    #[allow(clippy::too_many_lines)]
    fn check_statement(
        &mut self,
        statement: &BoundStatement,
        mut flow: FlowState,
    ) -> (TypedStatement, FlowState) {
        let kind = match &statement.kind {
            BoundStatementKind::Assignment { target, value } => {
                let expected = target.as_ref().map(|member| &member.data_type);
                let typed_value = self.check_expr(value, expected, &flow);
                if let Some(target) = target {
                    if self.loop_iterators.contains(&target.id) {
                        self.issue(
                            DiagnosticCode::INVALID_CONTROL_FLOW,
                            statement.range,
                            statement.id,
                            "FOR iterator must not be assigned inside its active loop",
                        );
                    }
                    if matches!(target.role, InterfaceRole::Input | InterfaceRole::Constant) {
                        self.issue(
                            DiagnosticCode::TYPE_MISMATCH,
                            statement.range,
                            statement.id,
                            "assignment target is read-only",
                        );
                    }
                    if typed_value.data_type != target.data_type
                        && typed_value.kind != TypedExprKind::Error
                    {
                        self.issue(
                            DiagnosticCode::TYPE_MISMATCH,
                            value.range,
                            value.id,
                            alloc::format!(
                                "assignment expects {:?} but expression is {:?}",
                                target.data_type,
                                typed_value.data_type
                            ),
                        );
                    }
                    if target.role == InterfaceRole::Temp {
                        flow.assigned_temps.insert(target.id);
                    }
                    if Some(target.id) == self.return_member {
                        flow.return_assigned = true;
                    }
                }
                TypedStatementKind::Assignment {
                    target: target.clone().unwrap_or(TypedMember {
                        id: InterfaceMemberId::new(0),
                        data_type: DataType::Bool,
                        role: InterfaceRole::Temp,
                    }),
                    value: typed_value,
                }
            }
            BoundStatementKind::If {
                branches,
                else_body,
            } => {
                let incoming = flow.clone();
                let mut typed_branches = Vec::with_capacity(branches.len());
                let mut exits = Vec::new();
                for (condition, body) in branches {
                    let condition = self.check_expr(condition, Some(&DataType::Bool), &incoming);
                    if condition.data_type != DataType::Bool
                        && condition.kind != TypedExprKind::Error
                    {
                        self.issue(
                            DiagnosticCode::TYPE_MISMATCH,
                            condition.range,
                            condition.id,
                            "IF and ELSIF conditions must be BOOL",
                        );
                    }
                    let (body, exit) = self.check_statements(body, incoming.clone());
                    typed_branches.push((condition, body));
                    exits.push(exit);
                }
                let (typed_else, else_exit) = if else_body.is_empty() {
                    (Vec::new(), incoming.clone())
                } else {
                    self.check_statements(else_body, incoming.clone())
                };
                exits.push(else_exit);
                flow = merge_flows(&exits, &incoming);
                TypedStatementKind::If {
                    branches: typed_branches,
                    else_body: typed_else,
                }
            }
            BoundStatementKind::Case {
                selector,
                arms,
                else_body,
            } => self.check_case(statement, selector, arms, else_body, &mut flow),
            BoundStatementKind::For {
                iterator,
                initial,
                limit,
                step,
                body,
            } => self.check_for(
                statement,
                iterator.as_ref(),
                initial,
                limit,
                step.as_deref(),
                body,
                &mut flow,
            ),
            BoundStatementKind::While { condition, body } => {
                let condition = self.check_bool_condition(condition, &flow, "WHILE");
                self.loop_depth = self.loop_depth.saturating_add(1);
                let (body, _) = self.check_statements(body, flow.clone());
                self.loop_depth = self.loop_depth.saturating_sub(1);
                TypedStatementKind::While { condition, body }
            }
            BoundStatementKind::Repeat { body, condition } => {
                self.loop_depth = self.loop_depth.saturating_add(1);
                let (body, body_flow) = self.check_statements(body, flow.clone());
                self.loop_depth = self.loop_depth.saturating_sub(1);
                let condition = self.check_bool_condition(condition, &body_flow, "UNTIL");
                TypedStatementKind::Repeat { body, condition }
            }
            BoundStatementKind::Exit => {
                if self.loop_depth == 0 {
                    self.issue(
                        DiagnosticCode::INVALID_CONTROL_FLOW,
                        statement.range,
                        statement.id,
                        "EXIT is only legal inside FOR, WHILE, or REPEAT",
                    );
                }
                flow.reachable = false;
                TypedStatementKind::Exit
            }
            BoundStatementKind::Continue => {
                if self.loop_depth == 0 {
                    self.issue(
                        DiagnosticCode::INVALID_CONTROL_FLOW,
                        statement.range,
                        statement.id,
                        "CONTINUE is only legal inside FOR, WHILE, or REPEAT",
                    );
                }
                flow.reachable = false;
                TypedStatementKind::Continue
            }
            BoundStatementKind::Call(call) => self.check_call(statement, call, &mut flow),
            BoundStatementKind::Return => {
                if self.return_member.is_some() && !flow.return_assigned {
                    self.issue(
                        DiagnosticCode::INVALID_CONTROL_FLOW,
                        statement.range,
                        statement.id,
                        "RETURN is reachable before FC Return storage is definitely assigned",
                    );
                }
                flow.reachable = false;
                TypedStatementKind::Return
            }
            BoundStatementKind::Error => TypedStatementKind::Error,
        };
        (
            TypedStatement {
                id: statement.id,
                range: statement.range,
                kind,
            },
            flow,
        )
    }

    fn check_bool_condition(
        &mut self,
        condition: &BoundExpr,
        flow: &FlowState,
        construct: &str,
    ) -> TypedExpr {
        let condition = self.check_expr(condition, Some(&DataType::Bool), flow);
        if condition.data_type != DataType::Bool && condition.kind != TypedExprKind::Error {
            self.issue(
                DiagnosticCode::TYPE_MISMATCH,
                condition.range,
                condition.id,
                alloc::format!("{construct} condition must be BOOL"),
            );
        }
        condition
    }

    fn check_case(
        &mut self,
        statement: &BoundStatement,
        selector: &BoundExpr,
        arms: &[BoundCaseArm],
        else_body: &[BoundStatement],
        flow: &mut FlowState,
    ) -> TypedStatementKind {
        let incoming = flow.clone();
        let selector = self.check_expr(selector, None, &incoming);
        let selector_is_ordinal = is_case_ordinal(&selector.data_type);
        if !selector_is_ordinal && selector.kind != TypedExprKind::Error {
            self.issue(
                DiagnosticCode::TYPE_MISMATCH,
                selector.range,
                selector.id,
                "CASE selector must have a canonical integer or CHAR type",
            );
        }

        let mut typed_arms = Vec::with_capacity(arms.len());
        let mut exits = Vec::new();
        let mut intervals = Vec::<(CanonicalValue, CanonicalValue, TextRange)>::new();
        for arm in arms {
            let mut typed_labels = Vec::with_capacity(arm.labels.len());
            for label in &arm.labels {
                let lower = self.check_case_constant(
                    &label.lower,
                    &selector.data_type,
                    &incoming,
                    "CASE label",
                );
                let upper = label.upper.as_ref().map(|upper| {
                    self.check_case_constant(
                        upper,
                        &selector.data_type,
                        &incoming,
                        "CASE range endpoint",
                    )
                });
                if selector_is_ordinal {
                    let lower_value = constant_value(&lower).cloned();
                    let upper_value = upper
                        .as_ref()
                        .and_then(constant_value)
                        .cloned()
                        .or_else(|| lower_value.clone());
                    if let (Some(lower_value), Some(upper_value)) = (lower_value, upper_value) {
                        if lower_value > upper_value {
                            self.issue(
                                DiagnosticCode::CONSTANT_RANGE_OR_ARITHMETIC,
                                label.range,
                                statement.id,
                                "CASE range lower endpoint exceeds its upper endpoint",
                            );
                        } else {
                            if intervals.iter().any(|(prior_lower, prior_upper, _)| {
                                lower_value <= *prior_upper && *prior_lower <= upper_value
                            }) {
                                self.issue(
                                    DiagnosticCode::INVALID_CONTROL_FLOW,
                                    label.range,
                                    statement.id,
                                    "CASE labels and ranges must not overlap",
                                );
                            }
                            intervals.push((lower_value, upper_value, label.range));
                        }
                    }
                }
                typed_labels.push(TypedCaseLabel {
                    range: label.range,
                    lower,
                    upper,
                });
            }
            let (body, exit) = self.check_statements(&arm.body, incoming.clone());
            exits.push(exit);
            typed_arms.push(TypedCaseArm {
                range: arm.range,
                labels: typed_labels,
                body,
            });
        }
        let (typed_else, else_exit) = if else_body.is_empty() {
            (Vec::new(), incoming.clone())
        } else {
            self.check_statements(else_body, incoming.clone())
        };
        exits.push(else_exit);
        *flow = merge_flows(&exits, &incoming);
        TypedStatementKind::Case {
            selector,
            arms: typed_arms,
            else_body: typed_else,
        }
    }

    fn check_case_constant(
        &mut self,
        expression: &BoundExpr,
        expected: &DataType,
        flow: &FlowState,
        description: &str,
    ) -> TypedExpr {
        let mut typed = self.check_expr(expression, Some(expected), flow);
        if typed.kind == TypedExprKind::Error {
            return typed;
        }
        if typed.data_type != *expected {
            self.issue(
                DiagnosticCode::TYPE_MISMATCH,
                typed.range,
                typed.id,
                alloc::format!("{description} must exactly match the CASE selector type"),
            );
            return typed;
        }
        match fold_ordinal_constant(&typed) {
            Some(value) => typed.kind = TypedExprKind::Constant(value),
            None => self.issue(
                DiagnosticCode::CONSTANT_RANGE_OR_ARITHMETIC,
                typed.range,
                typed.id,
                alloc::format!("{description} must be an integer or CHAR constant expression"),
            ),
        }
        typed
    }

    #[allow(clippy::too_many_arguments)]
    fn check_for(
        &mut self,
        statement: &BoundStatement,
        iterator: Option<&TypedMember>,
        initial: &BoundExpr,
        limit: &BoundExpr,
        step: Option<&BoundExpr>,
        body: &[BoundStatement],
        flow: &mut FlowState,
    ) -> TypedStatementKind {
        let iterator = iterator.cloned().unwrap_or(TypedMember {
            id: InterfaceMemberId::new(0),
            data_type: DataType::DInt,
            role: InterfaceRole::Temp,
        });
        if !is_for_integer(&iterator.data_type) {
            self.issue(
                DiagnosticCode::TYPE_MISMATCH,
                statement.range,
                statement.id,
                "FOR iterator must have a canonical signed integer type",
            );
        }
        if matches!(
            iterator.role,
            InterfaceRole::Input | InterfaceRole::Constant
        ) {
            self.issue(
                DiagnosticCode::TYPE_MISMATCH,
                statement.range,
                statement.id,
                "FOR iterator must be writable",
            );
        }
        if self.loop_iterators.contains(&iterator.id) {
            self.issue(
                DiagnosticCode::INVALID_CONTROL_FLOW,
                statement.range,
                statement.id,
                "nested FOR loops must not reuse an active iterator",
            );
        }
        let initial = self.check_expr(initial, Some(&iterator.data_type), flow);
        let initial = self.coerce_for_value(initial, &iterator.data_type, "initial");
        let limit = self.check_expr(limit, Some(&iterator.data_type), flow);
        let limit = self.coerce_for_value(limit, &iterator.data_type, "terminal");
        let step = self.check_for_step(statement, &iterator, step, flow);

        let mut body_flow = flow.clone();
        if iterator.role == InterfaceRole::Temp {
            body_flow.assigned_temps.insert(iterator.id);
        }
        if Some(iterator.id) == self.return_member {
            body_flow.return_assigned = true;
        }
        self.loop_depth = self.loop_depth.saturating_add(1);
        self.loop_iterators.push(iterator.id);
        let (body, _) = self.check_statements(body, body_flow.clone());
        self.loop_iterators.pop();
        self.loop_depth = self.loop_depth.saturating_sub(1);
        *flow = body_flow;
        TypedStatementKind::For {
            iterator,
            initial: alloc::boxed::Box::new(initial),
            limit: alloc::boxed::Box::new(limit),
            step: alloc::boxed::Box::new(step),
            body,
        }
    }

    fn check_for_step(
        &mut self,
        statement: &BoundStatement,
        iterator: &TypedMember,
        step: Option<&BoundExpr>,
        flow: &FlowState,
    ) -> TypedExpr {
        let step = step.map_or_else(
            || TypedExpr {
                id: statement.id,
                range: statement.range,
                data_type: iterator.data_type.clone(),
                kind: canonical_one(&iterator.data_type)
                    .map_or(TypedExprKind::Error, TypedExprKind::Constant),
            },
            |step| self.check_expr(step, Some(&iterator.data_type), flow),
        );
        let mut step = self.coerce_for_value(step, &iterator.data_type, "BY");
        if let Some(value) = fold_ordinal_constant(&step) {
            step.kind = TypedExprKind::Constant(value);
        }
        if constant_integer_sign(&step) == Some(0) {
            self.issue(
                DiagnosticCode::CONSTANT_RANGE_OR_ARITHMETIC,
                step.range,
                step.id,
                "FOR BY expression must not be zero",
            );
        }
        step
    }

    fn coerce_for_value(
        &mut self,
        expression: TypedExpr,
        destination: &DataType,
        description: &str,
    ) -> TypedExpr {
        if expression.kind == TypedExprKind::Error || expression.data_type == *destination {
            return expression;
        }
        let allowed = expression
            .data_type
            .primitive_type()
            .zip(destination.primitive_type())
            .is_some_and(|(source, destination)| {
                plc_types::implicit_conversion_allowed(source, destination)
            });
        if allowed {
            TypedExpr {
                id: expression.id,
                range: expression.range,
                data_type: destination.clone(),
                kind: TypedExprKind::Convert {
                    source: alloc::boxed::Box::new(expression),
                },
            }
        } else {
            self.issue(
                DiagnosticCode::TYPE_MISMATCH,
                expression.range,
                expression.id,
                alloc::format!(
                    "FOR {description} expression has no unique implicit conversion to the iterator type"
                ),
            );
            expression
        }
    }

    #[allow(clippy::too_many_lines)]
    fn check_call(
        &mut self,
        statement: &BoundStatement,
        call: &BoundCall,
        flow: &mut FlowState,
    ) -> TypedStatementKind {
        let Some(target) = &call.target else {
            return TypedStatementKind::Error;
        };
        if target.kind == ProgramUnitKind::Function {
            let mut inputs = Vec::new();
            let mut outputs = Vec::new();
            let mut seen = BTreeSet::new();
            for argument in &call.arguments {
                let Some(formal) = &argument.formal else {
                    continue;
                };
                if !seen.insert(formal.id) {
                    self.issue(
                        DiagnosticCode::ILLEGAL_OR_OVERLAPPING_BINDING,
                        argument.range,
                        statement.id,
                        "call formal is bound more than once",
                    );
                    continue;
                }
                match (&argument.actual, formal.role) {
                    (BoundCallActual::Input(value), InterfaceRole::Input) => {
                        let value = self.check_expr(value, Some(&formal.data_type), flow);
                        if value.data_type != formal.data_type && value.kind != TypedExprKind::Error
                        {
                            self.issue(
                                DiagnosticCode::TYPE_MISMATCH,
                                argument.range,
                                statement.id,
                                "call input type does not match its formal",
                            );
                        }
                        inputs.push(TypedCallInput {
                            formal: formal.id,
                            value,
                        });
                    }
                    (BoundCallActual::Input(value), InterfaceRole::InOut) => {
                        let value = self.check_expr(value, Some(&formal.data_type), flow);
                        let TypedExprKind::Member(actual) = &value.kind else {
                            self.issue(
                                DiagnosticCode::ILLEGAL_OR_OVERLAPPING_BINDING,
                                argument.range,
                                statement.id,
                                "IN_OUT binding requires one writable caller variable",
                            );
                            continue;
                        };
                        self.reject_active_iterator_write(
                            actual,
                            argument.range,
                            statement.id,
                            "IN_OUT binding",
                        );
                        if matches!(actual.role, InterfaceRole::Input | InterfaceRole::Constant)
                            || value.data_type != formal.data_type
                        {
                            self.issue(
                                DiagnosticCode::TYPE_MISMATCH,
                                argument.range,
                                statement.id,
                                "IN_OUT actual must be writable and exactly typed",
                            );
                        }
                        let actual = actual.clone();
                        inputs.push(TypedCallInput {
                            formal: formal.id,
                            value,
                        });
                        outputs.push(TypedCallOutput {
                            formal: formal.id,
                            target: actual,
                        });
                    }
                    (
                        BoundCallActual::Output(Some(actual)),
                        InterfaceRole::Output | InterfaceRole::Return,
                    ) => {
                        self.reject_active_iterator_write(
                            actual,
                            argument.range,
                            statement.id,
                            "call output binding",
                        );
                        if matches!(actual.role, InterfaceRole::Input | InterfaceRole::Constant)
                            || actual.data_type != formal.data_type
                        {
                            self.issue(
                                DiagnosticCode::TYPE_MISMATCH,
                                argument.range,
                                statement.id,
                                "call output actual must be writable and exactly typed",
                            );
                        }
                        outputs.push(TypedCallOutput {
                            formal: formal.id,
                            target: actual.clone(),
                        });
                    }
                    (BoundCallActual::Output(None | Some(_)) | BoundCallActual::Input(_), _) => {
                        self.issue(
                            DiagnosticCode::ILLEGAL_OR_OVERLAPPING_BINDING,
                            argument.range,
                            statement.id,
                            "call binding operator disagrees with formal direction",
                        );
                    }
                }
            }
            for member in target.interface.members.values() {
                let required = match member.role {
                    InterfaceRole::Input => member.default_value.is_none(),
                    InterfaceRole::InOut => true,
                    InterfaceRole::Output | InterfaceRole::Return => member.required_output_binding,
                    InterfaceRole::Static | InterfaceRole::Temp | InterfaceRole::Constant => false,
                };
                if required && !seen.contains(&member.id) {
                    self.issue(
                        DiagnosticCode::REQUIRED_BINDING_MISSING,
                        statement.range,
                        statement.id,
                        alloc::format!("required call formal '{}' is not bound", member.name),
                    );
                }
            }
            inputs.sort_by_key(|binding| binding.formal);
            outputs.sort_by_key(|binding| binding.formal);
            for output in &outputs {
                if output.target.role == InterfaceRole::Temp {
                    flow.assigned_temps.insert(output.target.id);
                }
                if Some(output.target.id) == self.return_member {
                    flow.return_assigned = true;
                }
            }
            return TypedStatementKind::Call(TypedCall {
                instruction: CALL_FC,
                target: target.id,
                inputs,
                outputs,
            });
        }
        self.issue(
            DiagnosticCode::INSTANCE_INVALID,
            statement.range,
            statement.id,
            "SCL FB calls require explicit instance syntax; this call form only targets FCs",
        );
        TypedStatementKind::Error
    }

    fn reject_active_iterator_write(
        &mut self,
        member: &TypedMember,
        range: TextRange,
        node: SemanticNodeId,
        binding: &str,
    ) {
        if self.loop_iterators.contains(&member.id) {
            self.issue(
                DiagnosticCode::INVALID_CONTROL_FLOW,
                range,
                node,
                alloc::format!(
                    "{binding} must not write the active FOR iterator from inside its loop"
                ),
            );
        }
    }

    #[allow(clippy::too_many_lines)]
    fn check_expr(
        &mut self,
        expression: &BoundExpr,
        expected: Option<&DataType>,
        flow: &FlowState,
    ) -> TypedExpr {
        let (data_type, kind) = match &expression.kind {
            BoundExprKind::Literal(literal) => match type_literal(literal, expected) {
                Ok((data_type, value)) => (data_type, TypedExprKind::Constant(value)),
                Err(cause) => {
                    self.issue(
                        DiagnosticCode::CONSTANT_RANGE_OR_ARITHMETIC,
                        expression.range,
                        expression.id,
                        cause,
                    );
                    (DataType::Bool, TypedExprKind::Error)
                }
            },
            BoundExprKind::Member {
                member,
                constant_value,
            } => {
                if let Some(value) = constant_value {
                    return TypedExpr {
                        id: expression.id,
                        range: expression.range,
                        data_type: member.data_type.clone(),
                        kind: TypedExprKind::Constant(value.clone()),
                    };
                }
                if member.role == InterfaceRole::Temp && !flow.assigned_temps.contains(&member.id) {
                    self.issue(
                        DiagnosticCode::INVALID_CONTROL_FLOW,
                        expression.range,
                        expression.id,
                        "TEMP is read before definite assignment",
                    );
                }
                (
                    member.data_type.clone(),
                    TypedExprKind::Member(member.clone()),
                )
            }
            BoundExprKind::Unary { operator, operand } => {
                let operand = self.check_expr(operand, expected, flow);
                let valid = match operator {
                    UnaryOp::Not => operand.data_type == DataType::Bool,
                    UnaryOp::Plus => is_numeric(&operand.data_type),
                    UnaryOp::Minus => operand.data_type.primitive_type().is_some_and(|primitive| {
                        primitive.is_signed_integer()
                            || matches!(
                                primitive,
                                plc_types::PrimitiveType::Real | plc_types::PrimitiveType::Lreal
                            )
                    }),
                };
                if !valid && operand.kind != TypedExprKind::Error {
                    self.issue(
                        DiagnosticCode::TYPE_MISMATCH,
                        expression.range,
                        expression.id,
                        "unary operator is not defined for the operand type",
                    );
                }
                (
                    operand.data_type.clone(),
                    TypedExprKind::Unary {
                        operator: *operator,
                        operand: alloc::boxed::Box::new(operand),
                    },
                )
            }
            BoundExprKind::Binary {
                operator,
                left,
                right,
            } => {
                let operand_hint = expected
                    .filter(|value| is_numeric(value))
                    .or_else(|| bound_type_hint(left))
                    .or_else(|| bound_type_hint(right));
                let left = self.check_expr(left, operand_hint, flow);
                let right = self.check_expr(right, Some(&left.data_type), flow);
                let operands_match = left.data_type == right.data_type;
                let result_type = match operator {
                    BinaryOp::And | BinaryOp::Xor | BinaryOp::Or => {
                        if left.data_type != DataType::Bool || right.data_type != DataType::Bool {
                            self.issue(
                                DiagnosticCode::TYPE_MISMATCH,
                                expression.range,
                                expression.id,
                                "AND, XOR, and OR require BOOL operands",
                            );
                        }
                        DataType::Bool
                    }
                    BinaryOp::Equal
                    | BinaryOp::NotEqual
                    | BinaryOp::Less
                    | BinaryOp::LessEqual
                    | BinaryOp::Greater
                    | BinaryOp::GreaterEqual => {
                        if !operands_match {
                            self.issue(
                                DiagnosticCode::TYPE_MISMATCH,
                                expression.range,
                                expression.id,
                                "comparison operands must have one exact canonical type",
                            );
                        }
                        DataType::Bool
                    }
                    BinaryOp::Multiply
                    | BinaryOp::Divide
                    | BinaryOp::Modulo
                    | BinaryOp::Add
                    | BinaryOp::Subtract => {
                        if !operands_match || !is_numeric(&left.data_type) {
                            self.issue(
                                DiagnosticCode::TYPE_MISMATCH,
                                expression.range,
                                expression.id,
                                "arithmetic operands must have one exact supported numeric type",
                            );
                        }
                        if *operator == BinaryOp::Modulo
                            && matches!(left.data_type, DataType::Real | DataType::LReal)
                        {
                            self.issue(
                                DiagnosticCode::TYPE_MISMATCH,
                                expression.range,
                                expression.id,
                                "MOD requires an integer type",
                            );
                        }
                        left.data_type.clone()
                    }
                };
                (
                    result_type,
                    TypedExprKind::Binary {
                        operator: *operator,
                        left: alloc::boxed::Box::new(left),
                        right: alloc::boxed::Box::new(right),
                    },
                )
            }
            BoundExprKind::Error => (DataType::Bool, TypedExprKind::Error),
        };
        TypedExpr {
            id: expression.id,
            range: expression.range,
            data_type,
            kind,
        }
    }

    fn issue(
        &mut self,
        code: DiagnosticCode,
        range: TextRange,
        node: SemanticNodeId,
        cause: impl Into<String>,
    ) {
        self.issues.push(SclIssue {
            code,
            range,
            semantic_node: Some(node),
            cause: cause.into(),
        });
    }
}

fn merge_flows(exits: &[FlowState], incoming: &FlowState) -> FlowState {
    let reachable: Vec<_> = exits.iter().filter(|flow| flow.reachable).collect();
    if reachable.is_empty() {
        return FlowState {
            assigned_temps: incoming.assigned_temps.clone(),
            return_assigned: incoming.return_assigned,
            reachable: false,
        };
    }
    let mut assigned_temps = reachable[0].assigned_temps.clone();
    for flow in &reachable[1..] {
        assigned_temps.retain(|member| flow.assigned_temps.contains(member));
    }
    FlowState {
        assigned_temps,
        return_assigned: reachable.iter().all(|flow| flow.return_assigned),
        reachable: true,
    }
}

fn bound_type_hint(expression: &BoundExpr) -> Option<&DataType> {
    match &expression.kind {
        BoundExprKind::Member { member, .. } => Some(&member.data_type),
        BoundExprKind::Unary { operand, .. } => bound_type_hint(operand),
        BoundExprKind::Binary { left, right, .. } => {
            bound_type_hint(left).or_else(|| bound_type_hint(right))
        }
        BoundExprKind::Literal(_) | BoundExprKind::Error => None,
    }
}

fn is_numeric(data_type: &DataType) -> bool {
    data_type
        .primitive_type()
        .is_some_and(plc_types::PrimitiveType::is_numeric)
}

fn is_case_ordinal(data_type: &DataType) -> bool {
    data_type.primitive_type().is_some_and(|primitive| {
        primitive.is_integer() || primitive == plc_types::PrimitiveType::Char
    })
}

fn is_for_integer(data_type: &DataType) -> bool {
    data_type
        .primitive_type()
        .is_some_and(plc_types::PrimitiveType::is_signed_integer)
}

fn constant_value(expression: &TypedExpr) -> Option<&CanonicalValue> {
    if let TypedExprKind::Constant(value) = &expression.kind {
        Some(value)
    } else {
        None
    }
}

fn fold_ordinal_constant(expression: &TypedExpr) -> Option<CanonicalValue> {
    match &expression.kind {
        TypedExprKind::Constant(value) if is_case_ordinal(&expression.data_type) => {
            Some(value.clone())
        }
        TypedExprKind::Unary {
            operator: UnaryOp::Plus,
            operand,
        } => fold_ordinal_constant(operand),
        TypedExprKind::Unary {
            operator: UnaryOp::Minus,
            operand,
        } => match fold_ordinal_constant(operand)? {
            CanonicalValue::SInt(value) => value.checked_neg().map(CanonicalValue::SInt),
            CanonicalValue::Int(value) => value.checked_neg().map(CanonicalValue::Int),
            CanonicalValue::DInt(value) => value.checked_neg().map(CanonicalValue::DInt),
            CanonicalValue::LInt(value) => value.checked_neg().map(CanonicalValue::LInt),
            CanonicalValue::Bool(_)
            | CanonicalValue::USInt(_)
            | CanonicalValue::UInt(_)
            | CanonicalValue::UDInt(_)
            | CanonicalValue::ULInt(_)
            | CanonicalValue::Byte(_)
            | CanonicalValue::Word(_)
            | CanonicalValue::DWord(_)
            | CanonicalValue::LWord(_)
            | CanonicalValue::RealBits(_)
            | CanonicalValue::LRealBits(_)
            | CanonicalValue::Char(_)
            | CanonicalValue::TimeMilliseconds(_)
            | CanonicalValue::StringBytes(_)
            | CanonicalValue::Aggregate(_) => None,
        },
        TypedExprKind::Binary {
            operator,
            left,
            right,
        } => fold_ordinal_binary(
            *operator,
            &expression.data_type,
            &fold_ordinal_constant(left)?,
            &fold_ordinal_constant(right)?,
        ),
        TypedExprKind::Convert { source } => ordinal_from_i128(
            &expression.data_type,
            ordinal_i128(&fold_ordinal_constant(source)?)?,
        ),
        TypedExprKind::Unary { .. }
        | TypedExprKind::Constant(_)
        | TypedExprKind::Member(_)
        | TypedExprKind::Error => None,
    }
}

fn fold_ordinal_binary(
    operator: BinaryOp,
    data_type: &DataType,
    left: &CanonicalValue,
    right: &CanonicalValue,
) -> Option<CanonicalValue> {
    let left = ordinal_i128(left)?;
    let right = ordinal_i128(right)?;
    let result = match operator {
        BinaryOp::Add => left.checked_add(right),
        BinaryOp::Subtract => left.checked_sub(right),
        BinaryOp::Multiply => left.checked_mul(right),
        BinaryOp::Divide => left.checked_div(right),
        BinaryOp::Modulo => left.checked_rem(right),
        BinaryOp::And
        | BinaryOp::Xor
        | BinaryOp::Or
        | BinaryOp::Equal
        | BinaryOp::NotEqual
        | BinaryOp::Less
        | BinaryOp::LessEqual
        | BinaryOp::Greater
        | BinaryOp::GreaterEqual => None,
    }?;
    ordinal_from_i128(data_type, result)
}

fn ordinal_i128(value: &CanonicalValue) -> Option<i128> {
    Some(match value {
        CanonicalValue::SInt(value) => i128::from(*value),
        CanonicalValue::Int(value) => i128::from(*value),
        CanonicalValue::DInt(value) => i128::from(*value),
        CanonicalValue::LInt(value) => i128::from(*value),
        CanonicalValue::USInt(value) | CanonicalValue::Char(value) => i128::from(*value),
        CanonicalValue::UInt(value) => i128::from(*value),
        CanonicalValue::UDInt(value) => i128::from(*value),
        CanonicalValue::ULInt(value) => i128::from(*value),
        CanonicalValue::Bool(_)
        | CanonicalValue::Byte(_)
        | CanonicalValue::Word(_)
        | CanonicalValue::DWord(_)
        | CanonicalValue::LWord(_)
        | CanonicalValue::RealBits(_)
        | CanonicalValue::LRealBits(_)
        | CanonicalValue::TimeMilliseconds(_)
        | CanonicalValue::StringBytes(_)
        | CanonicalValue::Aggregate(_) => return None,
    })
}

fn ordinal_from_i128(data_type: &DataType, value: i128) -> Option<CanonicalValue> {
    Some(match data_type {
        DataType::SInt => CanonicalValue::SInt(i8::try_from(value).ok()?),
        DataType::Int => CanonicalValue::Int(i16::try_from(value).ok()?),
        DataType::DInt => CanonicalValue::DInt(i32::try_from(value).ok()?),
        DataType::LInt => CanonicalValue::LInt(i64::try_from(value).ok()?),
        DataType::USInt => CanonicalValue::USInt(u8::try_from(value).ok()?),
        DataType::UInt => CanonicalValue::UInt(u16::try_from(value).ok()?),
        DataType::UDInt => CanonicalValue::UDInt(u32::try_from(value).ok()?),
        DataType::ULInt => CanonicalValue::ULInt(u64::try_from(value).ok()?),
        DataType::Bool
        | DataType::Byte
        | DataType::Word
        | DataType::DWord
        | DataType::LWord
        | DataType::Real
        | DataType::LReal
        | DataType::Char
        | DataType::Time
        | DataType::String { .. }
        | DataType::Named(_)
        | DataType::BlockInstance(_)
        | DataType::InstructionState(_)
        | DataType::Aggregate(_) => return None,
    })
}

fn canonical_one(data_type: &DataType) -> Option<CanonicalValue> {
    Some(match data_type {
        DataType::SInt => CanonicalValue::SInt(1),
        DataType::Int => CanonicalValue::Int(1),
        DataType::DInt => CanonicalValue::DInt(1),
        DataType::LInt => CanonicalValue::LInt(1),
        DataType::USInt => CanonicalValue::USInt(1),
        DataType::UInt => CanonicalValue::UInt(1),
        DataType::UDInt => CanonicalValue::UDInt(1),
        DataType::ULInt => CanonicalValue::ULInt(1),
        DataType::Bool
        | DataType::Byte
        | DataType::Word
        | DataType::DWord
        | DataType::LWord
        | DataType::Real
        | DataType::LReal
        | DataType::Char
        | DataType::Time
        | DataType::String { .. }
        | DataType::Named(_)
        | DataType::BlockInstance(_)
        | DataType::InstructionState(_)
        | DataType::Aggregate(_) => return None,
    })
}

fn constant_integer_sign(expression: &TypedExpr) -> Option<i8> {
    match ordinal_i128(&fold_ordinal_constant(expression)?)?.cmp(&0) {
        core::cmp::Ordering::Less => Some(-1),
        core::cmp::Ordering::Equal => Some(0),
        core::cmp::Ordering::Greater => Some(1),
    }
}

fn type_literal(
    literal: &Literal,
    expected: Option<&DataType>,
) -> Result<(DataType, CanonicalValue), String> {
    match literal {
        Literal::Bool(value) => Ok((DataType::Bool, CanonicalValue::Bool(*value))),
        Literal::Integer(text) => integer_literal(text, expected),
        Literal::Real(text) => real_literal(text),
        Literal::Quoted(text) => quoted_literal(text, expected),
        Literal::Time(text) => time_literal(text),
        Literal::Typed(text) => typed_literal(text, expected),
    }
}

fn integer_literal(
    text: &str,
    expected: Option<&DataType>,
) -> Result<(DataType, CanonicalValue), String> {
    let value = parse_integer(text)?;
    match expected {
        Some(DataType::SInt) => i8::try_from(value)
            .map(|value| (DataType::SInt, CanonicalValue::SInt(value)))
            .map_err(|_| "integer literal is outside SINT range".to_string()),
        Some(DataType::Int) => i16::try_from(value)
            .map(|value| (DataType::Int, CanonicalValue::Int(value)))
            .map_err(|_| "integer literal is outside INT range".to_string()),
        Some(DataType::DInt) | None => i32::try_from(value)
            .map(|value| (DataType::DInt, CanonicalValue::DInt(value)))
            .map_err(|_| "integer literal is outside DINT range".to_string()),
        Some(DataType::LInt) => Ok((DataType::LInt, CanonicalValue::LInt(value))),
        Some(DataType::USInt) => u8::try_from(value)
            .map(|value| (DataType::USInt, CanonicalValue::USInt(value)))
            .map_err(|_| "integer literal is outside USINT range".to_string()),
        Some(DataType::UInt) => u16::try_from(value)
            .map(|value| (DataType::UInt, CanonicalValue::UInt(value)))
            .map_err(|_| "integer literal is outside UINT range".to_string()),
        Some(DataType::UDInt) => u32::try_from(value)
            .map(|value| (DataType::UDInt, CanonicalValue::UDInt(value)))
            .map_err(|_| "integer literal is outside UDINT range".to_string()),
        Some(DataType::ULInt) => u64::try_from(value)
            .map(|value| (DataType::ULInt, CanonicalValue::ULInt(value)))
            .map_err(|_| "integer literal is outside ULINT range".to_string()),
        Some(other) => Err(alloc::format!(
            "integer literal cannot implicitly bind to {other:?}"
        )),
    }
}

fn parse_integer(text: &str) -> Result<i64, String> {
    if let Some((base, digits)) = text.split_once('#') {
        let radix = match base {
            "2" => 2,
            "8" => 8,
            "16" => 16,
            _ => return Err("integer base must be 2, 8, or 16".into()),
        };
        i64::from_str_radix(digits, radix)
            .map_err(|_| "base-qualified integer has invalid digits or range".into())
    } else {
        text.parse::<i64>()
            .map_err(|_| "decimal integer has invalid digits or range".into())
    }
}

fn real_literal(text: &str) -> Result<(DataType, CanonicalValue), String> {
    let value = text
        .parse::<f32>()
        .map_err(|_| "REAL literal is malformed or outside range".to_string())?;
    if !value.is_finite() {
        return Err("REAL literal must be finite".into());
    }
    Ok((DataType::Real, CanonicalValue::RealBits(value.to_bits())))
}

fn quoted_literal(
    text: &str,
    expected: Option<&DataType>,
) -> Result<(DataType, CanonicalValue), String> {
    let decoded = decode_quoted(text)?;
    if expected == Some(&DataType::Char) {
        let bytes = decoded.as_bytes();
        return match bytes {
            [value] => Ok((DataType::Char, CanonicalValue::Char(*value))),
            _ => Err("CHAR literal requires exactly one canonical 8-bit character".into()),
        };
    }
    let capacity = match expected {
        Some(DataType::String { capacity }) => *capacity,
        Some(other) => {
            return Err(alloc::format!(
                "quoted literal cannot implicitly bind to {other:?}"
            ));
        }
        None => u16::try_from(decoded.len())
            .map_err(|_| "quoted literal exceeds canonical STRING capacity".to_string())?,
    };
    if decoded.len() > usize::from(capacity) {
        return Err("quoted literal exceeds destination STRING capacity".into());
    }
    Ok((
        DataType::String { capacity },
        CanonicalValue::StringBytes(decoded.into_bytes()),
    ))
}

fn decode_quoted(text: &str) -> Result<String, String> {
    let inner = text
        .strip_prefix('\'')
        .and_then(|value| value.strip_suffix('\''))
        .ok_or_else(|| "quoted literal is missing delimiters".to_string())?;
    let mut decoded = String::new();
    let mut chars = inner.chars().peekable();
    while let Some(character) = chars.next() {
        if character == '\'' {
            if chars.next() == Some('\'') {
                decoded.push('\'');
            } else {
                return Err("single quote content must use doubled quote escaping".into());
            }
        } else {
            decoded.push(character);
        }
    }
    Ok(decoded)
}

fn time_literal(text: &str) -> Result<(DataType, CanonicalValue), String> {
    let (_, components) = text
        .split_once('#')
        .ok_or_else(|| "TIME literal requires T# or TIME# prefix".to_string())?;
    let mut offset = 0;
    let mut last_rank = 6_u8;
    let mut seen = BTreeSet::new();
    let mut total = 0_i64;
    while offset < components.len() {
        let number_start = offset;
        while components
            .as_bytes()
            .get(offset)
            .is_some_and(u8::is_ascii_digit)
        {
            offset += 1;
        }
        if number_start == offset {
            return Err("TIME component requires decimal digits".into());
        }
        let amount = components[number_start..offset]
            .parse::<i64>()
            .map_err(|_| "TIME component exceeds canonical range".to_string())?;
        let remaining = &components[offset..];
        let (unit, rank, multiplier) =
            if remaining.len() >= 2 && remaining[..2].eq_ignore_ascii_case("ms") {
                offset += 2;
                ("ms", 1, 1_i64)
            } else {
                let unit_byte = *components
                    .as_bytes()
                    .get(offset)
                    .ok_or_else(|| "TIME component requires a unit".to_string())?;
                offset += 1;
                match unit_byte.to_ascii_lowercase() {
                    b'd' => ("d", 5, 86_400_000_i64),
                    b'h' => ("h", 4, 3_600_000_i64),
                    b'm' => ("m", 3, 60_000_i64),
                    b's' => ("s", 2, 1_000_i64),
                    _ => return Err("TIME unit must be d, h, m, s, or ms".into()),
                }
            };
        if rank >= last_rank || !seen.insert(unit) {
            return Err("TIME components must be unique and in descending unit order".into());
        }
        last_rank = rank;
        total = total
            .checked_add(
                amount
                    .checked_mul(multiplier)
                    .ok_or_else(|| "TIME component multiplication overflow".to_string())?,
            )
            .ok_or_else(|| "TIME literal summation overflow".to_string())?;
    }
    Ok((DataType::Time, CanonicalValue::TimeMilliseconds(total)))
}

fn typed_literal(
    text: &str,
    expected: Option<&DataType>,
) -> Result<(DataType, CanonicalValue), String> {
    let (prefix, payload) = text
        .split_once('#')
        .ok_or_else(|| "typed literal requires a registered type prefix".to_string())?;
    if prefix.eq_ignore_ascii_case("BOOL") {
        if payload.eq_ignore_ascii_case("TRUE") {
            Ok((DataType::Bool, CanonicalValue::Bool(true)))
        } else if payload.eq_ignore_ascii_case("FALSE") {
            Ok((DataType::Bool, CanonicalValue::Bool(false)))
        } else {
            Err("BOOL typed literal requires TRUE or FALSE".into())
        }
    } else if prefix.eq_ignore_ascii_case("INT") {
        integer_literal(payload, Some(&DataType::Int))
    } else if prefix.eq_ignore_ascii_case("DINT") {
        integer_literal(payload, Some(&DataType::DInt))
    } else if prefix.eq_ignore_ascii_case("SINT") {
        integer_literal(payload, Some(&DataType::SInt))
    } else if prefix.eq_ignore_ascii_case("LINT") {
        integer_literal(payload, Some(&DataType::LInt))
    } else if prefix.eq_ignore_ascii_case("USINT") {
        integer_literal(payload, Some(&DataType::USInt))
    } else if prefix.eq_ignore_ascii_case("UINT") {
        integer_literal(payload, Some(&DataType::UInt))
    } else if prefix.eq_ignore_ascii_case("UDINT") {
        integer_literal(payload, Some(&DataType::UDInt))
    } else if prefix.eq_ignore_ascii_case("ULINT") {
        integer_literal(payload, Some(&DataType::ULInt))
    } else if prefix.eq_ignore_ascii_case("REAL") {
        real_literal(payload)
    } else if prefix.eq_ignore_ascii_case("CHAR") {
        quoted_literal(payload, Some(&DataType::Char))
    } else if prefix.eq_ignore_ascii_case("STRING") {
        quoted_literal(payload, expected)
    } else {
        Err("typed literal prefix is not in the canonical registry".into())
    }
}
