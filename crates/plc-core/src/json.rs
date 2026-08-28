#![allow(clippy::struct_field_names)]

use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum JsonValue {
    Null,
    Bool(bool),
    Number(String),
    String(String),
    Array(Vec<Self>),
    Object(BTreeMap<String, Self>),
}

impl JsonValue {
    pub(crate) fn object(entries: impl IntoIterator<Item = (String, Self)>) -> Self {
        Self::Object(entries.into_iter().collect())
    }

    pub(crate) fn as_object(&self) -> Result<&BTreeMap<String, Self>, JsonError> {
        match self {
            Self::Object(value) => Ok(value),
            _ => Err(JsonError::TypeMismatch("object")),
        }
    }

    pub(crate) fn as_array(&self) -> Result<&[Self], JsonError> {
        match self {
            Self::Array(value) => Ok(value),
            _ => Err(JsonError::TypeMismatch("array")),
        }
    }

    pub(crate) fn as_str(&self) -> Result<&str, JsonError> {
        match self {
            Self::String(value) => Ok(value),
            _ => Err(JsonError::TypeMismatch("string")),
        }
    }

    pub(crate) fn as_bool(&self) -> Result<bool, JsonError> {
        match self {
            Self::Bool(value) => Ok(*value),
            _ => Err(JsonError::TypeMismatch("boolean")),
        }
    }

    pub(crate) fn as_u64(&self) -> Result<u64, JsonError> {
        match self {
            Self::Number(value) => value.parse().map_err(|_| JsonError::InvalidNumber),
            _ => Err(JsonError::TypeMismatch("number")),
        }
    }
}

impl From<&str> for JsonValue {
    fn from(value: &str) -> Self {
        Self::String(value.to_owned())
    }
}

