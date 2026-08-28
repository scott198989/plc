use core::sync::atomic::{AtomicBool, Ordering};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResourceLimits {
    pub max_source_bytes_per_block: usize,
    pub max_tokens_per_block: usize,
    pub max_syntax_nodes_per_block: usize,
    pub max_syntax_depth: usize,
    pub max_dependency_edges: usize,
    pub max_diagnostics: usize,
    pub max_ir_operations: usize,
    pub max_compiler_work_units: u64,
    pub max_artifact_bytes: usize,
}

impl ResourceLimits {
    pub const EDU21: Self = Self {
        max_source_bytes_per_block: 1_048_576,
        max_tokens_per_block: 262_144,
        max_syntax_nodes_per_block: 262_144,
        max_syntax_depth: 256,
        max_dependency_edges: 1_000_000,
        max_diagnostics: 10_000,
        max_ir_operations: 1_000_000,
        max_compiler_work_units: 10_000_000,
        max_artifact_bytes: 64 * 1_048_576,
    };
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self::EDU21
    }
}

#[derive(Debug, Default)]
pub struct CancellationToken {
    cancelled: AtomicBool,
}

impl CancellationToken {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            cancelled: AtomicBool::new(false),
        }
    }

    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Release);
    }

    #[must_use]
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Acquire)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResourceLimit {
    pub key: &'static str,
    pub current: u64,
    pub maximum: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum WorkStop {
    Cancelled,
    Resource(ResourceLimit),
}

pub(crate) struct WorkMeter<'a> {
    limits: ResourceLimits,
    cancellation: Option<&'a CancellationToken>,
    used: u64,
}

impl<'a> WorkMeter<'a> {
    pub(crate) const fn new(
        limits: ResourceLimits,
        cancellation: Option<&'a CancellationToken>,
    ) -> Self {
        Self {
            limits,
            cancellation,
            used: 0,
        }
    }

    pub(crate) fn checkpoint(&self) -> Result<(), WorkStop> {
        if self
            .cancellation
            .is_some_and(CancellationToken::is_cancelled)
        {
            Err(WorkStop::Cancelled)
        } else {
            Ok(())
        }
    }

    pub(crate) fn charge(&mut self, amount: u64) -> Result<(), WorkStop> {
        self.checkpoint()?;
        let next = self.used.saturating_add(amount);
        if next > self.limits.max_compiler_work_units {
            return Err(WorkStop::Resource(ResourceLimit {
                key: "compiler.work_units",
                current: next,
                maximum: self.limits.max_compiler_work_units,
            }));
        }
        self.used = next;
        Ok(())
    }

    pub(crate) const fn used(&self) -> u64 {
        self.used
    }
}
