use alloc::{
    collections::{BTreeMap, BTreeSet},
    string::{String, ToString},
    vec::Vec,
};

use plc_program::{
    BlockId, CanonicalValue, DataType, InterfaceMember, InterfaceMemberId, InterfaceRole,
    ProgramBlock, ProgramUnitKind,
};

use crate::{
    DiagnosticCode, ResourceLimit, ResourceLimits, SclSource, SemanticNodeId, SourceAnchor,
    SourceLanguage, TextRange,
};

use super::{
    BinaryOp, Expr, ExprKind, Literal, MissingToken, SclIssue, Statement, StatementKind,
    SyntaxTree, UnaryOp, parse_scl,
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
    pub access: SclAccessKind,
    pub resolution: SclOccurrenceResolution,
    pub member: Option<InterfaceMemberId>,
    pub data_type: Option<DataType>,
    pub role: Option<InterfaceRole>,
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
        bind_and_typecheck_with_occurrences(&tree, block);
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
    Return,
    Error,
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
    Return,
    Error,
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
    Member(TypedMember),
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

pub(crate) fn bind_and_typecheck(
    tree: &SyntaxTree,
    block: &ProgramBlock,
) -> (TypedBlock, Vec<SclIssue>) {
    let (typed, issues, _) = bind_and_typecheck_with_occurrences(tree, block);
    (typed, issues)
}

fn bind_and_typecheck_with_occurrences(
    tree: &SyntaxTree,
    block: &ProgramBlock,
) -> (TypedBlock, Vec<SclIssue>, Vec<SclSymbolOccurrence>) {
    let (bound, mut issues, occurrences) = bind(tree, block);
    let (typed, type_issues) = typecheck(&bound, block);
    issues.extend(type_issues);
    sort_issues(&mut issues);
    (typed, issues, occurrences)
}

fn bind(
    tree: &SyntaxTree,
    block: &ProgramBlock,
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
        issues: Vec::new(),
        occurrences: Vec::new(),
        source_owner: tree.source().owner(),
        source_revision: tree.source().revision_hash(),
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
    issues: Vec<SclIssue>,
    occurrences: Vec<SclSymbolOccurrence>,
    source_owner: BlockId,
    source_revision: crate::Hash32,
}

impl Binder<'_> {
    fn bind_statements(&mut self, statements: &[Statement]) -> Vec<BoundStatement> {
        statements
            .iter()
            .map(|statement| self.bind_statement(statement))
            .collect()
    }

    fn bind_statement(&mut self, statement: &Statement) -> BoundStatement {
        let kind = match &statement.kind {
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
            StatementKind::Return => BoundStatementKind::Return,
            StatementKind::Error => BoundStatementKind::Error,
        };
        BoundStatement {
            id: statement.id,
            range: statement.range,
            kind,
        }
    }

    fn bind_expr(&mut self, expression: &Expr) -> BoundExpr {
        let kind = match &expression.kind {
            ExprKind::Literal(literal) => BoundExprKind::Literal(literal.clone()),
            ExprKind::Name(name) => self
                .resolve(
                    &name.spelling,
                    name.range,
                    expression.id,
                    SclAccessKind::Read,
                )
                .map_or(BoundExprKind::Error, BoundExprKind::Member),
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
            source: SourceAnchor {
                owner_object_id: self.source_owner,
                source_revision_hash: self.source_revision,
                language: SourceLanguage::Scl,
                semantic_node_id: node,
                text_range: range,
            },
            spelling: spelling.into(),
            access,
            resolution,
            member: member.map(|value| value.id),
            data_type: member.map(|value| value.data_type.clone()),
            role: member.map(|value| value.role),
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
        if let StatementKind::If {
            branches,
            else_body,
        } = &statement.kind
        {
            ranges.push(statement.range);
            for (_, body) in branches {
                collect_folding_ranges(body, ranges);
            }
            collect_folding_ranges(else_body, ranges);
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
            TypedStatementKind::Return | TypedStatementKind::Error => {}
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
        source: SourceAnchor {
            owner_object_id: source.owner(),
            source_revision_hash: source.revision_hash(),
            language: SourceLanguage::Scl,
            semantic_node_id: expression.id,
            text_range: expression.range,
        },
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
            BoundExprKind::Member(member) => {
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
                    UnaryOp::Plus | UnaryOp::Minus => is_numeric(&operand.data_type),
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
                        if *operator == BinaryOp::Modulo && left.data_type == DataType::Real {
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
        BoundExprKind::Member(member) => Some(&member.data_type),
        BoundExprKind::Unary { operand, .. } => bound_type_hint(operand),
        BoundExprKind::Binary { left, right, .. } => {
            bound_type_hint(left).or_else(|| bound_type_hint(right))
        }
        BoundExprKind::Literal(_) | BoundExprKind::Error => None,
    }
}

fn is_numeric(data_type: &DataType) -> bool {
    matches!(data_type, DataType::Int | DataType::DInt | DataType::Real)
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
        Some(DataType::Int) => i16::try_from(value)
            .map(|value| (DataType::Int, CanonicalValue::Int(value)))
            .map_err(|_| "integer literal is outside INT range".to_string()),
        Some(DataType::DInt) | None => i32::try_from(value)
            .map(|value| (DataType::DInt, CanonicalValue::DInt(value)))
            .map_err(|_| "integer literal is outside DINT range".to_string()),
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
    } else if prefix.eq_ignore_ascii_case("REAL") {
        real_literal(payload)
    } else if prefix.eq_ignore_ascii_case("STRING") {
        quoted_literal(payload, expected)
    } else {
        Err("typed literal prefix is not in the canonical registry".into())
    }
}
