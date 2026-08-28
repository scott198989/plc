use alloc::{string::String, vec::Vec};

use crate::{DiagnosticCode, ResourceLimit, ResourceLimits, SclSource, SemanticNodeId, TextRange};

use super::{LexedSource, SclIssue, Token, TokenKind, Trivia, lex_scl};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MissingToken {
    pub expected: TokenKind,
    pub at: u32,
}

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
pub(crate) struct Statement {
    pub id: SemanticNodeId,
    pub range: TextRange,
    pub kind: StatementKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum StatementKind {
    Assignment {
        target: Name,
        value: Expr,
    },
    If {
        branches: Vec<(Expr, Vec<Statement>)>,
        else_body: Vec<Statement>,
    },
    Return,
    Error,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Name {
    pub spelling: String,
    pub range: TextRange,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Expr {
    pub id: SemanticNodeId,
    pub range: TextRange,
    pub kind: ExprKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ExprKind {
    Literal(Literal),
    Name(Name),
    Unary {
        operator: UnaryOp,
        operand: alloc::boxed::Box<Expr>,
    },
    Binary {
        operator: BinaryOp,
        left: alloc::boxed::Box<Expr>,
        right: alloc::boxed::Box<Expr>,
    },
    Error,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Literal {
    Bool(bool),
    Integer(String),
    Real(String),
    Quoted(String),
    Time(String),
    Typed(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum UnaryOp {
    Plus,
    Minus,
    Not,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BinaryOp {
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
            self.statements = self.parse_statements_until(&[TokenKind::Eof]);
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

    fn parse_statements_until(&mut self, stop: &[TokenKind]) -> Vec<Statement> {
        let mut statements = Vec::new();
        while !stop.contains(&self.current().kind)
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
            TokenKind::Identifier => self.parse_assignment(),
            TokenKind::If => self.parse_if(),
            TokenKind::Return => self.parse_return(),
            TokenKind::UnsupportedKeyword => self.parse_unsupported_statement(),
            _ => {
                let token = *self.current();
                self.issue(
                    DiagnosticCode::MALFORMED_STRUCTURE,
                    token.range,
                    None,
                    "expected assignment, IF, or RETURN statement",
                );
                self.recover_statement();
                Statement {
                    id: self.node_id(),
                    range: token.range,
                    kind: StatementKind::Error,
                }
            }
        }
    }

    fn parse_assignment(&mut self) -> Statement {
        let name_token = self.advance();
        let name = Name {
            spelling: self.text(name_token.range).into(),
            range: name_token.range,
        };
        let id = self.node_id();
        if matches!(
            self.current().kind,
            TokenKind::Dot | TokenKind::LeftBracket | TokenKind::LeftParen
        ) {
            let start = name_token.range.start;
            self.issue(
                DiagnosticCode::RECOGNIZED_UNSUPPORTED_SYNTAX,
                TextRange {
                    start,
                    end: self.current().range.end,
                },
                Some(id),
                "member, index, and call designators are not yet in the initial SCL slice",
            );
            self.recover_statement();
            return Statement {
                id,
                range: TextRange {
                    start,
                    end: self.previous_end(),
                },
                kind: StatementKind::Error,
            };
        }
        if !self.consume(TokenKind::Assign) {
            self.missing(TokenKind::Assign, "assignment requires ':='");
        }
        let value = self.parse_expression(0, 0);
        let end = self.require_semicolon();
        Statement {
            id,
            range: TextRange {
                start: name.range.start,
                end,
            },
            kind: StatementKind::Assignment {
                target: name,
                value,
            },
        }
    }

    fn parse_if(&mut self) -> Statement {
        let start = self.advance().range.start;
        let id = self.node_id();
        let mut branches = Vec::new();
        let condition = self.parse_expression(0, 0);
        if !self.consume(TokenKind::Then) {
            self.missing(TokenKind::Then, "IF condition requires THEN");
        }
        let body = self.parse_statements_until(&[
            TokenKind::Elsif,
            TokenKind::Else,
            TokenKind::EndIf,
            TokenKind::Eof,
        ]);
        branches.push((condition, body));
        while self.consume(TokenKind::Elsif) {
            let condition = self.parse_expression(0, 0);
            if !self.consume(TokenKind::Then) {
                self.missing(TokenKind::Then, "ELSIF condition requires THEN");
            }
            let body = self.parse_statements_until(&[
                TokenKind::Elsif,
                TokenKind::Else,
                TokenKind::EndIf,
                TokenKind::Eof,
            ]);
            branches.push((condition, body));
        }
        let else_body = if self.consume(TokenKind::Else) {
            self.parse_statements_until(&[TokenKind::EndIf, TokenKind::Eof])
        } else {
            Vec::new()
        };
        if !self.consume(TokenKind::EndIf) {
            self.missing(TokenKind::EndIf, "IF statement requires END_IF");
        }
        let end = self.require_semicolon();
        Statement {
            id,
            range: TextRange { start, end },
            kind: StatementKind::If {
                branches,
                else_body,
            },
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
        }
    }

    fn parse_unsupported_statement(&mut self) -> Statement {
        let token = self.advance();
        let id = self.node_id();
        self.issue(
            DiagnosticCode::RECOGNIZED_UNSUPPORTED_SYNTAX,
            token.range,
            Some(id),
            "recognized baseline SCL construct is outside this initial vertical slice",
        );
        self.recover_statement();
        Statement {
            id,
            range: TextRange {
                start: token.range.start,
                end: self.previous_end(),
            },
            kind: StatementKind::Error,
        }
    }

    fn parse_expression(&mut self, minimum_binding_power: u8, depth: usize) -> Expr {
        if depth > self.limits.max_syntax_depth {
            self.resource_limit = Some(ResourceLimit {
                key: "scl.syntax_depth",
                current: depth as u64,
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
                    left: alloc::boxed::Box::new(left),
                    right: alloc::boxed::Box::new(right),
                },
            };
        }
        left
    }

    #[allow(clippy::too_many_lines)]
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
                    operand: alloc::boxed::Box::new(operand),
                },
            };
        }
        match token.kind {
            TokenKind::True | TokenKind::False => {
                self.advance();
                Expr {
                    id: self.node_id(),
                    range: token.range,
                    kind: ExprKind::Literal(Literal::Bool(token.kind == TokenKind::True)),
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
                }
            }
            TokenKind::Identifier => {
                self.advance();
                let id = self.node_id();
                if self.current().kind == TokenKind::LeftParen {
                    self.issue(
                        DiagnosticCode::RECOGNIZED_UNSUPPORTED_SYNTAX,
                        TextRange {
                            start: token.range.start,
                            end: self.current().range.end,
                        },
                        Some(id),
                        "intrinsic and block calls are not yet in the initial SCL slice",
                    );
                    self.skip_balanced_parentheses();
                    return Expr {
                        id,
                        range: TextRange {
                            start: token.range.start,
                            end: self.previous_end(),
                        },
                        kind: ExprKind::Error,
                    };
                }
                Expr {
                    id,
                    range: token.range,
                    kind: ExprKind::Name(Name {
                        spelling: self.text(token.range).into(),
                        range: token.range,
                    }),
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

    fn error_expr(&mut self, range: TextRange) -> Expr {
        Expr {
            id: self.node_id(),
            range,
            kind: ExprKind::Error,
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
        let mut nested_if = 0_u32;
        while self.current().kind != TokenKind::Eof {
            match self.current().kind {
                TokenKind::If => nested_if = nested_if.saturating_add(1),
                TokenKind::EndIf if nested_if > 0 => nested_if -= 1,
                TokenKind::Semicolon if nested_if == 0 => {
                    self.advance();
                    break;
                }
                TokenKind::Elsif | TokenKind::Else | TokenKind::EndIf if nested_if == 0 => break,
                _ => {}
            }
            self.advance();
        }
    }

    fn skip_balanced_parentheses(&mut self) {
        if !self.consume(TokenKind::LeftParen) {
            return;
        }
        let mut depth = 1_u32;
        while depth != 0 && self.current().kind != TokenKind::Eof {
            match self.advance().kind {
                TokenKind::LeftParen => depth = depth.saturating_add(1),
                TokenKind::RightParen => depth -= 1,
                _ => {}
            }
        }
    }

    fn missing(&mut self, expected: TokenKind, cause: &'static str) {
        let at = self.current().range.start;
        self.missing_tokens.push(MissingToken { expected, at });
        self.issue(
            DiagnosticCode::MALFORMED_STRUCTURE,
            TextRange::empty(at),
            None,
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
        self.node_count += 1;
        if self.node_count > self.limits.max_syntax_nodes_per_block && self.resource_limit.is_none()
        {
            self.resource_limit = Some(ResourceLimit {
                key: "scl.syntax_nodes",
                current: self.node_count as u64,
                maximum: self.limits.max_syntax_nodes_per_block as u64,
            });
        }
        let id = SemanticNodeId::new(self.next_node);
        self.next_node = self.next_node.saturating_add(1);
        id
    }

    fn current(&self) -> &Token {
        &self.tokens[self.index.min(self.tokens.len() - 1)]
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

    fn previous_end(&self) -> u32 {
        self.tokens
            .get(self.index.saturating_sub(1))
            .map_or(0, |token| token.range.end)
    }

    fn text(&self, range: TextRange) -> &str {
        self.source.range_text(range).unwrap_or("")
    }
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
        assert_eq!(tree.statements.len(), 2);
    }

    #[test]
    fn precedence_and_compound_if_build_real_nodes() {
        let source = SclSource::new(
            BlockId::new(1),
            "IF a OR b AND NOT c THEN out := 2 + 3 * 4; ELSE out := 0; END_IF;",
        );
        let tree = parse_scl(&source, ResourceLimits::default());
        assert!(tree.issues().is_empty());
        assert_eq!(tree.statements.len(), 1);
        assert!(matches!(tree.statements[0].kind, StatementKind::If { .. }));
    }
}
