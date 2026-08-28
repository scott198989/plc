use alloc::string::String;

use plc_program::BlockId;
use plc_runtime::Hash32;

use crate::{SemanticNodeId, hash::hash_bytes};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TextRange {
    pub start: u32,
    pub end: u32,
}

impl TextRange {
    #[must_use]
    pub const fn new(start: u32, end: u32) -> Option<Self> {
        if start <= end {
            Some(Self { start, end })
        } else {
            None
        }
    }

    #[must_use]
    pub const fn empty(at: u32) -> Self {
        Self { start: at, end: at }
    }

    #[must_use]
    pub const fn len(self) -> u32 {
        self.end - self.start
    }

    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.start == self.end
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SourceLanguage {
    Scl,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SclSource {
    owner: BlockId,
    revision_hash: Hash32,
    text: String,
}

impl SclSource {
    #[must_use]
    pub fn new(owner: BlockId, text: impl Into<String>) -> Self {
        let text = text.into();
        let revision_hash = hash_bytes("PES-SCL-SOURCE-1", text.as_bytes());
        Self {
            owner,
            revision_hash,
            text,
        }
    }

    #[must_use]
    pub const fn owner(&self) -> BlockId {
        self.owner
    }

    #[must_use]
    pub const fn revision_hash(&self) -> Hash32 {
        self.revision_hash
    }

    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    #[must_use]
    pub fn range_text(&self, range: TextRange) -> Option<&str> {
        let start = usize::try_from(range.start).ok()?;
        let end = usize::try_from(range.end).ok()?;
        self.text.get(start..end)
    }

    #[must_use]
    pub fn line_column(&self, byte_offset: u32) -> Option<LineColumn> {
        let offset = usize::try_from(byte_offset).ok()?;
        if offset > self.text.len() || !self.text.is_char_boundary(offset) {
            return None;
        }
        let prefix = &self.text[..offset];
        let line = prefix.bytes().filter(|byte| *byte == b'\n').count() + 1;
        let line_start = prefix.rfind('\n').map_or(0, |index| index + 1);
        let column = self.text[line_start..offset].chars().count() + 1;
        Some(LineColumn {
            line: u32::try_from(line).ok()?,
            column: u32::try_from(column).ok()?,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LineColumn {
    pub line: u32,
    pub column: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SourceAnchor {
    pub owner_object_id: BlockId,
    pub source_revision_hash: Hash32,
    pub language: SourceLanguage,
    pub semantic_node_id: SemanticNodeId,
    pub text_range: TextRange,
}
