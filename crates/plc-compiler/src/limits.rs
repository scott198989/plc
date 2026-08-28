use core::sync::atomic::{AtomicBool, Ordering};

use plc_hardware::{ProfileAllowlist, TrainingProfile};

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
    /// Projects compiler ceilings from an admitted training profile. There is
    /// no independently maintained shipped compiler-limit table.
    ///
    /// # Errors
    ///
    /// Returns a fail-closed error if the profile is not the exact shipped
    /// allowlisted value or a ceiling cannot be represented on this target.
    pub fn from_training_profile(profile: &TrainingProfile) -> Result<Self, ResourceProfileError> {
        let admitted = ProfileAllowlist::load(&profile.pin())
            .map_err(|_| ResourceProfileError::UnapprovedProfile)?;
        if admitted != *profile {
            return Err(ResourceProfileError::UnapprovedProfile);
        }
        let limits = profile.limits();
        let source_bytes = to_usize(
            u64::from(limits.source_bytes_per_block),
            "source_bytes_per_block",
        )?;
        Ok(Self {
            max_source_bytes_per_block: source_bytes,
            max_tokens_per_block: source_bytes / 4,
            max_syntax_nodes_per_block: source_bytes / 4,
            max_syntax_depth: to_usize(u64::from(limits.syntax_nesting), "syntax_nesting")?,
            max_dependency_edges: to_usize(limits.dependency_edges, "dependency_edges")?,
            max_diagnostics: to_usize(
                u64::from(limits.diagnostics_per_build),
                "diagnostics_per_build",
            )?,
            max_ir_operations: to_usize(
                limits.constant_evaluation_operations,
                "constant_evaluation_operations",
            )?,
            max_compiler_work_units: limits.semantic_work_units_per_build,
            max_artifact_bytes: to_usize(limits.artifact_package_bytes, "artifact_package_bytes")?,
        })
    }
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self::from_training_profile(&TrainingProfile::edu21())
            .expect("the embedded EDU-21 profile has target-representable compiler limits")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResourceProfileError {
    UnapprovedProfile,
    LimitOutOfRange(&'static str),
}

fn to_usize(value: u64, field: &'static str) -> Result<usize, ResourceProfileError> {
    usize::try_from(value).map_err(|_| ResourceProfileError::LimitOutOfRange(field))
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
