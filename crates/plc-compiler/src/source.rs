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
    Lad,
    Fbd,
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
    pub text_range: Option<TextRange>,
    pub network_id: Option<u128>,
    pub node_id: Option<u128>,
    pub port_id: Option<u128>,
    pub edge_id: Option<u128>,
    pub operand_id: Option<u128>,
    pub call_site_id: Option<u128>,
    pub state_instance_id: Option<u128>,
}

/// Revision-independent source identity used only for guarded relocation.
///
/// Text anchors retain the compiler-issued semantic node identity. Graph
/// anchors instead retain the authored graph identities required by
/// `PES-SMAP-0001`; graph semantic order is deliberately absent so a safe
/// relocation does not turn into a coordinate or ordering match.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct StableSourceIdentity {
    pub owner_object_id: BlockId,
    pub language: SourceLanguage,
    pub semantic_node_id: Option<SemanticNodeId>,
    pub network_id: Option<u128>,
    pub node_id: Option<u128>,
    pub port_id: Option<u128>,
    pub edge_id: Option<u128>,
    pub operand_id: Option<u128>,
    pub call_site_id: Option<u128>,
    pub state_instance_id: Option<u128>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GraphSourceIds {
    pub network_id: Option<u128>,
    pub node_id: Option<u128>,
    pub port_id: Option<u128>,
    pub edge_id: Option<u128>,
    pub operand_id: Option<u128>,
    pub call_site_id: Option<u128>,
    pub state_instance_id: Option<u128>,
}

impl SourceAnchor {
    #[must_use]
    pub const fn scl(
        owner_object_id: BlockId,
        source_revision_hash: Hash32,
        semantic_node_id: SemanticNodeId,
        text_range: TextRange,
    ) -> Self {
        Self {
            owner_object_id,
            source_revision_hash,
            language: SourceLanguage::Scl,
            semantic_node_id,
            text_range: Some(text_range),
            network_id: None,
            node_id: None,
            port_id: None,
            edge_id: None,
            operand_id: None,
            call_site_id: None,
            state_instance_id: None,
        }
    }

    #[must_use]
    pub const fn graph(
        owner_object_id: BlockId,
        source_revision_hash: Hash32,
        language: SourceLanguage,
        semantic_node_id: SemanticNodeId,
        ids: GraphSourceIds,
    ) -> Option<Self> {
        if matches!(language, SourceLanguage::Scl) {
            return None;
        }
        Some(Self {
            owner_object_id,
            source_revision_hash,
            language,
            semantic_node_id,
            text_range: None,
            network_id: ids.network_id,
            node_id: ids.node_id,
            port_id: ids.port_id,
            edge_id: ids.edge_id,
            operand_id: ids.operand_id,
            call_site_id: ids.call_site_id,
            state_instance_id: ids.state_instance_id,
        })
    }

    /// Returns the only identity that may be used to relocate an anchor
    /// across source revisions. Names, text, addresses, line numbers, and
    /// layout are intentionally excluded.
    #[must_use]
    pub const fn stable_identity(&self) -> StableSourceIdentity {
        match self.language {
            SourceLanguage::Scl => StableSourceIdentity {
                owner_object_id: self.owner_object_id,
                language: SourceLanguage::Scl,
                semantic_node_id: Some(self.semantic_node_id),
                network_id: None,
                node_id: None,
                port_id: None,
                edge_id: None,
                operand_id: None,
                call_site_id: None,
                state_instance_id: None,
            },
            SourceLanguage::Lad | SourceLanguage::Fbd => StableSourceIdentity {
                owner_object_id: self.owner_object_id,
                language: self.language,
                semantic_node_id: None,
                network_id: self.network_id,
                node_id: self.node_id,
                port_id: self.port_id,
                edge_id: self.edge_id,
                operand_id: self.operand_id,
                call_site_id: self.call_site_id,
                state_instance_id: self.state_instance_id,
            },
        }
    }

    /// Checks the language-specific identity shape and owning IR function.
    /// Text and graph locations are deliberately disjoint so an adapter can
    /// never reinterpret pixel/graph identity as an SCL byte range.
    #[must_use]
    pub fn is_well_formed_for(&self, owner: BlockId) -> bool {
        if self.owner_object_id != owner {
            return false;
        }
        match self.language {
            SourceLanguage::Scl => {
                self.text_range.is_some()
                    && self.network_id.is_none()
                    && self.node_id.is_none()
                    && self.port_id.is_none()
                    && self.edge_id.is_none()
                    && self.operand_id.is_none()
                    && self.call_site_id.is_none()
                    && self.state_instance_id.is_none()
            }
            SourceLanguage::Lad | SourceLanguage::Fbd => {
                self.text_range.is_none() && self.network_id.is_some()
            }
        }
    }
}
