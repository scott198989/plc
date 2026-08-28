use alloc::{
    collections::{BTreeMap, BTreeSet},
    string::String,
    vec::Vec,
};

use crate::{
    BlockId, BlockInterface, CallSiteId, CanonicalValue, ControllerId, EngineeringNumber,
    InstructionCode, InstructionUseId, InterfaceMemberId, PHASE2_INSTRUCTION_REGISTRY_VERSION,
    PROGRAM_MODEL_SCHEMA_VERSION,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ObDeclaration {
    CyclicMain,
    Startup,
    TimedCyclic {
        period_milliseconds: u32,
        offset_milliseconds: u32,
        priority: u16,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DataBlockKind {
    Global,
    Instance { fb_type: BlockId },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ProgramUnitKind {
    OrganizationBlock(ObDeclaration),
    Function,
    FunctionBlock,
    DataBlock(DataBlockKind),
}

impl ProgramUnitKind {
    #[must_use]
    pub const fn is_executable(self) -> bool {
        matches!(
            self,
            Self::OrganizationBlock(_) | Self::Function | Self::FunctionBlock
        )
    }

    #[must_use]
    pub const fn engineering_prefix(self) -> &'static str {
        match self {
            Self::OrganizationBlock(_) => "OB",
            Self::Function => "FC",
            Self::FunctionBlock => "FB",
            Self::DataBlock(_) => "DB",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum VariableRef {
    CallerMember(InterfaceMemberId),
    DataBlockMember {
        data_block: BlockId,
        member: InterfaceMemberId,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BindingActual {
    Literal(CanonicalValue),
    Variable(VariableRef),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParameterBinding {
    pub formal: InterfaceMemberId,
    pub actual: BindingActual,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum InstanceOwner {
    InstanceDb(BlockId),
    MultiInstance {
        owner_fb: BlockId,
        static_member: InterfaceMemberId,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InstancePath {
    pub root_instance_db: BlockId,
    pub multi_instance_slots: Vec<InterfaceMemberId>,
}

impl InstanceOwner {
    /// Materializes structural state identity without allocating runtime state.
    /// A single instance starts a root path; a multi-instance extends the
    /// caller's existing instance path by its stable Static-member identity.
    #[must_use]
    pub fn materialize_path(self, parent: Option<&InstancePath>) -> Option<InstancePath> {
        match self {
            Self::InstanceDb(root_instance_db) => Some(InstancePath {
                root_instance_db,
                multi_instance_slots: Vec::new(),
            }),
            Self::MultiInstance { static_member, .. } => {
                let mut path = parent?.clone();
                path.multi_instance_slots.push(static_member);
                Some(path)
            }
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CallSite {
    pub id: CallSiteId,
    pub instruction: InstructionCode,
    pub callee: BlockId,
    pub bindings: Vec<ParameterBinding>,
    pub instance_owner: Option<InstanceOwner>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstructionUse {
    pub id: InstructionUseId,
    pub instruction: InstructionCode,
    pub state_owner: Option<VariableRef>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProgramBlock {
    pub id: BlockId,
    pub display_name: String,
    pub engineering_number: EngineeringNumber,
    pub kind: ProgramUnitKind,
    pub interface: BlockInterface,
    pub instructions: Vec<InstructionUse>,
    pub calls: Vec<CallSite>,
}

impl ProgramBlock {
    #[must_use]
    pub fn new(
        id: BlockId,
        display_name: impl Into<String>,
        engineering_number: EngineeringNumber,
        kind: ProgramUnitKind,
        interface: BlockInterface,
    ) -> Self {
        Self {
            id,
            display_name: display_name.into(),
            engineering_number,
            kind,
            interface,
            instructions: Vec::new(),
            calls: Vec::new(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProgramEditError {
    DuplicateBlockId(BlockId),
    MissingBlock(BlockId),
    RevisionOverflow,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ControllerProgram {
    schema_version: u16,
    controller_id: ControllerId,
    semantic_revision: u64,
    registry_version: String,
    blocks: BTreeMap<BlockId, ProgramBlock>,
}

impl ControllerProgram {
    #[must_use]
    pub fn new(controller_id: ControllerId) -> Self {
        Self {
            schema_version: PROGRAM_MODEL_SCHEMA_VERSION,
            controller_id,
            semantic_revision: 0,
            registry_version: String::from(PHASE2_INSTRUCTION_REGISTRY_VERSION),
            blocks: BTreeMap::new(),
        }
    }

    /// Rehydrates an aggregate while keeping version mismatches observable to
    /// validation instead of silently upgrading semantic identity.
    #[must_use]
    pub fn from_parts(
        schema_version: u16,
        controller_id: ControllerId,
        semantic_revision: u64,
        registry_version: impl Into<String>,
        blocks: BTreeMap<BlockId, ProgramBlock>,
    ) -> Self {
        Self {
            schema_version,
            controller_id,
            semantic_revision,
            registry_version: registry_version.into(),
            blocks,
        }
    }

    #[must_use]
    pub const fn schema_version(&self) -> u16 {
        self.schema_version
    }

    #[must_use]
    pub const fn controller_id(&self) -> ControllerId {
        self.controller_id
    }

    #[must_use]
    pub const fn semantic_revision(&self) -> u64 {
        self.semantic_revision
    }

    #[must_use]
    pub fn registry_version(&self) -> &str {
        &self.registry_version
    }

    #[must_use]
    pub fn blocks(&self) -> &BTreeMap<BlockId, ProgramBlock> {
        &self.blocks
    }

    #[must_use]
    pub fn block(&self, id: BlockId) -> Option<&ProgramBlock> {
        self.blocks.get(&id)
    }

    pub(crate) fn block_mut(&mut self, id: BlockId) -> Option<&mut ProgramBlock> {
        self.blocks.get_mut(&id)
    }

    /// Inserts a block and advances the aggregate semantic revision.
    ///
    /// # Errors
    ///
    /// Returns [`ProgramEditError::DuplicateBlockId`] when the stable identity
    /// already exists or [`ProgramEditError::RevisionOverflow`] when the
    /// canonical aggregate revision cannot advance. It never replaces an
    /// existing block implicitly.
    pub fn insert_block(&mut self, block: ProgramBlock) -> Result<(), ProgramEditError> {
        if self.blocks.contains_key(&block.id) {
            return Err(ProgramEditError::DuplicateBlockId(block.id));
        }
        let next_revision = self
            .semantic_revision
            .checked_add(1)
            .ok_or(ProgramEditError::RevisionOverflow)?;
        self.blocks.insert(block.id, block);
        self.semantic_revision = next_revision;
        Ok(())
    }

    /// Replaces a block with the same stable identity and advances revision.
    ///
    /// # Errors
    ///
    /// Returns [`ProgramEditError::MissingBlock`] when the identity is absent
    /// or [`ProgramEditError::RevisionOverflow`] when revision cannot advance.
    pub fn replace_block(&mut self, block: ProgramBlock) -> Result<(), ProgramEditError> {
        if !self.blocks.contains_key(&block.id) {
            return Err(ProgramEditError::MissingBlock(block.id));
        }
        let next_revision = self
            .semantic_revision
            .checked_add(1)
            .ok_or(ProgramEditError::RevisionOverflow)?;
        self.blocks.insert(block.id, block);
        self.semantic_revision = next_revision;
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DependencyReason {
    Call,
    DataUse,
    InstanceOf,
    MultiInstanceState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DependencyEdge {
    pub dependent: BlockId,
    pub dependency: BlockId,
    pub reason: DependencyReason,
    pub call_site: Option<CallSiteId>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct DependencyGraph {
    edges: BTreeSet<DependencyEdge>,
}

impl DependencyGraph {
    pub(crate) fn insert(&mut self, edge: DependencyEdge) {
        self.edges.insert(edge);
    }

    #[must_use]
    pub fn edges(&self) -> &BTreeSet<DependencyEdge> {
        &self.edges
    }

    pub fn dependencies_of(&self, block: BlockId) -> impl Iterator<Item = &DependencyEdge> {
        self.edges
            .iter()
            .filter(move |edge| edge.dependent == block)
    }

    pub fn dependents_of(&self, block: BlockId) -> impl Iterator<Item = &DependencyEdge> {
        self.edges
            .iter()
            .filter(move |edge| edge.dependency == block)
    }
}
