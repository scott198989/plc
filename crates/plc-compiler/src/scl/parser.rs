use alloc::{boxed::Box, string::String, vec::Vec};

use crate::{DiagnosticCode, ResourceLimit, ResourceLimits, SclSource, SemanticNodeId, TextRange};

use super::{LexedSource, SclIssue, Token, TokenKind, Trivia, lex_scl};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MissingToken {
    pub expected: TokenKind,
    pub at: u32,
}

/// A source-preserving parse of one canonical SCL body.
///
/// [`Statement::parsed_only`] and [`Expr::parsed_only`] retain compound and
/// postfix grammar without losing source shape. The semantic pipeline consumes
/// its explicitly supported compound nodes; every other parsed-only form carries
/// a blocking `RECOGNIZED_UNSUPPORTED_SYNTAX` issue.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SyntaxTree {
    source: SclSource,
    tokens: Vec<Token>,
    trivia: Vec<Trivia>,
    pub(crate) statements: Vec<Statement>,
    missing_tokens: Vec<MissingToken>,
    issues: Vec<SclIssue>,
    resource_limit: Option<ResourceLimit>,
}

impl SyntaxTree {
    #[must_use]
    pub const fn source(&self) -> &SclSource {
        &self.source
    }

    #[must_use]
    pub fn tokens(&self) -> &[Token] {
        &self.tokens
    }

    #[must_use]
    pub fn trivia(&self) -> &[Trivia] {
        &self.trivia
    }

    #[must_use]
    pub fn statements(&self) -> &[Statement] {
        &self.statements
    }

    #[must_use]
    pub fn missing_tokens(&self) -> &[MissingToken] {
        &self.missing_tokens
    }

    #[must_use]
    pub fn issues(&self) -> &[SclIssue] {
        &self.issues
    }

    #[must_use]
    pub const fn resource_limit(&self) -> Option<&ResourceLimit> {
        self.resource_limit.as_ref()
    }
}