impl From<String> for JsonValue {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl From<u64> for JsonValue {
    fn from(value: u64) -> Self {
        Self::Number(value.to_string())
    }
}

impl From<u32> for JsonValue {
    fn from(value: u32) -> Self {
        Self::Number(value.to_string())
    }
}

impl From<i64> for JsonValue {
    fn from(value: i64) -> Self {
        Self::Number(value.to_string())
    }
}

impl From<bool> for JsonValue {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct JsonLimits {
    pub max_depth: usize,
    pub max_string_bytes: usize,
    pub max_collection_items: usize,
    pub max_total_values: usize,
}

impl Default for JsonLimits {
    fn default() -> Self {
        Self {
            max_depth: 32,
            max_string_bytes: 1024 * 1024,
            max_collection_items: 100_000,
            max_total_values: 1_000_000,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum JsonError {
    BomForbidden,
    DuplicateKey,
    InvalidEscape,
    InvalidNumber,
    InvalidSyntax,
    InvalidUnicode,
    LimitExceeded,
    TrailingData,
    TypeMismatch(&'static str),
    UnknownField(String),
    MissingField(&'static str),
}

pub(crate) fn canonical_json(value: &JsonValue) -> Vec<u8> {
    let mut output = String::new();
    write_value(value, &mut output);
    output.into_bytes()
}

fn write_value(value: &JsonValue, output: &mut String) {
    match value {
        JsonValue::Null => output.push_str("null"),
        JsonValue::Bool(true) => output.push_str("true"),
        JsonValue::Bool(false) => output.push_str("false"),
        JsonValue::Number(number) => output.push_str(number),
        JsonValue::String(string) => write_string(string, output),
        JsonValue::Array(items) => {
            output.push('[');
            for (index, item) in items.iter().enumerate() {
                if index != 0 {
                    output.push(',');
                }
                write_value(item, output);
            }
            output.push(']');
        }
        JsonValue::Object(entries) => {
            output.push('{');
            for (index, (key, item)) in entries.iter().enumerate() {
                if index != 0 {
                    output.push(',');
                }
                write_string(key, output);
                output.push(':');
                write_value(item, output);
            }
            output.push('}');
        }
    }
}

fn write_string(value: &str, output: &mut String) {
    output.push('"');
    for character in value.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\u{08}' => output.push_str("\\b"),
            '\u{0c}' => output.push_str("\\f"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            '\u{00}'..='\u{1f}' => {
                use core::fmt::Write;
                write!(output, "\\u{:04x}", u32::from(character))
                    .expect("writing to a String cannot fail");
            }
            _ => output.push(character),
        }
    }
    output.push('"');
}

pub(crate) fn parse_json(input: &[u8], limits: JsonLimits) -> Result<JsonValue, JsonError> {
    if input.starts_with(&[0xef, 0xbb, 0xbf]) {
        return Err(JsonError::BomForbidden);
    }
    let text = core::str::from_utf8(input).map_err(|_| JsonError::InvalidUnicode)?;
    if text.contains('\r') {
        return Err(JsonError::InvalidSyntax);
    }
    let mut parser = Parser {
        bytes: text.as_bytes(),
        index: 0,
        limits,
        total_values: 0,
    };
    let value = parser.parse_value(0)?;
    parser.skip_whitespace();
    if parser.index != parser.bytes.len() {
        return Err(JsonError::TrailingData);
    }
    Ok(value)
}

struct Parser<'a> {
    bytes: &'a [u8],
    index: usize,
    limits: JsonLimits,
    total_values: usize,
}

impl Parser<'_> {
    fn parse_value(&mut self, depth: usize) -> Result<JsonValue, JsonError> {
        if depth > self.limits.max_depth {
            return Err(JsonError::LimitExceeded);
        }
        self.total_values = self
            .total_values
            .checked_add(1)
            .ok_or(JsonError::LimitExceeded)?;
        if self.total_values > self.limits.max_total_values {
            return Err(JsonError::LimitExceeded);
        }
        self.skip_whitespace();
        match self.peek() {
            Some(b'n') => {
                self.consume_literal(b"null")?;
                Ok(JsonValue::Null)
            }
            Some(b't') => {
                self.consume_literal(b"true")?;
                Ok(JsonValue::Bool(true))
            }
            Some(b'f') => {
                self.consume_literal(b"false")?;
                Ok(JsonValue::Bool(false))
            }
            Some(b'"') => self.parse_string().map(JsonValue::String),
            Some(b'[') => self.parse_array(depth + 1),
            Some(b'{') => self.parse_object(depth + 1),
            Some(b'-' | b'0'..=b'9') => self.parse_number().map(JsonValue::Number),
            _ => Err(JsonError::InvalidSyntax),
        }
    }

    fn parse_array(&mut self, depth: usize) -> Result<JsonValue, JsonError> {
        self.expect(b'[')?;
        self.skip_whitespace();
        let mut values = Vec::new();
        if self.peek() == Some(b']') {
            self.index += 1;
            return Ok(JsonValue::Array(values));
        }
        loop {
            if values.len() >= self.limits.max_collection_items {
                return Err(JsonError::LimitExceeded);
            }
            values.push(self.parse_value(depth)?);
            self.skip_whitespace();
            match self.peek() {
                Some(b',') => self.index += 1,
                Some(b']') => {
                    self.index += 1;
                    break;
                }
                _ => return Err(JsonError::InvalidSyntax),
            }
        }
        Ok(JsonValue::Array(values))
    }

    fn parse_object(&mut self, depth: usize) -> Result<JsonValue, JsonError> {
        self.expect(b'{')?;
        self.skip_whitespace();
        let mut entries = BTreeMap::new();
        if self.peek() == Some(b'}') {
            self.index += 1;
            return Ok(JsonValue::Object(entries));
        }
        loop {
            if entries.len() >= self.limits.max_collection_items {
                return Err(JsonError::LimitExceeded);
            }
            self.skip_whitespace();
            let key = self.parse_string()?;
            if entries.contains_key(&key) {
                return Err(JsonError::DuplicateKey);
            }
            self.skip_whitespace();
            self.expect(b':')?;
            let value = self.parse_value(depth)?;
            entries.insert(key, value);
            self.skip_whitespace();
            match self.peek() {
                Some(b',') => self.index += 1,
                Some(b'}') => {
                    self.index += 1;
                    break;
                }
                _ => return Err(JsonError::InvalidSyntax),
            }
        }
        Ok(JsonValue::Object(entries))
    }

    fn parse_string(&mut self) -> Result<String, JsonError> {
        self.expect(b'"')?;
        let mut output = String::new();
        while let Some(byte) = self.peek() {
            self.index += 1;
            match byte {
                b'"' => {
                    if output.len() > self.limits.max_string_bytes {
                        return Err(JsonError::LimitExceeded);
                    }
                    return Ok(output);
                }
                b'\\' => self.parse_escape(&mut output)?,
                0x00..=0x1f => return Err(JsonError::InvalidSyntax),
                0x20..=0x7f => output.push(char::from(byte)),
                _ => {
                    self.index -= 1;
                    let remaining = core::str::from_utf8(&self.bytes[self.index..])
                        .map_err(|_| JsonError::InvalidUnicode)?;
                    let character = remaining.chars().next().ok_or(JsonError::InvalidUnicode)?;
                    output.push(character);
                    self.index += character.len_utf8();
                }
            }
            if output.len() > self.limits.max_string_bytes {
                return Err(JsonError::LimitExceeded);
            }
        }
        Err(JsonError::InvalidSyntax)
    }

    fn parse_escape(&mut self, output: &mut String) -> Result<(), JsonError> {
        let escaped = self.peek().ok_or(JsonError::InvalidEscape)?;
        self.index += 1;
        match escaped {
            b'"' => output.push('"'),
            b'\\' => output.push('\\'),
            b'/' => output.push('/'),
            b'b' => output.push('\u{08}'),
            b'f' => output.push('\u{0c}'),
            b'n' => output.push('\n'),
            b'r' => output.push('\r'),
            b't' => output.push('\t'),
            b'u' => {
                let code = self.parse_hex_quad()?;
                if (0xd800..=0xdbff).contains(&code) {
                    if self.bytes.get(self.index..self.index + 2) != Some(b"\\u") {
                        return Err(JsonError::InvalidUnicode);
                    }
                    self.index += 2;
                    let low = self.parse_hex_quad()?;
                    if !(0xdc00..=0xdfff).contains(&low) {
                        return Err(JsonError::InvalidUnicode);
                    }
                    let scalar =
                        0x1_0000 + ((u32::from(code) - 0xd800) << 10) + (u32::from(low) - 0xdc00);
                    output.push(char::from_u32(scalar).ok_or(JsonError::InvalidUnicode)?);
                } else if (0xdc00..=0xdfff).contains(&code) {
                    return Err(JsonError::InvalidUnicode);
                } else {
                    output.push(char::from_u32(u32::from(code)).ok_or(JsonError::InvalidUnicode)?);
                }
            }
            _ => return Err(JsonError::InvalidEscape),
        }
        Ok(())
    }

    fn parse_hex_quad(&mut self) -> Result<u16, JsonError> {
        let bytes = self
            .bytes
            .get(self.index..self.index + 4)
            .ok_or(JsonError::InvalidEscape)?;
        let mut value = 0_u16;
        for byte in bytes {
            value = value
                .checked_mul(16)
                .and_then(|current| hex_value(*byte).map(|next| current + u16::from(next)))
                .ok_or(JsonError::InvalidEscape)?;
        }
        self.index += 4;
        Ok(value)
    }

    fn parse_number(&mut self) -> Result<String, JsonError> {
        let start = self.index;
        if self.peek() == Some(b'-') {
            self.index += 1;
        }
        match self.peek() {
            Some(b'0') => {
                self.index += 1;
                if matches!(self.peek(), Some(b'0'..=b'9')) {
                    return Err(JsonError::InvalidNumber);
                }
            }
            Some(b'1'..=b'9') => {
                self.index += 1;
                while matches!(self.peek(), Some(b'0'..=b'9')) {
                    self.index += 1;
                }
            }
            _ => return Err(JsonError::InvalidNumber),
        }
        if matches!(self.peek(), Some(b'.' | b'e' | b'E')) {
            return Err(JsonError::InvalidNumber);
        }
        core::str::from_utf8(&self.bytes[start..self.index])
            .map(str::to_owned)
            .map_err(|_| JsonError::InvalidNumber)
    }

    fn consume_literal(&mut self, literal: &[u8]) -> Result<(), JsonError> {
        if self.bytes.get(self.index..self.index + literal.len()) != Some(literal) {
            return Err(JsonError::InvalidSyntax);
        }
        self.index += literal.len();
        Ok(())
    }

    fn expect(&mut self, expected: u8) -> Result<(), JsonError> {
        if self.peek() != Some(expected) {
            return Err(JsonError::InvalidSyntax);
        }
        self.index += 1;
        Ok(())
    }

    fn skip_whitespace(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\n' | b'\t')) {
            self.index += 1;
        }
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.index).copied()
    }
}

fn hex_value(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

pub(crate) fn required<'a>(
    object: &'a BTreeMap<String, JsonValue>,
    key: &'static str,
) -> Result<&'a JsonValue, JsonError> {
    object.get(key).ok_or(JsonError::MissingField(key))
}

pub(crate) fn require_only_fields(
    object: &BTreeMap<String, JsonValue>,
    allowed: &[&str],
) -> Result<(), JsonError> {
    for key in object.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(JsonError::UnknownField(key.clone()));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{JsonError, JsonLimits, JsonValue, canonical_json, parse_json};

    #[test]
    fn canonical_object_is_sorted_and_round_trips() {
        let value = JsonValue::object([
            ("z".to_owned(), JsonValue::from(2_u64)),
            ("a".to_owned(), JsonValue::from("line\nvalue")),
        ]);
        let encoded = canonical_json(&value);
        assert_eq!(encoded, br#"{"a":"line\nvalue","z":2}"#);
        assert_eq!(parse_json(&encoded, JsonLimits::default()), Ok(value));
    }

    #[test]
    fn rejects_duplicate_keys_bom_and_noncanonical_numbers() {
        assert_eq!(
            parse_json(br#"{"a":1,"a":2}"#, JsonLimits::default()),
            Err(JsonError::DuplicateKey)
        );
        assert_eq!(
            parse_json(&[0xef, 0xbb, 0xbf, b'{', b'}'], JsonLimits::default()),
            Err(JsonError::BomForbidden)
        );
        assert_eq!(
            parse_json(b"01", JsonLimits::default()),
            Err(JsonError::InvalidNumber)
        );
    }
}
