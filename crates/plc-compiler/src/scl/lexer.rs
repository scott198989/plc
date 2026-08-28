use alloc::{string::String, vec::Vec};

use crate::{DiagnosticCode, ResourceLimit, ResourceLimits, SclSource, TextRange};

use super::SclIssue;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TokenKind {
    Identifier,
    IntegerLiteral,
    RealLiteral,
    QuotedLiteral,
    TimeLiteral,
    TypedLiteral,
    True,
    False,
    If,
    Then,
    Elsif,
    Else,
    EndIf,
    Return,
    And,
    Xor,
    Or,
    Not,
    Mod,
    UnsupportedKeyword,
    Assign,
    BindOutput,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    Plus,
    Minus,
    Star,
    Slash,
    LeftParen,
    RightParen,
    LeftBracket,
    RightBracket,
    Comma,
    Dot,
    DotDot,
    Colon,
    Semicolon,
    Malformed,
    Eof,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub range: TextRange,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TriviaKind {
    Whitespace,
    LineComment,
    BlockComment,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Trivia {
    pub kind: TriviaKind,
    pub range: TextRange,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LexedSource {
    source: SclSource,
    tokens: Vec<Token>,
    trivia: Vec<Trivia>,
    issues: Vec<SclIssue>,
    resource_limit: Option<ResourceLimit>,
}

impl LexedSource {
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
    pub fn issues(&self) -> &[SclIssue] {
        &self.issues
    }

    #[must_use]
    pub const fn resource_limit(&self) -> Option<&ResourceLimit> {
        self.resource_limit.as_ref()
    }
}

#[must_use]
pub fn lex_scl(source: &SclSource, limits: ResourceLimits) -> LexedSource {
    Lexer::new(source.clone(), limits).run()
}

struct Lexer {
    source: SclSource,
    limits: ResourceLimits,
    offset: usize,
    tokens: Vec<Token>,
    trivia: Vec<Trivia>,
    issues: Vec<SclIssue>,
    resource_limit: Option<ResourceLimit>,
}

impl Lexer {
    fn new(source: SclSource, limits: ResourceLimits) -> Self {
        Self {
            source,
            limits,
            offset: 0,
            tokens: Vec::new(),
            trivia: Vec::new(),
            issues: Vec::new(),
            resource_limit: None,
        }
    }

    fn run(mut self) -> LexedSource {
        let source_len = self.source.text().len();
        if source_len > self.limits.max_source_bytes_per_block {
            self.resource_limit = Some(ResourceLimit {
                key: "scl.source_bytes",
                current: source_len as u64,
                maximum: self.limits.max_source_bytes_per_block as u64,
            });
        } else {
            while self.offset < source_len && self.resource_limit.is_none() {
                self.scan_one();
            }
        }
        let eof = u32::try_from(self.offset).unwrap_or(u32::MAX);
        self.tokens.push(Token {
            kind: TokenKind::Eof,
            range: TextRange::empty(eof),
        });
        LexedSource {
            source: self.source,
            tokens: self.tokens,
            trivia: self.trivia,
            issues: self.issues,
            resource_limit: self.resource_limit,
        }
    }

    #[allow(clippy::too_many_lines)]
    fn scan_one(&mut self) {
        if self.tokens.len() >= self.limits.max_tokens_per_block {
            self.resource_limit = Some(ResourceLimit {
                key: "scl.tokens",
                current: (self.tokens.len() + 1) as u64,
                maximum: self.limits.max_tokens_per_block as u64,
            });
            return;
        }
        let start = self.offset;
        let bytes = self.source.text().as_bytes();
        let byte = bytes[start];
        if byte.is_ascii_whitespace() {
            self.offset += 1;
            while self
                .source
                .text()
                .as_bytes()
                .get(self.offset)
                .is_some_and(u8::is_ascii_whitespace)
            {
                self.offset += 1;
            }
            self.push_trivia(TriviaKind::Whitespace, start, self.offset);
            return;
        }
        if self.starts_with(b"//") {
            self.offset += 2;
            while self
                .source
                .text()
                .as_bytes()
                .get(self.offset)
                .is_some_and(|value| *value != b'\n' && *value != b'\r')
            {
                self.offset += 1;
            }
            self.push_trivia(TriviaKind::LineComment, start, self.offset);
            return;
        }
        if self.starts_with(b"(*") {
            self.offset += 2;
            while self.offset < self.source.text().len() && !self.starts_with(b"*)") {
                self.offset += self.next_char_len();
            }
            if self.starts_with(b"*)") {
                self.offset += 2;
                self.push_trivia(TriviaKind::BlockComment, start, self.offset);
            } else {
                self.push_issue(
                    DiagnosticCode::MALFORMED_STRUCTURE,
                    start,
                    self.offset,
                    "unterminated non-nested block comment",
                );
                self.push_trivia(TriviaKind::BlockComment, start, self.offset);
            }
            return;
        }
        if byte.is_ascii_alphabetic() || byte == b'_' {
            self.scan_word_or_prefixed_literal(start);
            return;
        }
        if byte.is_ascii_digit() {
            self.scan_number(start);
            return;
        }
        if byte == b'\'' {
            self.scan_quoted(start, TokenKind::QuotedLiteral);
            return;
        }

        let (kind, width) = match byte {
            b':' if self.starts_with(b":=") => (TokenKind::Assign, 2),
            b'=' if self.starts_with(b"=>") => (TokenKind::BindOutput, 2),
            b'<' if self.starts_with(b"<>") => (TokenKind::NotEqual, 2),
            b'<' if self.starts_with(b"<=") => (TokenKind::LessEqual, 2),
            b'>' if self.starts_with(b">=") => (TokenKind::GreaterEqual, 2),
            b'.' if self.starts_with(b"..") => (TokenKind::DotDot, 2),
            b'=' => (TokenKind::Equal, 1),
            b'<' => (TokenKind::Less, 1),
            b'>' => (TokenKind::Greater, 1),
            b'+' => (TokenKind::Plus, 1),
            b'-' => (TokenKind::Minus, 1),
            b'*' => (TokenKind::Star, 1),
            b'/' => (TokenKind::Slash, 1),
            b'(' => (TokenKind::LeftParen, 1),
            b')' => (TokenKind::RightParen, 1),
            b'[' => (TokenKind::LeftBracket, 1),
            b']' => (TokenKind::RightBracket, 1),
            b',' => (TokenKind::Comma, 1),
            b'.' => (TokenKind::Dot, 1),
            b':' => (TokenKind::Colon, 1),
            b';' => (TokenKind::Semicolon, 1),
            _ => (TokenKind::Malformed, self.next_char_len()),
        };
        self.offset += width;
        self.push_token(kind, start, self.offset);
        if kind == TokenKind::Malformed {
            self.push_issue(
                DiagnosticCode::MALFORMED_TOKEN,
                start,
                self.offset,
                "unregistered character or token",
            );
        }
    }

    fn scan_word_or_prefixed_literal(&mut self, start: usize) {
        self.offset += 1;
        while self
            .source
            .text()
            .as_bytes()
            .get(self.offset)
            .is_some_and(|value| value.is_ascii_alphanumeric() || *value == b'_')
        {
            self.offset += 1;
        }
        let word_end = self.offset;
        if word_end - start > 128 {
            self.push_issue(
                DiagnosticCode::MALFORMED_TOKEN,
                start,
                word_end,
                "identifier exceeds the 128-byte canonical limit",
            );
        }
        if self.source.text().as_bytes().get(self.offset) == Some(&b'#') {
            let word = &self.source.text()[start..word_end];
            self.offset += 1;
            let literal_start = self.offset;
            if word.eq_ignore_ascii_case("T") || word.eq_ignore_ascii_case("TIME") {
                while self
                    .source
                    .text()
                    .as_bytes()
                    .get(self.offset)
                    .is_some_and(u8::is_ascii_alphanumeric)
                {
                    self.offset += 1;
                }
                if self.offset == literal_start {
                    self.push_issue(
                        DiagnosticCode::MALFORMED_TOKEN,
                        start,
                        self.offset,
                        "TIME literal requires at least one component",
                    );
                }
                self.push_token(TokenKind::TimeLiteral, start, self.offset);
                return;
            }
            if is_registered_type_prefix(word) {
                if self.source.text().as_bytes().get(self.offset) == Some(&b'\'') {
                    self.scan_quoted(start, TokenKind::TypedLiteral);
                    return;
                }
                while self
                    .source
                    .text()
                    .as_bytes()
                    .get(self.offset)
                    .is_some_and(|value| {
                        value.is_ascii_alphanumeric() || matches!(*value, b'.' | b'+' | b'-' | b'#')
                    })
                {
                    self.offset += 1;
                }
                self.push_token(TokenKind::TypedLiteral, start, self.offset);
                return;
            }
            while self
                .source
                .text()
                .as_bytes()
                .get(self.offset)
                .is_some_and(u8::is_ascii_alphanumeric)
            {
                self.offset += 1;
            }
            self.push_token(TokenKind::Malformed, start, self.offset);
            self.push_issue(
                DiagnosticCode::RECOGNIZED_UNSUPPORTED_SYNTAX,
                start,
                self.offset,
                "unregistered explicit type or vendor literal prefix",
            );
            return;
        }
        let word = &self.source.text()[start..word_end];
        let kind = keyword(word);
        self.push_token(kind, start, word_end);
    }

    fn scan_number(&mut self, start: usize) {
        self.offset += 1;
        while self
            .source
            .text()
            .as_bytes()
            .get(self.offset)
            .is_some_and(u8::is_ascii_digit)
        {
            self.offset += 1;
        }
        if self.source.text().as_bytes().get(self.offset) == Some(&b'#') {
            let base = &self.source.text()[start..self.offset];
            self.offset += 1;
            let digits_start = self.offset;
            while self
                .source
                .text()
                .as_bytes()
                .get(self.offset)
                .is_some_and(u8::is_ascii_alphanumeric)
            {
                self.offset += 1;
            }
            let valid_base = matches!(base, "2" | "8" | "16");
            if !valid_base || self.offset == digits_start {
                self.push_issue(
                    DiagnosticCode::MALFORMED_TOKEN,
                    start,
                    self.offset,
                    "base-qualified integer requires base 2, 8, or 16 and digits",
                );
            }
            self.push_token(TokenKind::IntegerLiteral, start, self.offset);
            return;
        }
        let mut real = false;
        if self.source.text().as_bytes().get(self.offset) == Some(&b'.')
            && self.source.text().as_bytes().get(self.offset + 1) != Some(&b'.')
        {
            real = true;
            self.offset += 1;
            while self
                .source
                .text()
                .as_bytes()
                .get(self.offset)
                .is_some_and(u8::is_ascii_digit)
            {
                self.offset += 1;
            }
        }
        if self
            .source
            .text()
            .as_bytes()
            .get(self.offset)
            .is_some_and(|value| matches!(*value, b'E' | b'e'))
        {
            real = true;
            self.offset += 1;
            if self
                .source
                .text()
                .as_bytes()
                .get(self.offset)
                .is_some_and(|value| matches!(*value, b'+' | b'-'))
            {
                self.offset += 1;
            }
            let exponent_start = self.offset;
            while self
                .source
                .text()
                .as_bytes()
                .get(self.offset)
                .is_some_and(u8::is_ascii_digit)
            {
                self.offset += 1;
            }
            if exponent_start == self.offset {
                self.push_issue(
                    DiagnosticCode::MALFORMED_TOKEN,
                    start,
                    self.offset,
                    "real exponent requires decimal digits",
                );
            }
        }
        self.push_token(
            if real {
                TokenKind::RealLiteral
            } else {
                TokenKind::IntegerLiteral
            },
            start,
            self.offset,
        );
    }

    fn scan_quoted(&mut self, token_start: usize, kind: TokenKind) {
        if self.source.text().as_bytes().get(self.offset) != Some(&b'\'') {
            // Typed literals enter with `offset` at the opening quote.
            while self.source.text().as_bytes().get(self.offset) != Some(&b'\'')
                && self.offset < self.source.text().len()
            {
                self.offset += 1;
            }
        }
        if self.source.text().as_bytes().get(self.offset) == Some(&b'\'') {
            self.offset += 1;
        }
        let mut closed = false;
        while let Some(&value) = self.source.text().as_bytes().get(self.offset) {
            if matches!(value, b'\r' | b'\n') {
                break;
            }
            if value == b'\'' {
                if self.source.text().as_bytes().get(self.offset + 1) == Some(&b'\'') {
                    self.offset += 2;
                    continue;
                }
                self.offset += 1;
                closed = true;
                break;
            }
            self.offset += self.next_char_len();
        }
        if !closed {
            self.push_issue(
                DiagnosticCode::MALFORMED_TOKEN,
                token_start,
                self.offset,
                "quoted literal is unterminated or crosses a line boundary",
            );
        }
        self.push_token(kind, token_start, self.offset);
    }

    fn starts_with(&self, value: &[u8]) -> bool {
        self.source.text().as_bytes()[self.offset..].starts_with(value)
    }

    fn next_char_len(&self) -> usize {
        self.source.text()[self.offset..]
            .chars()
            .next()
            .map_or(1, char::len_utf8)
    }

    fn push_token(&mut self, kind: TokenKind, start: usize, end: usize) {
        self.tokens.push(Token {
            kind,
            range: range(start, end),
        });
    }

    fn push_trivia(&mut self, kind: TriviaKind, start: usize, end: usize) {
        self.trivia.push(Trivia {
            kind,
            range: range(start, end),
        });
    }

    fn push_issue(
        &mut self,
        code: DiagnosticCode,
        start: usize,
        end: usize,
        cause: impl Into<String>,
    ) {
        if self.issues.len() < self.limits.max_diagnostics {
            self.issues.push(SclIssue {
                code,
                range: range(start, end),
                semantic_node: None,
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
}

fn range(start: usize, end: usize) -> TextRange {
    TextRange {
        start: u32::try_from(start).unwrap_or(u32::MAX),
        end: u32::try_from(end).unwrap_or(u32::MAX),
    }
}

fn is_registered_type_prefix(value: &str) -> bool {
    ["BOOL", "INT", "DINT", "REAL", "STRING"]
        .iter()
        .any(|candidate| value.eq_ignore_ascii_case(candidate))
}

fn keyword(value: &str) -> TokenKind {
    if value.eq_ignore_ascii_case("TRUE") {
        TokenKind::True
    } else if value.eq_ignore_ascii_case("FALSE") {
        TokenKind::False
    } else if value.eq_ignore_ascii_case("IF") {
        TokenKind::If
    } else if value.eq_ignore_ascii_case("THEN") {
        TokenKind::Then
    } else if value.eq_ignore_ascii_case("ELSIF") {
        TokenKind::Elsif
    } else if value.eq_ignore_ascii_case("ELSE") {
        TokenKind::Else
    } else if value.eq_ignore_ascii_case("END_IF") {
        TokenKind::EndIf
    } else if value.eq_ignore_ascii_case("RETURN") {
        TokenKind::Return
    } else if value.eq_ignore_ascii_case("AND") {
        TokenKind::And
    } else if value.eq_ignore_ascii_case("XOR") {
        TokenKind::Xor
    } else if value.eq_ignore_ascii_case("OR") {
        TokenKind::Or
    } else if value.eq_ignore_ascii_case("NOT") {
        TokenKind::Not
    } else if value.eq_ignore_ascii_case("MOD") {
        TokenKind::Mod
    } else if [
        "CASE",
        "OF",
        "END_CASE",
        "FOR",
        "TO",
        "BY",
        "DO",
        "END_FOR",
        "WHILE",
        "END_WHILE",
        "REPEAT",
        "UNTIL",
        "END_REPEAT",
        "EXIT",
        "CONTINUE",
        "VAR",
        "VAR_INPUT",
        "VAR_OUTPUT",
        "VAR_IN_OUT",
        "VAR_TEMP",
        "BEGIN",
        "END_BLOCK",
        "CONFIGURATION",
        "RESOURCE",
    ]
    .iter()
    .any(|candidate| value.eq_ignore_ascii_case(candidate))
    {
        TokenKind::UnsupportedKeyword
    } else {
        TokenKind::Identifier
    }
}

#[cfg(test)]
mod tests {
    use plc_program::BlockId;

    use super::*;

    #[test]
    fn keywords_comments_and_longest_tokens_are_preserved() {
        let source = SclSource::new(
            BlockId::new(1),
            "// head\nIF ready <= TRUE THEN (* note *) out := 16#0F; END_IF;",
        );
        let lexed = lex_scl(&source, ResourceLimits::default());
        assert!(lexed.issues().is_empty());
        assert_eq!(
            lexed
                .trivia()
                .iter()
                .filter(|trivia| matches!(
                    trivia.kind,
                    TriviaKind::LineComment | TriviaKind::BlockComment
                ))
                .count(),
            2
        );
        assert!(
            lexed
                .tokens()
                .iter()
                .any(|token| token.kind == TokenKind::LessEqual)
        );
        assert!(
            lexed
                .tokens()
                .iter()
                .any(|token| token.kind == TokenKind::IntegerLiteral)
        );
        assert_eq!(lexed.source().text(), source.text());
    }
}
