use plc_compiler::{
    DiagnosticCode, ResourceLimits, SclSource,
    scl::{
        BinaryOp, Expr, ExprKind, ParsedOnlyExpression, ParsedOnlyStatement, PostfixSuffix,
        StatementKind, TokenKind, TriviaKind, lex_scl, parse_scl,
    },
};
use plc_program::BlockId;

fn source(text: &str) -> SclSource {
    SclSource::new(BlockId::new(700), text)
}

fn has_issue(tree: &plc_compiler::scl::SyntaxTree, code: DiagnosticCode) -> bool {
    tree.issues().iter().any(|issue| issue.code == code)
}

fn assignment_value(statement: &plc_compiler::scl::Statement) -> &Expr {
    let StatementKind::Assignment { value, .. } = &statement.kind else {
        panic!("expected an assignment, got {statement:?}");
    };
    value
}

#[test]
fn lexer_golden_preserves_spelling_ranges_comments_and_all_literal_categories() {
    let text = "// lead\r\nTrUe FALSE 0 2#101 8#17 16#Af 1.25 1E3 1.0e-2 'x' 'it''s' t#1D2h3m4s5ms TIME#250MS (* tail *)";
    let input = source(text);
    let lexed = lex_scl(&input, ResourceLimits::default());
    assert!(lexed.issues().is_empty(), "{:?}", lexed.issues());
    assert_eq!(lexed.source().text(), text);

    let kinds: Vec<_> = lexed.tokens().iter().map(|token| token.kind).collect();
    assert_eq!(
        kinds,
        vec![
            TokenKind::True,
            TokenKind::False,
            TokenKind::IntegerLiteral,
            TokenKind::IntegerLiteral,
            TokenKind::IntegerLiteral,
            TokenKind::IntegerLiteral,
            TokenKind::RealLiteral,
            TokenKind::RealLiteral,
            TokenKind::RealLiteral,
            TokenKind::QuotedLiteral,
            TokenKind::QuotedLiteral,
            TokenKind::TimeLiteral,
            TokenKind::TimeLiteral,
            TokenKind::Eof,
        ]
    );
    let spellings: Vec<_> = lexed.tokens()[..lexed.tokens().len() - 1]
        .iter()
        .map(|token| input.range_text(token.range).expect("token range"))
        .collect();
    assert_eq!(
        spellings,
        vec![
            "TrUe",
            "FALSE",
            "0",
            "2#101",
            "8#17",
            "16#Af",
            "1.25",
            "1E3",
            "1.0e-2",
            "'x'",
            "'it''s'",
            "t#1D2h3m4s5ms",
            "TIME#250MS",
        ]
    );
    let comments: Vec<_> = lexed
        .trivia()
        .iter()
        .filter(|trivia| {
            matches!(
                trivia.kind,
                TriviaKind::LineComment | TriviaKind::BlockComment
            )
        })
        .map(|trivia| input.range_text(trivia.range).expect("trivia range"))
        .collect();
    assert_eq!(comments, vec!["// lead", "(* tail *)"]);
}

#[test]
fn lexer_accepts_exact_canonical_non_time_type_prefix_registry() {
    let text = "BOOL#TRUE SINT#1 INT#12 DINT#16#FF LINT#1 USINT#1 UINT#1 UDINT#1 ULINT#1 BYTE#2#1 WORD#8#7 DWORD#16#F LWORD#16#FF REAL#1.0 LREAL#1e-2 CHAR#'A' STRING#'A''B'";
    let input = source(text);
    let lexed = lex_scl(&input, ResourceLimits::default());
    assert!(lexed.issues().is_empty(), "{:?}", lexed.issues());
    assert_eq!(
        lexed
            .tokens()
            .iter()
            .filter(|token| token.kind == TokenKind::TypedLiteral)
            .count(),
        17
    );

    let malformed = lex_scl(&source("INT#"), ResourceLimits::default());
    assert!(
        malformed
            .issues()
            .iter()
            .any(|issue| issue.code == DiagnosticCode::MALFORMED_TOKEN)
    );
    let unsupported = lex_scl(&source("VENDOR#1"), ResourceLimits::default());
    assert!(
        unsupported
            .issues()
            .iter()
            .any(|issue| { issue.code == DiagnosticCode::RECOGNIZED_UNSUPPORTED_SYNTAX })
    );
}

