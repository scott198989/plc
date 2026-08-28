//! Source-preserving SCL frontend.
//!
//! Source is never executed as host code. The lexer and recovery parser retain
//! the exact source text and comments. Compound grammar is retained in explicit
//! parsed-only nodes; executable control-flow nodes are consumed by the typed
//! semantic pipeline, while unsupported nodes still emit blocking diagnostics.

mod lexer;
mod parser;
mod semantics;

pub use lexer::{LexedSource, Token, TokenKind, Trivia, TriviaKind, lex_scl};
pub use parser::{
    BinaryOp, CallActual, CallArgument, CallBinding, CaseArm, CaseLabel, Designator,
    DesignatorSuffix, Expr, ExprKind, Literal, MissingToken, Name, ParsedCallArgument,
    ParsedOnlyExpression, ParsedOnlyStatement, PostfixSuffix, Statement, StatementKind, SyntaxTree,
    UnaryOp, parse_scl,
};
pub use semantics::{
    SclAccessKind, SclOccurrenceResolution, SclSemanticSnapshot, SclSemanticSymbol,
    SclSymbolOccurrence, SclTypeFact, analyze_scl, analyze_scl_with_program,
};

pub(crate) use semantics::{
    TypedBlock, TypedCall, TypedCaseArm, TypedExpr, TypedExprKind, TypedMember, TypedStatement,
    TypedStatementKind, bind_and_typecheck_with_program,
};

use alloc::string::String;

use crate::{DiagnosticCode, SemanticNodeId, TextRange};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SclIssue {
    pub code: DiagnosticCode,
    pub range: TextRange,
    pub semantic_node: Option<SemanticNodeId>,
    pub cause: String,
}