#[must_use]
pub fn parse_scl(source: &SclSource, limits: ResourceLimits) -> SyntaxTree {
    let lexed = lex_scl(source, limits);
    Parser::new(&lexed, limits).run()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Statement {
    pub id: SemanticNodeId,
    pub range: TextRange,
    /// The simple-statement subset. Compound statements are source-preserved in
    /// `parsed_only` and admitted individually by the semantic pipeline.
    pub kind: StatementKind,
    /// Fully parsed compound or unsupported syntax retained for semantic routing.
    pub parsed_only: Option<ParsedOnlyStatement>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StatementKind {
    Assignment {
        target: Name,
        value: Expr,
    },
    If {
        branches: Vec<(Expr, Vec<Statement>)>,
        else_body: Vec<Statement>,
    },
    Call {
        callee: Name,
        arguments: Vec<CallArgument>,
    },
    Return,
    Error,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ParsedOnlyStatement {
    Assignment {
        target: Designator,
        value: Expr,
    },
    Call {
        callee: Designator,
        arguments: Vec<ParsedCallArgument>,
    },
    Case {
        selector: Expr,
        arms: Vec<CaseArm>,
        else_body: Vec<Statement>,
    },
    For {
        iterator: Name,
        initial: Expr,
        limit: Expr,
        step: Option<Expr>,
        body: Vec<Statement>,
    },
    While {
        condition: Expr,
        body: Vec<Statement>,
    },
    Repeat {
        body: Vec<Statement>,
        condition: Expr,
    },
    Exit,
    Continue,
    UnsupportedDeclaration {
        introducer: Name,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CaseArm {
    pub range: TextRange,
    pub labels: Vec<CaseLabel>,
    pub body: Vec<Statement>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CaseLabel {
    pub range: TextRange,
    pub lower: Expr,
    pub upper: Option<Expr>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CallArgument {
    pub formal: Name,
    pub range: TextRange,
    pub actual: CallActual,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CallActual {
    Input(Expr),
    Output(Name),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParsedCallArgument {
    pub range: TextRange,
    pub binding: Option<CallBinding>,
    pub value: Expr,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CallBinding {
    /// `:=`; Input versus `InOut` is resolved from the canonical callee signature.
    Assign(Name),
    /// `=>`; the associated value must be an lvalue.
    Output(Name),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Designator {
    pub range: TextRange,
    pub root: Name,
    pub suffixes: Vec<DesignatorSuffix>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DesignatorSuffix {
    Member(Name),
    Index {
        range: TextRange,
        indices: Vec<Expr>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Name {
    pub spelling: String,
    pub range: TextRange,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Expr {
    pub id: SemanticNodeId,
    pub range: TextRange,
    /// The executable subset understood by the current semantic pipeline.
    pub kind: ExprKind,
    /// Postfix syntax retained while semantic support remains fail-closed.
    pub parsed_only: Option<ParsedOnlyExpression>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ParsedOnlyExpression {
    Postfix {
        base: Box<Expr>,
        suffixes: Vec<PostfixSuffix>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PostfixSuffix {
    Member(Name),
    Index {
        range: TextRange,
        indices: Vec<Expr>,
    },
    Call {
        range: TextRange,
        arguments: Vec<ParsedCallArgument>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExprKind {
    Literal(Literal),
    Name(Name),
    Unary {
        operator: UnaryOp,
        operand: Box<Expr>,
    },
    Binary {
        operator: BinaryOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },
    Error,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Literal {
    Bool(bool),
    Integer(String),
    Real(String),
    /// A single-quoted literal whose CHAR/STRING meaning is contextual.
    Quoted(String),
    Time(String),
    Typed(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnaryOp {
    Plus,
    Minus,
    Not,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinaryOp {
    Multiply,
    Divide,
    Modulo,
    Add,
    Subtract,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    And,
    Xor,
    Or,
}

struct Parser {
    source: SclSource,
    tokens: Vec<Token>,
    trivia: Vec<Trivia>,
    index: usize,
    compound_depth: usize,
    next_node: u32,
    node_count: usize,
    limits: ResourceLimits,
    statements: Vec<Statement>,
    missing_tokens: Vec<MissingToken>,
    issues: Vec<SclIssue>,
    resource_limit: Option<ResourceLimit>,
}

impl Parser {
    fn new(lexed: &LexedSource, limits: ResourceLimits) -> Self {
        Self {
            source: lexed.source().clone(),
            tokens: lexed.tokens().to_vec(),
            trivia: lexed.trivia().to_vec(),
            index: 0,
            compound_depth: 0,
            next_node: 1,
            node_count: 0,
            limits,
            statements: Vec::new(),
            missing_tokens: Vec::new(),
            issues: lexed.issues().to_vec(),
            resource_limit: lexed.resource_limit().cloned(),
        }
    }

    fn run(mut self) -> SyntaxTree {
        if self.resource_limit.is_none() {
            self.statements = self.parse_statements_until(&[TokenKind::Eof], &[]);
        }
        SyntaxTree {
            source: self.source,
            tokens: self.tokens,
            trivia: self.trivia,
            statements: self.statements,
            missing_tokens: self.missing_tokens,
            issues: self.issues,
            resource_limit: self.resource_limit,
        }
    }

    fn parse_statements_until(
        &mut self,
        stop: &[TokenKind],
        stop_keywords: &[&str],
    ) -> Vec<Statement> {
        let mut statements = Vec::new();
        while !stop.contains(&self.current().kind)
            && !stop_keywords.iter().any(|keyword| self.at_keyword(keyword))
            && self.current().kind != TokenKind::Eof
            && self.resource_limit.is_none()
        {
            let before = self.index;
            statements.push(self.parse_statement());
            if self.index == before {
                self.advance();
            }
        }
        statements
    }

    fn parse_statement(&mut self) -> Statement {
        match self.current().kind {
            TokenKind::Identifier => self.parse_assignment_or_call(),
            TokenKind::If => self.parse_if(),
            TokenKind::Return => self.parse_return(),
            TokenKind::UnsupportedKeyword if self.at_keyword("CASE") => self.parse_case(),
            TokenKind::UnsupportedKeyword if self.at_keyword("FOR") => self.parse_for(),
            TokenKind::UnsupportedKeyword if self.at_keyword("WHILE") => self.parse_while(),
            TokenKind::UnsupportedKeyword if self.at_keyword("REPEAT") => self.parse_repeat(),
            TokenKind::UnsupportedKeyword if self.at_keyword("EXIT") => {
                self.parse_loop_control(true)
            }
            TokenKind::UnsupportedKeyword if self.at_keyword("CONTINUE") => {
                self.parse_loop_control(false)
            }
            TokenKind::UnsupportedKeyword => self.parse_unsupported_statement(),
            _ => {
                let token = *self.current();
                self.issue(
                    DiagnosticCode::MALFORMED_STRUCTURE,
                    token.range,
                    None,
                    "expected an SCL body statement",
                );
                self.recover_statement();
                Statement {
                    id: self.node_id(),
                    range: token.range,
                    kind: StatementKind::Error,
                    parsed_only: None,
                }
            }
        }
    }

    fn parse_assignment_or_call(&mut self) -> Statement {
        let designator = self.parse_designator(0);
        if self.current().kind == TokenKind::LeftParen {
            return self.finish_call_statement(designator);
        }

        let id = self.node_id();
        if !self.consume(TokenKind::Assign) {
            self.missing(TokenKind::Assign, "assignment requires ':='");
        }
        let value = self.parse_expression(0, 0);
        let end = self.require_semicolon();
        let range = TextRange {
            start: designator.range.start,
            end,
        };
        if designator.suffixes.is_empty() {
            Statement {
                id,
                range,
                kind: StatementKind::Assignment {
                    target: designator.root,
                    value,
                },
                parsed_only: None,
            }
        } else {
            self.unsupported(
                range,
                id,
                "member and indexed assignment targets are parsed but not yet lowered",
            );
            Statement {
                id,
                range,
                kind: StatementKind::Error,
                parsed_only: Some(ParsedOnlyStatement::Assignment {
                    target: designator,
                    value,
                }),
            }
        }
    }

    fn finish_call_statement(&mut self, callee: Designator) -> Statement {
        let id = self.node_id();
        let arguments = self.parse_call_arguments(1);
        let end = self.require_semicolon();
        let range = TextRange {
            start: callee.range.start,
            end,
        };
        if let (Some(simple_callee), Some(simple_arguments)) = (
            simple_designator(&callee),
            lowerable_call_arguments(&arguments),
        ) {
            Statement {
                id,
                range,
                kind: StatementKind::Call {
                    callee: simple_callee,
                    arguments: simple_arguments,
                },
                parsed_only: None,
            }
        } else {
            self.unsupported(
                range,
                id,
                "positional, postfix, or expression-valued output calls are parsed but not yet lowered",
            );
            Statement {
                id,
                range,
                kind: StatementKind::Error,
                parsed_only: Some(ParsedOnlyStatement::Call { callee, arguments }),
            }
        }
    }

    fn parse_designator(&mut self, depth: usize) -> Designator {
        let token = *self.current();
        let root = if token.kind == TokenKind::Identifier {
            self.advance();
            self.name(token)
        } else {
            self.missing(TokenKind::Identifier, "designator requires an identifier");
            Name {
                spelling: String::new(),
                range: TextRange::empty(token.range.start),
            }
        };
        let mut end = root.range.end;
        let mut suffixes = Vec::new();
        loop {
            if self.consume(TokenKind::Dot) {
                let member_token = *self.current();
                if member_token.kind == TokenKind::Identifier {
                    self.advance();
                    let member = self.name(member_token);
                    end = member.range.end;
                    self.account_syntax_node();
                    suffixes.push(DesignatorSuffix::Member(member));
                } else {
                    self.missing(
                        TokenKind::Identifier,
                        "member access requires an identifier",
                    );
                }
            } else if self.current().kind == TokenKind::LeftBracket {
                let (range, indices) = self.parse_index_suffix(depth + 1);
                end = range.end;
                suffixes.push(DesignatorSuffix::Index { range, indices });
            } else {
                break;
            }
        }
        Designator {
            range: TextRange {
                start: root.range.start,
                end,
            },
            root,
            suffixes,
        }
    }

    fn parse_index_suffix(&mut self, depth: usize) -> (TextRange, Vec<Expr>) {
        self.account_syntax_node();
        let start = self.advance().range.start;
        let mut indices = Vec::new();
        if self.current().kind == TokenKind::RightBracket {
            self.issue(
                DiagnosticCode::MALFORMED_STRUCTURE,
                self.current().range,
                None,
                "index suffix requires at least one expression",
            );
        } else {
            loop {
                indices.push(self.parse_expression(0, depth));
                if !self.consume(TokenKind::Comma) {
                    break;
                }
                if self.current().kind == TokenKind::RightBracket {
                    self.issue(
                        DiagnosticCode::MALFORMED_STRUCTURE,
                        self.current().range,
                        None,
                        "index suffix cannot end with a comma",
                    );
                    break;
                }
            }
        }
        let end = if self.consume(TokenKind::RightBracket) {
            self.previous_end()
        } else {
            self.missing(TokenKind::RightBracket, "index suffix requires ']'");
            indices.last().map_or(start, |index| index.range.end)
        };
        (TextRange { start, end }, indices)
    }

    fn parse_call_arguments(&mut self, depth: usize) -> Vec<ParsedCallArgument> {
        self.consume(TokenKind::LeftParen);
        let mut arguments = Vec::new();
        let mut named_seen = false;
        if self.current().kind != TokenKind::RightParen {
            loop {
                let start = self.current().range.start;
                let binding = if self.current().kind == TokenKind::Identifier
                    && matches!(self.peek_kind(1), TokenKind::Assign | TokenKind::BindOutput)
                {
                    let formal_token = self.advance();
                    let formal = self.name(formal_token);
                    if self.consume(TokenKind::Assign) {
                        Some(CallBinding::Assign(formal))
                    } else {
                        self.consume(TokenKind::BindOutput);
                        Some(CallBinding::Output(formal))
                    }
                } else {
                    None
                };
                if binding.is_some() {
                    named_seen = true;
                } else if named_seen {
                    self.issue(
                        DiagnosticCode::MALFORMED_STRUCTURE,
                        self.current().range,
                        None,
                        "a positional argument cannot follow a named argument",
                    );
                }
                let value = self.parse_expression(0, depth);
                if matches!(&binding, Some(CallBinding::Output(_))) && !expression_is_lvalue(&value)
                {
                    self.issue(
                        DiagnosticCode::MALFORMED_STRUCTURE,
                        value.range,
                        Some(value.id),
                        "an output binding requires an lvalue designator",
                    );
                }
                self.account_syntax_node();
                arguments.push(ParsedCallArgument {
                    range: TextRange {
                        start,
                        end: value.range.end,
                    },
                    binding,
                    value,
                });
                if !self.consume(TokenKind::Comma) {
                    break;
                }
                if self.current().kind == TokenKind::RightParen {
                    self.issue(
                        DiagnosticCode::MALFORMED_STRUCTURE,
                        self.current().range,
                        None,
                        "call argument list cannot end with a comma",
                    );
                    break;
                }
            }
        }
        if !self.consume(TokenKind::RightParen) {
            self.missing(TokenKind::RightParen, "call argument list requires ')'");
            self.recover_call_argument_list();
            self.consume(TokenKind::RightParen);
        }
        arguments
    }

    fn recover_call_argument_list(&mut self) {
        while !matches!(
            self.current().kind,
            TokenKind::RightParen | TokenKind::Semicolon | TokenKind::Eof
        ) {
            self.advance();
        }
    }

    fn parse_if(&mut self) -> Statement {
        let start_token = self.advance();
        let id = self.node_id();
        if !self.enter_compound() {
            return Self::depth_limited_statement(start_token, id);
        }
        let start = start_token.range.start;
        let mut branches = Vec::new();
        let condition = self.parse_expression(0, 0);
        if !self.consume(TokenKind::Then) {
            self.missing(TokenKind::Then, "IF condition requires THEN");
        }
        let body = self.parse_statements_until(
            &[
                TokenKind::Elsif,
                TokenKind::Else,
                TokenKind::EndIf,
                TokenKind::Eof,
            ],
            &[],
        );
        branches.push((condition, body));
        while self.consume(TokenKind::Elsif) {
            let condition = self.parse_expression(0, 0);
            if !self.consume(TokenKind::Then) {
                self.missing(TokenKind::Then, "ELSIF condition requires THEN");
            }
            let body = self.parse_statements_until(
                &[
                    TokenKind::Elsif,
                    TokenKind::Else,
                    TokenKind::EndIf,
                    TokenKind::Eof,
                ],
                &[],
            );
            branches.push((condition, body));
        }
        let else_body = if self.consume(TokenKind::Else) {
            self.parse_statements_until(&[TokenKind::EndIf, TokenKind::Eof], &[])
        } else {
            Vec::new()
        };
        if !self.consume(TokenKind::EndIf) {
            self.missing(TokenKind::EndIf, "IF statement requires END_IF");
        }
        let end = self.require_semicolon();
        self.leave_compound();
        Statement {
            id,
            range: TextRange { start, end },
            kind: StatementKind::If {
                branches,
                else_body,
            },
            parsed_only: None,
        }
    }

    fn parse_case(&mut self) -> Statement {
        let start_token = self.advance();
        let id = self.node_id();
        if !self.enter_compound() {
            return Self::depth_limited_statement(start_token, id);
        }
        let selector = self.parse_expression(0, 0);
        if !self.consume_keyword("OF") {
            self.missing_keyword("CASE selector requires OF");
        }
        let mut arms = Vec::new();
        while !matches!(self.current().kind, TokenKind::Else | TokenKind::Eof)
            && !self.at_keyword("END_CASE")
            && self.resource_limit.is_none()
        {
            let before = self.index;
            arms.push(self.parse_case_arm());
            if self.index == before {
                self.advance();
            }
        }
        let else_body = if self.consume(TokenKind::Else) {
            self.parse_statements_until(&[TokenKind::Eof], &["END_CASE"])
        } else {
            Vec::new()
        };
        if !self.consume_keyword("END_CASE") {
            self.missing_keyword("CASE statement requires END_CASE");
        }
        let end = self.require_semicolon();
        self.leave_compound();
        let range = TextRange {
            start: start_token.range.start,
            end,
        };
        Statement {
            id,
            range,
            kind: StatementKind::Error,
            parsed_only: Some(ParsedOnlyStatement::Case {
                selector,
                arms,
                else_body,
            }),
        }
    }

    fn parse_case_arm(&mut self) -> CaseArm {
        self.account_syntax_node();
        let start = self.current().range.start;
        let mut labels = Vec::new();
        loop {
            let lower = self.parse_expression(0, 0);
            let upper = if self.consume(TokenKind::DotDot) {
                Some(self.parse_expression(0, 0))
            } else {
                None
            };
            let end = upper
                .as_ref()
                .map_or(lower.range.end, |value| value.range.end);
            self.account_syntax_node();
            labels.push(CaseLabel {
                range: TextRange {
                    start: lower.range.start,
                    end,
                },
                lower,
                upper,
            });
            if !self.consume(TokenKind::Comma) {
                break;
            }
        }
        if !self.consume(TokenKind::Colon) {
            self.missing(TokenKind::Colon, "CASE arm labels require ':'");
        }
        let mut body = Vec::new();
        while !matches!(self.current().kind, TokenKind::Else | TokenKind::Eof)
            && !self.at_keyword("END_CASE")
            && !self.looks_like_case_arm_start()
            && self.resource_limit.is_none()
        {
            let before = self.index;
            body.push(self.parse_statement());
            if self.index == before {
                self.advance();
            }
        }
        CaseArm {
            range: TextRange {
                start,
                end: body
                    .last()
                    .map_or_else(|| self.previous_end(), |statement| statement.range.end),
            },
            labels,
            body,
        }
    }

    fn looks_like_case_arm_start(&self) -> bool {
        if !expression_start(self.current().kind) {
            return false;
        }
        let mut parentheses = 0_u32;
        let mut brackets = 0_u32;
        let mut cursor = self.index;
        while let Some(token) = self.tokens.get(cursor) {
            match token.kind {
                TokenKind::LeftParen => parentheses = parentheses.saturating_add(1),
                TokenKind::RightParen if parentheses > 0 => parentheses -= 1,
                TokenKind::LeftBracket => brackets = brackets.saturating_add(1),
                TokenKind::RightBracket if brackets > 0 => brackets -= 1,
                TokenKind::Colon if parentheses == 0 && brackets == 0 => return true,
                TokenKind::Assign
                | TokenKind::BindOutput
                | TokenKind::Semicolon
                | TokenKind::Else
                | TokenKind::Eof
                    if parentheses == 0 && brackets == 0 =>
                {
                    return false;
                }
                _ => {}
            }
            cursor += 1;
        }
        false
    }

    fn parse_for(&mut self) -> Statement {
        let start_token = self.advance();
        let id = self.node_id();
        if !self.enter_compound() {
            return Self::depth_limited_statement(start_token, id);
        }
        let iterator_token = *self.current();
        let iterator = if iterator_token.kind == TokenKind::Identifier {
            self.advance();
            self.name(iterator_token)
        } else {
            self.missing(TokenKind::Identifier, "FOR requires an iterator identifier");
            Name {
                spelling: String::new(),
                range: TextRange::empty(iterator_token.range.start),
            }
        };
        if !self.consume(TokenKind::Assign) {
            self.missing(TokenKind::Assign, "FOR iterator requires ':='");
        }
        let initial = self.parse_expression(0, 0);
        if !self.consume_keyword("TO") {
            self.missing_keyword("FOR requires TO");
        }
        let limit = self.parse_expression(0, 0);
        let step = if self.consume_keyword("BY") {
            Some(self.parse_expression(0, 0))
        } else {
            None
        };
        if !self.consume_keyword("DO") {
            self.missing_keyword("FOR header requires DO");
        }
        let body = self.parse_statements_until(&[TokenKind::Eof], &["END_FOR"]);
        if !self.consume_keyword("END_FOR") {
            self.missing_keyword("FOR statement requires END_FOR");
        }
        let end = self.require_semicolon();
        self.leave_compound();
        let range = TextRange {
            start: start_token.range.start,
            end,
        };
        Statement {
            id,
            range,
            kind: StatementKind::Error,
            parsed_only: Some(ParsedOnlyStatement::For {
                iterator,
                initial,
                limit,
                step,
                body,
            }),
        }
    }

    fn parse_while(&mut self) -> Statement {
        let start_token = self.advance();
        let id = self.node_id();
        if !self.enter_compound() {
            return Self::depth_limited_statement(start_token, id);
        }
        let condition = self.parse_expression(0, 0);
        if !self.consume_keyword("DO") {
            self.missing_keyword("WHILE condition requires DO");
        }
        let body = self.parse_statements_until(&[TokenKind::Eof], &["END_WHILE"]);
        if !self.consume_keyword("END_WHILE") {
            self.missing_keyword("WHILE statement requires END_WHILE");
        }
        let end = self.require_semicolon();
        self.leave_compound();
        let range = TextRange {
            start: start_token.range.start,
            end,
        };
        Statement {
            id,
            range,
            kind: StatementKind::Error,
            parsed_only: Some(ParsedOnlyStatement::While { condition, body }),
        }
    }

    fn parse_repeat(&mut self) -> Statement {
        let start_token = self.advance();
        let id = self.node_id();
        if !self.enter_compound() {
            return Self::depth_limited_statement(start_token, id);
        }
        let body = self.parse_statements_until(&[TokenKind::Eof], &["UNTIL", "END_REPEAT"]);
        let condition = if self.consume_keyword("UNTIL") {
            self.parse_expression(0, 0)
        } else {
            self.missing_keyword("REPEAT statement requires UNTIL");
            self.error_expr(self.current().range)
        };
        if !self.consume_keyword("END_REPEAT") {
            self.missing_keyword("REPEAT statement requires END_REPEAT");
        }
        let end = self.require_semicolon();
        self.leave_compound();
        let range = TextRange {
            start: start_token.range.start,
            end,
        };
        Statement {
            id,
            range,
            kind: StatementKind::Error,
            parsed_only: Some(ParsedOnlyStatement::Repeat { body, condition }),
        }
    }

    fn parse_loop_control(&mut self, exit: bool) -> Statement {
        let token = self.advance();
        let id = self.node_id();
        let end = self.require_semicolon();
        let range = TextRange {
            start: token.range.start,
            end,
        };
        Statement {
            id,
            range,
            kind: StatementKind::Error,
            parsed_only: Some(if exit {
                ParsedOnlyStatement::Exit
            } else {
                ParsedOnlyStatement::Continue
            }),
        }
    }

    fn parse_return(&mut self) -> Statement {
        let token = self.advance();
        let id = self.node_id();
        let end = self.require_semicolon();
        Statement {
            id,
            range: TextRange {
                start: token.range.start,
                end,
            },
            kind: StatementKind::Return,
            parsed_only: None,
        }
    }

    fn parse_unsupported_statement(&mut self) -> Statement {
        let token = self.advance();
        let introducer = self.name(token);
        let id = self.node_id();
        let terminator = unsupported_terminator(&introducer.spelling);
        if let Some(terminator) = terminator {
            while self.current().kind != TokenKind::Eof {
                if self.current().kind == TokenKind::UnsupportedKeyword
                    && self
                        .text(self.current().range)
                        .eq_ignore_ascii_case(terminator)
                {
                    self.advance();
                    self.consume(TokenKind::Semicolon);
                    break;
                }
                self.advance();
            }
        } else {
            self.recover_statement();
        }
        let range = TextRange {
            start: token.range.start,
            end: self.previous_end(),
        };
        self.unsupported(
            range,
            id,
            "declarations and complete vendor program units are recognized but unsupported in an SCL body",
        );
        Statement {
            id,
            range,
            kind: StatementKind::Error,
            parsed_only: Some(ParsedOnlyStatement::UnsupportedDeclaration { introducer }),
        }
    }

    fn parse_expression(&mut self, minimum_binding_power: u8, depth: usize) -> Expr {
        let absolute_depth = self.compound_depth.saturating_add(depth);
        if absolute_depth > self.limits.max_syntax_depth {
            self.resource_limit = Some(ResourceLimit {
                key: "scl.syntax_depth",
                current: absolute_depth as u64,
                maximum: self.limits.max_syntax_depth as u64,
            });
            return self.error_expr(self.current().range);
        }
        let mut left = self.parse_prefix(depth + 1);
        let mut comparison_seen = false;
        loop {
            let Some((operator, left_power, right_power, comparison)) = infix(self.current().kind)
            else {
                break;
            };
            if left_power < minimum_binding_power {
                break;
            }
            let operator_token = self.advance();
            if comparison && comparison_seen {
                self.issue(
                    DiagnosticCode::MALFORMED_STRUCTURE,
                    operator_token.range,
                    Some(left.id),
                    "chained comparisons are not legal SCL expressions",
                );
            }
            comparison_seen |= comparison;
            let right = self.parse_expression(right_power, depth + 1);
            let range = TextRange {
                start: left.range.start,
                end: right.range.end,
            };
            left = Expr {
                id: self.node_id(),
                range,
                kind: ExprKind::Binary {
                    operator,
                    left: Box::new(left),
                    right: Box::new(right),
                },
                parsed_only: None,
            };
        }
        left
    }

    fn parse_prefix(&mut self, depth: usize) -> Expr {
        let token = *self.current();
        let unary = match token.kind {
            TokenKind::Plus => Some(UnaryOp::Plus),
            TokenKind::Minus => Some(UnaryOp::Minus),
            TokenKind::Not => Some(UnaryOp::Not),
            _ => None,
        };
        if let Some(operator) = unary {
            self.advance();
            let operand = self.parse_expression(13, depth + 1);
            return Expr {
                id: self.node_id(),
                range: TextRange {
                    start: token.range.start,
                    end: operand.range.end,
                },
                kind: ExprKind::Unary {
                    operator,
                    operand: Box::new(operand),
                },
                parsed_only: None,
            };
        }
        let primary = self.parse_primary(depth);
        self.parse_postfix(primary, depth)
    }

    fn parse_primary(&mut self, depth: usize) -> Expr {
        let token = *self.current();
        match token.kind {
            TokenKind::True | TokenKind::False => {
                self.advance();
                Expr {
                    id: self.node_id(),
                    range: token.range,
                    kind: ExprKind::Literal(Literal::Bool(token.kind == TokenKind::True)),
                    parsed_only: None,
                }
            }
            TokenKind::IntegerLiteral
            | TokenKind::RealLiteral
            | TokenKind::QuotedLiteral
            | TokenKind::TimeLiteral
            | TokenKind::TypedLiteral => {
                self.advance();
                let text = self.text(token.range).into();
                let literal = match token.kind {
                    TokenKind::IntegerLiteral => Literal::Integer(text),
                    TokenKind::RealLiteral => Literal::Real(text),
                    TokenKind::QuotedLiteral => Literal::Quoted(text),
                    TokenKind::TimeLiteral => Literal::Time(text),
                    TokenKind::TypedLiteral => Literal::Typed(text),
                    _ => unreachable!(),
                };
                Expr {
                    id: self.node_id(),
                    range: token.range,
                    kind: ExprKind::Literal(literal),
                    parsed_only: None,
                }
            }
            TokenKind::Identifier => {
                self.advance();
                Expr {
                    id: self.node_id(),
                    range: token.range,
                    kind: ExprKind::Name(self.name(token)),
                    parsed_only: None,
                }
            }
            TokenKind::LeftParen => {
                let start = self.advance().range.start;
                let mut expression = self.parse_expression(0, depth + 1);
                let end = if self.consume(TokenKind::RightParen) {
                    self.previous_end()
                } else {
                    self.missing(
                        TokenKind::RightParen,
                        "parenthesized expression requires ')'",
                    );
                    expression.range.end
                };
                expression.range = TextRange { start, end };
                expression
            }
            _ => {
                self.issue(
                    DiagnosticCode::MALFORMED_STRUCTURE,
                    token.range,
                    None,
                    "expected an expression",
                );
                if token.kind != TokenKind::Eof {
                    self.advance();
                }
                self.error_expr(token.range)
            }
        }
    }

    fn parse_postfix(&mut self, base: Expr, depth: usize) -> Expr {
        let mut suffixes = Vec::new();
        let mut end = base.range.end;
        loop {
            if self.consume(TokenKind::Dot) {
                let member_token = *self.current();
                if member_token.kind == TokenKind::Identifier {
                    self.advance();
                    let member = self.name(member_token);
                    end = member.range.end;
                    self.account_syntax_node();
                    suffixes.push(PostfixSuffix::Member(member));
                } else {
                    self.missing(
                        TokenKind::Identifier,
                        "member access requires an identifier",
                    );
                }
            } else if self.current().kind == TokenKind::LeftBracket {
                let (range, indices) = self.parse_index_suffix(depth + 1);
                end = range.end;
                suffixes.push(PostfixSuffix::Index { range, indices });
            } else if self.current().kind == TokenKind::LeftParen {
                let start = self.current().range.start;
                let arguments = self.parse_call_arguments(depth + 1);
                end = self.previous_end();
                self.account_syntax_node();
                suffixes.push(PostfixSuffix::Call {
                    range: TextRange { start, end },
                    arguments,
                });
            } else {
                break;
            }
        }
        if suffixes.is_empty() {
            base
        } else {
            let id = self.node_id();
            let range = TextRange {
                start: base.range.start,
                end,
            };
            self.unsupported(
                range,
                id,
                "member, index, and call postfix expressions are parsed but not yet lowered",
            );
            Expr {
                id,
                range,
                kind: ExprKind::Error,
                parsed_only: Some(ParsedOnlyExpression::Postfix {
                    base: Box::new(base),
                    suffixes,
                }),
            }
        }
    }

    fn error_expr(&mut self, range: TextRange) -> Expr {
        Expr {
            id: self.node_id(),
            range,
            kind: ExprKind::Error,
            parsed_only: None,
        }
    }

    fn enter_compound(&mut self) -> bool {
        let next = self.compound_depth.saturating_add(1);
        if next > self.limits.max_syntax_depth {
            self.resource_limit = Some(ResourceLimit {
                key: "scl.syntax_depth",
                current: next as u64,
                maximum: self.limits.max_syntax_depth as u64,
            });
            false
        } else {
            self.compound_depth = next;
            true
        }
    }

    fn leave_compound(&mut self) {
        self.compound_depth = self.compound_depth.saturating_sub(1);
    }

    fn depth_limited_statement(token: Token, id: SemanticNodeId) -> Statement {
        Statement {
            id,
            range: token.range,
            kind: StatementKind::Error,
            parsed_only: None,
        }
    }

    fn require_semicolon(&mut self) -> u32 {
        if self.consume(TokenKind::Semicolon) {
            self.previous_end()
        } else {
            let at = self.current().range.start;
            self.missing(
                TokenKind::Semicolon,
                "every simple or compound statement requires a trailing semicolon",
            );
            at
        }
    }

    fn recover_statement(&mut self) {
        while self.current().kind != TokenKind::Eof {
            if self.consume(TokenKind::Semicolon) {
                break;
            }
            if matches!(
                self.current().kind,
                TokenKind::Elsif | TokenKind::Else | TokenKind::EndIf
            ) {
                break;
            }
            if self.current().kind == TokenKind::UnsupportedKeyword
                && is_recovery_keyword(self.text(self.current().range))
            {
                break;
            }
            self.advance();
        }
    }

    fn missing(&mut self, expected: TokenKind, cause: &'static str) {
        let at = self.current().range.start;
        self.account_syntax_node();
        self.missing_tokens.push(MissingToken { expected, at });
        self.issue(
            DiagnosticCode::MALFORMED_STRUCTURE,
            TextRange::empty(at),
            None,
            cause,
        );
    }

    fn missing_keyword(&mut self, cause: &'static str) {
        self.missing(TokenKind::UnsupportedKeyword, cause);
    }

    fn unsupported(&mut self, range: TextRange, id: SemanticNodeId, cause: &'static str) {
        self.issue(
            DiagnosticCode::RECOGNIZED_UNSUPPORTED_SYNTAX,
            range,
            Some(id),
            cause,
        );
    }

    fn issue(
        &mut self,
        code: DiagnosticCode,
        range: TextRange,
        semantic_node: Option<SemanticNodeId>,
        cause: impl Into<String>,
    ) {
        if self.issues.len() < self.limits.max_diagnostics {
            self.issues.push(SclIssue {
                code,
                range,
                semantic_node,
                cause: cause.into(),
            });
        } else if self.resource_limit.is_none() {
            self.resource_limit = Some(ResourceLimit {
                key: "compiler.diagnostics",
                current: (self.issues.len() + 1) as u64,
                maximum: self.limits.max_diagnostics as u64,
            });
        }
    }

    fn node_id(&mut self) -> SemanticNodeId {
        self.account_syntax_node();
        let id = SemanticNodeId::new(self.next_node);
        self.next_node = self.next_node.saturating_add(1);
        id
    }

    fn account_syntax_node(&mut self) {
        self.node_count += 1;
        if self.node_count > self.limits.max_syntax_nodes_per_block && self.resource_limit.is_none()
        {
            self.resource_limit = Some(ResourceLimit {
                key: "scl.syntax_nodes",
                current: self.node_count as u64,
                maximum: self.limits.max_syntax_nodes_per_block as u64,
            });
        }
    }

    fn current(&self) -> &Token {
        &self.tokens[self.index.min(self.tokens.len() - 1)]
    }

    fn peek_kind(&self, offset: usize) -> TokenKind {
        self.tokens
            .get(self.index.saturating_add(offset))
            .unwrap_or_else(|| self.tokens.last().expect("lexer always emits EOF"))
            .kind
    }

    fn advance(&mut self) -> Token {
        let token = *self.current();
        if self.index + 1 < self.tokens.len() {
            self.index += 1;
        }
        token
    }

    fn consume(&mut self, kind: TokenKind) -> bool {
        if self.current().kind == kind {
            self.advance();
            true
        } else {
            false
        }
    }

    fn at_keyword(&self, keyword: &str) -> bool {
        self.current().kind == TokenKind::UnsupportedKeyword
            && self
                .text(self.current().range)
                .eq_ignore_ascii_case(keyword)
    }

    fn consume_keyword(&mut self, keyword: &str) -> bool {
        if self.at_keyword(keyword) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn previous_end(&self) -> u32 {
        self.tokens
            .get(self.index.saturating_sub(1))
            .map_or(0, |token| token.range.end)
    }

    fn text(&self, range: TextRange) -> &str {
        self.source.range_text(range).unwrap_or("")
    }

    fn name(&self, token: Token) -> Name {
        Name {
            spelling: self.text(token.range).into(),
            range: token.range,
        }
    }
}

fn simple_designator(designator: &Designator) -> Option<Name> {
    designator
        .suffixes
        .is_empty()
        .then(|| designator.root.clone())
}

fn lowerable_call_arguments(arguments: &[ParsedCallArgument]) -> Option<Vec<CallArgument>> {
    arguments
        .iter()
        .map(|argument| {
            let (formal, actual) = match &argument.binding {
                Some(CallBinding::Assign(formal)) => {
                    (formal.clone(), CallActual::Input(argument.value.clone()))
                }
                Some(CallBinding::Output(formal)) => {
                    let ExprKind::Name(actual) = &argument.value.kind else {
                        return None;
                    };
                    if argument.value.parsed_only.is_some() {
                        return None;
                    }
                    (formal.clone(), CallActual::Output(actual.clone()))
                }
                None => return None,
            };
            Some(CallArgument {
                formal,
                range: argument.range,
                actual,
            })
        })
        .collect()
}

fn expression_is_lvalue(expression: &Expr) -> bool {
    if matches!(expression.kind, ExprKind::Name(_)) && expression.parsed_only.is_none() {
        return true;
    }
    let Some(ParsedOnlyExpression::Postfix { base, suffixes }) = &expression.parsed_only else {
        return false;
    };
    matches!(base.kind, ExprKind::Name(_))
        && base.parsed_only.is_none()
        && suffixes.iter().all(|suffix| {
            matches!(
                suffix,
                PostfixSuffix::Member(_) | PostfixSuffix::Index { .. }
            )
        })
}

fn unsupported_terminator(introducer: &str) -> Option<&'static str> {
    if introducer.eq_ignore_ascii_case("VAR")
        || introducer.eq_ignore_ascii_case("VAR_INPUT")
        || introducer.eq_ignore_ascii_case("VAR_OUTPUT")
        || introducer.eq_ignore_ascii_case("VAR_IN_OUT")
        || introducer.eq_ignore_ascii_case("VAR_TEMP")
        || introducer.eq_ignore_ascii_case("VAR_GLOBAL")
        || introducer.eq_ignore_ascii_case("VAR_EXTERNAL")
        || introducer.eq_ignore_ascii_case("VAR_STAT")
        || introducer.eq_ignore_ascii_case("VAR_CONSTANT")
        || introducer.eq_ignore_ascii_case("VAR_CONFIG")
        || introducer.eq_ignore_ascii_case("VAR_ACCESS")
    {
        Some("END_VAR")
    } else if introducer.eq_ignore_ascii_case("TYPE") {
        Some("END_TYPE")
    } else if introducer.eq_ignore_ascii_case("STRUCT") {
        Some("END_STRUCT")
    } else if introducer.eq_ignore_ascii_case("FUNCTION") {
        Some("END_FUNCTION")
    } else if introducer.eq_ignore_ascii_case("FUNCTION_BLOCK") {
        Some("END_FUNCTION_BLOCK")
    } else if introducer.eq_ignore_ascii_case("ORGANIZATION_BLOCK") {
        Some("END_ORGANIZATION_BLOCK")
    } else if introducer.eq_ignore_ascii_case("DATA_BLOCK") {
        Some("END_DATA_BLOCK")
    } else if introducer.eq_ignore_ascii_case("PROGRAM") {
        Some("END_PROGRAM")
    } else if introducer.eq_ignore_ascii_case("CONFIGURATION") {
        Some("END_CONFIGURATION")
    } else if introducer.eq_ignore_ascii_case("RESOURCE") {
        Some("END_RESOURCE")
    } else if introducer.eq_ignore_ascii_case("BEGIN") {
        Some("END_BLOCK")
    } else {
        None
    }
}

fn is_recovery_keyword(keyword: &str) -> bool {
    ["END_CASE", "END_FOR", "END_WHILE", "UNTIL", "END_REPEAT"]
        .iter()
        .any(|candidate| keyword.eq_ignore_ascii_case(candidate))
}

const fn expression_start(kind: TokenKind) -> bool {
    matches!(
        kind,
        TokenKind::Identifier
            | TokenKind::IntegerLiteral
            | TokenKind::RealLiteral
            | TokenKind::QuotedLiteral
            | TokenKind::TimeLiteral
            | TokenKind::TypedLiteral
            | TokenKind::True
            | TokenKind::False
            | TokenKind::Plus
            | TokenKind::Minus
            | TokenKind::Not
            | TokenKind::LeftParen
    )
}

const fn infix(kind: TokenKind) -> Option<(BinaryOp, u8, u8, bool)> {
    let value = match kind {
        TokenKind::Or => (BinaryOp::Or, 1, 2, false),
        TokenKind::Xor => (BinaryOp::Xor, 3, 4, false),
        TokenKind::And => (BinaryOp::And, 5, 6, false),
        TokenKind::Equal => (BinaryOp::Equal, 7, 8, true),
        TokenKind::NotEqual => (BinaryOp::NotEqual, 7, 8, true),
        TokenKind::Less => (BinaryOp::Less, 7, 8, true),
        TokenKind::LessEqual => (BinaryOp::LessEqual, 7, 8, true),
        TokenKind::Greater => (BinaryOp::Greater, 7, 8, true),
        TokenKind::GreaterEqual => (BinaryOp::GreaterEqual, 7, 8, true),
        TokenKind::Plus => (BinaryOp::Add, 9, 10, false),
        TokenKind::Minus => (BinaryOp::Subtract, 9, 10, false),
        TokenKind::Star => (BinaryOp::Multiply, 11, 12, false),
        TokenKind::Slash => (BinaryOp::Divide, 11, 12, false),
        TokenKind::Mod => (BinaryOp::Modulo, 11, 12, false),
        _ => return None,
    };
    Some(value)
}

#[cfg(test)]
mod tests {
    use plc_program::BlockId;

    use super::*;

    #[test]
    fn parser_recovers_missing_semicolon_and_preserves_following_statement() {
        let source = SclSource::new(BlockId::new(1), "a := 1 b := 2;");
        let tree = parse_scl(&source, ResourceLimits::default());
        assert_eq!(tree.source().text(), source.text());
        assert_eq!(tree.missing_tokens().len(), 1);
        assert_eq!(tree.statements().len(), 2);
    }

    #[test]
    fn precedence_and_compound_if_build_real_nodes() {
        let source = SclSource::new(
            BlockId::new(1),
            "IF a OR b AND NOT c THEN out := 2 + 3 * 4; ELSE out := 0; END_IF;",
        );
        let tree = parse_scl(&source, ResourceLimits::default());
        assert!(tree.issues().is_empty(), "{:?}", tree.issues());
        assert_eq!(tree.statements().len(), 1);
        assert!(matches!(
            tree.statements()[0].kind,
            StatementKind::If { .. }
        ));
    }
}