#[test]
fn lexer_preserves_case_range_after_typed_integer_literals() {
    let input = source("DINT#2..DINT#4");
    let lexed = lex_scl(&input, ResourceLimits::default());
    assert!(lexed.issues().is_empty(), "{:?}", lexed.issues());
    assert_eq!(
        lexed
            .tokens()
            .iter()
            .map(|token| token.kind)
            .collect::<Vec<_>>(),
        vec![
            TokenKind::TypedLiteral,
            TokenKind::DotDot,
            TokenKind::TypedLiteral,
            TokenKind::Eof,
        ]
    );
    assert_eq!(
        lexed.tokens()[..3]
            .iter()
            .map(|token| input.range_text(token.range).expect("token range"))
            .collect::<Vec<_>>(),
        vec!["DINT#2", "..", "DINT#4"]
    );
}

#[test]
fn lexer_uses_longest_valid_operator_tokens() {
    let source = source(":= => <> <= >= .. . : = < >");
    let lexed = lex_scl(&source, ResourceLimits::default());
    let kinds: Vec<_> = lexed.tokens().iter().map(|token| token.kind).collect();
    assert_eq!(
        kinds,
        vec![
            TokenKind::Assign,
            TokenKind::BindOutput,
            TokenKind::NotEqual,
            TokenKind::LessEqual,
            TokenKind::GreaterEqual,
            TokenKind::DotDot,
            TokenKind::Dot,
            TokenKind::Colon,
            TokenKind::Equal,
            TokenKind::Less,
            TokenKind::Greater,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn complete_body_statement_grammar_preserves_executable_compound_nodes() {
    let text = r"
target := TRUE;
Block(In := 1, Out => result);
IF ready THEN target := FALSE; ELSIF retry THEN RETURN; ELSE target := TRUE; END_IF;
cAsE selector oF
    0, 1..3: result := 1;
    4: EXIT;
ELSE result := 0;
eNd_CaSe;
fOr Index := 0 tO 10 bY 2 dO CONTINUE; eNd_FoR;
wHiLe running Do result := result + 1; eNd_WhIlE;
rEpEaT result := result - 1; uNtIl result = 0 eNd_RePeAt;
RETURN;
";
    let tree = parse_scl(&source(text), ResourceLimits::default());
    assert!(
        !has_issue(&tree, DiagnosticCode::MALFORMED_TOKEN)
            && !has_issue(&tree, DiagnosticCode::MALFORMED_STRUCTURE),
        "valid body grammar was diagnosed as malformed: {:?}",
        tree.issues()
    );
    assert_eq!(tree.statements().len(), 8);
    assert!(matches!(
        tree.statements()[0].kind,
        StatementKind::Assignment { .. }
    ));
    assert!(matches!(
        tree.statements()[1].kind,
        StatementKind::Call { .. }
    ));
    assert!(matches!(
        tree.statements()[2].kind,
        StatementKind::If { .. }
    ));

    let Some(ParsedOnlyStatement::Case {
        arms, else_body, ..
    }) = &tree.statements()[3].parsed_only
    else {
        panic!("CASE surface was not retained");
    };
    assert_eq!(arms.len(), 2);
    assert_eq!(arms[0].labels.len(), 2);
    assert!(arms[0].labels[1].upper.is_some());
    assert!(matches!(
        arms[1].body[0].parsed_only,
        Some(ParsedOnlyStatement::Exit)
    ));
    assert_eq!(else_body.len(), 1);

    let Some(ParsedOnlyStatement::For { step, body, .. }) = &tree.statements()[4].parsed_only
    else {
        panic!("FOR surface was not retained");
    };
    assert!(step.is_some());
    assert!(matches!(
        body[0].parsed_only,
        Some(ParsedOnlyStatement::Continue)
    ));
    assert!(matches!(
        tree.statements()[5].parsed_only,
        Some(ParsedOnlyStatement::While { .. })
    ));
    assert!(matches!(
        tree.statements()[6].parsed_only,
        Some(ParsedOnlyStatement::Repeat { .. })
    ));
    assert!(matches!(tree.statements()[7].kind, StatementKind::Return));

    assert!(!has_issue(
        &tree,
        DiagnosticCode::RECOGNIZED_UNSUPPORTED_SYNTAX
    ));
    for statement in &tree.statements()[3..7] {
        assert!(matches!(statement.kind, StatementKind::Error));
        assert!(statement.parsed_only.is_some());
    }
}

#[test]
fn designators_calls_and_postfix_expressions_are_preserved_without_fake_semantics() {
    let text = "fb.member[1](x, In := arr[i], Out => obj.field); obj.field[index, j + 1] := Factory(1).Output;";
    let parsed_source = source(text);
    let tree = parse_scl(&parsed_source, ResourceLimits::default());
    assert!(!has_issue(&tree, DiagnosticCode::MALFORMED_STRUCTURE));
    assert_eq!(tree.statements().len(), 2);

    let Some(ParsedOnlyStatement::Call { callee, arguments }) = &tree.statements()[0].parsed_only
    else {
        panic!("complex call surface was not retained");
    };
    assert_eq!(callee.suffixes.len(), 2);
    assert_eq!(arguments.len(), 3);
    assert!(arguments[0].binding.is_none());
    assert!(arguments[1].value.parsed_only.is_some());
    assert!(arguments[2].value.parsed_only.is_some());

    let Some(ParsedOnlyStatement::Assignment { target, value }) = &tree.statements()[1].parsed_only
    else {
        panic!("complex assignment surface was not retained");
    };
    assert_eq!(target.suffixes.len(), 2);
    let Some(ParsedOnlyExpression::Postfix { suffixes, .. }) = &value.parsed_only else {
        panic!("postfix expression surface was not retained");
    };
    assert!(matches!(suffixes[0], PostfixSuffix::Call { .. }));
    assert!(matches!(suffixes[1], PostfixSuffix::Member(_)));

    for statement in tree.statements() {
        let retained = parsed_source
            .range_text(statement.range)
            .expect("statement source range");
        assert!(retained.ends_with(';'));
        assert!(matches!(statement.kind, StatementKind::Error));
    }
}

#[test]
fn precedence_is_canonical_and_binary_operators_are_left_associative() {
    let tree = parse_scl(
        &source("x := a OR b XOR c AND d = e + f * g; y := 10 - 3 - 2;"),
        ResourceLimits::default(),
    );
    assert!(tree.issues().is_empty(), "{:?}", tree.issues());

    let ExprKind::Binary {
        operator: BinaryOp::Or,
        right,
        ..
    } = &assignment_value(&tree.statements()[0]).kind
    else {
        panic!("OR was not the lowest-precedence root");
    };
    let ExprKind::Binary {
        operator: BinaryOp::Xor,
        right,
        ..
    } = &right.kind
    else {
        panic!("XOR precedence is wrong");
    };
    let ExprKind::Binary {
        operator: BinaryOp::And,
        right,
        ..
    } = &right.kind
    else {
        panic!("AND precedence is wrong");
    };
    let ExprKind::Binary {
        operator: BinaryOp::Equal,
        right,
        ..
    } = &right.kind
    else {
        panic!("comparison precedence is wrong");
    };
    assert!(matches!(
        right.kind,
        ExprKind::Binary {
            operator: BinaryOp::Add,
            ..
        }
    ));

    let ExprKind::Binary {
        operator: BinaryOp::Subtract,
        left,
        ..
    } = &assignment_value(&tree.statements()[1]).kind
    else {
        panic!("subtraction root missing");
    };
    assert!(matches!(
        left.kind,
        ExprKind::Binary {
            operator: BinaryOp::Subtract,
            ..
        }
    ));
}

#[test]
fn malformed_literals_and_chained_comparisons_are_rejected_deterministically() {
    for text in [
        "x := 2#102;",
        "x := 8#8;",
        "x := 16#G;",
        "x := 1.;",
        "x := 1e+;",
        "x := INT#-1;",
        "x := TIME#1s2s;",
        "x := TIME#1ms2s;",
        "x := 'first\nsecond';",
    ] {
        let tree = parse_scl(&source(text), ResourceLimits::default());
        assert!(
            has_issue(&tree, DiagnosticCode::MALFORMED_TOKEN),
            "expected malformed-token issue for {text:?}: {:?}",
            tree.issues()
        );
    }

    for text in [
        "x := a < b < c;",
        "x := TIME#1s-2ms;",
        "Call(Named := 1, 2);",
        "Call(Output => 1 + 2);",
    ] {
        let tree = parse_scl(&source(text), ResourceLimits::default());
        assert!(
            has_issue(&tree, DiagnosticCode::MALFORMED_STRUCTURE),
            "expected malformed-structure issue for {text:?}: {:?}",
            tree.issues()
        );
    }
}

#[test]
fn signed_and_default_for_steps_and_case_bound_expressions_use_normal_expressions() {
    let tree = parse_scl(
        &source(
            "FOR i := 10 TO 0 BY -1 DO EXIT; END_FOR; \
             FOR j := 0 TO upper DO CONTINUE; END_FOR; \
             CASE value OF -1..1 + 2 * 3: RETURN; END_CASE;",
        ),
        ResourceLimits::default(),
    );
    assert!(
        !has_issue(&tree, DiagnosticCode::MALFORMED_TOKEN)
            && !has_issue(&tree, DiagnosticCode::MALFORMED_STRUCTURE),
        "{:?}",
        tree.issues()
    );
    let Some(ParsedOnlyStatement::For {
        step: Some(step), ..
    }) = &tree.statements()[0].parsed_only
    else {
        panic!("negative FOR step was not retained");
    };
    assert!(matches!(step.kind, ExprKind::Unary { .. }));
    let Some(ParsedOnlyStatement::For { step: None, .. }) = &tree.statements()[1].parsed_only
    else {
        panic!("default FOR step was not retained");
    };
    let Some(ParsedOnlyStatement::Case { arms, .. }) = &tree.statements()[2].parsed_only else {
        panic!("CASE bound expression was not retained");
    };
    let upper = arms[0].labels[0]
        .upper
        .as_ref()
        .expect("CASE range upper bound");
    assert!(matches!(
        upper.kind,
        ExprKind::Binary {
            operator: BinaryOp::Add,
            ..
        }
    ));
}

#[test]
fn missing_semicolons_and_unsupported_declarations_recover_to_following_statements() {
    let tree = parse_scl(
        &source(
            "a := 1 b := 2; IF TRUE THEN a := 3; END_IF b := 4; \
             VAR_TEMP temp : INT; END_VAR; c := 5; \
             BEGIN hidden := 9; also_hidden := 10; END_BLOCK; d := 6;",
        ),
        ResourceLimits::default(),
    );
    assert_eq!(tree.missing_tokens().len(), 2, "{:?}", tree.issues());
    assert_eq!(tree.statements().len(), 8, "{:?}", tree.statements());
    assert!(matches!(
        tree.statements()[0].kind,
        StatementKind::Assignment { .. }
    ));
    assert!(matches!(
        tree.statements()[1].kind,
        StatementKind::Assignment { .. }
    ));
    assert!(matches!(
        tree.statements()[2].kind,
        StatementKind::If { .. }
    ));
    assert!(matches!(
        tree.statements()[3].kind,
        StatementKind::Assignment { .. }
    ));
    assert!(matches!(
        tree.statements()[4].parsed_only,
        Some(ParsedOnlyStatement::UnsupportedDeclaration { .. })
    ));
    assert!(matches!(
        tree.statements()[5].kind,
        StatementKind::Assignment { .. }
    ));
    assert!(matches!(
        tree.statements()[6].parsed_only,
        Some(ParsedOnlyStatement::UnsupportedDeclaration { .. })
    ));
    assert!(matches!(
        tree.statements()[7].kind,
        StatementKind::Assignment { .. }
    ));
}

#[test]
fn missing_compound_delimiters_produce_missing_tokens_without_losing_source() {
    for text in [
        "IF TRUE value := 1; END_IF;",
        "CASE value 1: RETURN; END_CASE;",
        "FOR i := 0 10 DO RETURN; END_FOR;",
        "WHILE TRUE RETURN; END_WHILE;",
        "REPEAT RETURN; END_REPEAT;",
    ] {
        let source = source(text);
        let tree = parse_scl(&source, ResourceLimits::default());
        assert_eq!(tree.source().text(), text);
        assert!(
            !tree.missing_tokens().is_empty(),
            "expected a missing-token node for {text:?}"
        );
        assert!(
            has_issue(&tree, DiagnosticCode::MALFORMED_STRUCTURE),
            "expected a blocking recovery diagnostic for {text:?}: {:?}",
            tree.issues()
        );
        assert!(tree.resource_limit().is_none());
    }
}

#[test]
fn identifiers_enforce_the_normative_128_byte_limit() {
    let accepted = "a".repeat(128);
    let accepted_source = source(&accepted);
    let accepted = lex_scl(&accepted_source, ResourceLimits::default());
    assert!(accepted.issues().is_empty());

    let rejected = "a".repeat(129);
    let rejected_source = source(&rejected);
    let rejected = lex_scl(&rejected_source, ResourceLimits::default());
    assert!(
        rejected
            .issues()
            .iter()
            .any(|issue| issue.code == DiagnosticCode::MALFORMED_TOKEN)
    );
}

#[test]
fn source_token_node_depth_and_diagnostic_limits_fail_closed() {
    let limits = ResourceLimits {
        max_source_bytes_per_block: 3,
        ..ResourceLimits::default()
    };
    let tree = parse_scl(&source("value := 1;"), limits);
    assert_eq!(
        tree.resource_limit().expect("source limit").key,
        "scl.source_bytes"
    );

    let limits = ResourceLimits {
        max_tokens_per_block: 2,
        ..ResourceLimits::default()
    };
    let tree = parse_scl(&source("a := 1;"), limits);
    assert_eq!(
        tree.resource_limit().expect("token limit").key,
        "scl.tokens"
    );

    let limits = ResourceLimits {
        max_syntax_nodes_per_block: 2,
        ..ResourceLimits::default()
    };
    let tree = parse_scl(&source("a := 1 + 2 + 3;"), limits);
    assert_eq!(
        tree.resource_limit().expect("node limit").key,
        "scl.syntax_nodes"
    );

    let limits = ResourceLimits {
        max_syntax_depth: 2,
        ..ResourceLimits::default()
    };
    let tree = parse_scl(&source("a := ((((1))));"), limits);
    assert_eq!(
        tree.resource_limit().expect("depth limit").key,
        "scl.syntax_depth"
    );

    let limits = ResourceLimits {
        max_diagnostics: 1,
        ..ResourceLimits::default()
    };
    let tree = parse_scl(&source("@; @;"), limits);
    assert_eq!(
        tree.resource_limit().expect("diagnostic limit").key,
        "compiler.diagnostics"
    );

    let limits = ResourceLimits {
        max_syntax_depth: 1,
        ..ResourceLimits::default()
    };
    let tree = parse_scl(
        &source("IF TRUE THEN IF TRUE THEN RETURN; END_IF; END_IF;"),
        limits,
    );
    assert_eq!(
        tree.resource_limit().expect("compound depth limit").key,
        "scl.syntax_depth"
    );
}
