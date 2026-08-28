use core::{error::Error, fmt};

use plc_commissioning::{
    SessionCommandBinding, SessionState, VirtualOnlineSessionId, VirtualUniverse,
};
use plc_runtime::{CpuState, Hash32, UniverseId, VirtualControllerId};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum PublicationBoundary {
    SerializedCommand = 1,
    ScanEnd = 2,
    FatalFault = 3,
    SnapshotReplay = 4,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ObservationContext {
    pub universe_id: UniverseId,
    pub universe_epoch: u64,
    pub controller_id: VirtualControllerId,
    pub controller_epoch: u64,
    pub session_id: VirtualOnlineSessionId,
    pub session_epoch: u64,
    pub package_fingerprint: Hash32,
    pub artifact_fingerprint: Hash32,
    pub profile_fingerprint: Hash32,
    pub target_state_hash: Hash32,
    pub cpu_state: CpuState,
    pub virtual_timestamp_ms: u64,
    pub scan_sequence: u64,
    pub event_sequence: u64,
    pub publication_boundary: PublicationBoundary,
}

impl ObservationContext {
    pub fn from_virtual_universe(
        universe: &VirtualUniverse,
        binding: SessionCommandBinding,
        publication_boundary: PublicationBoundary,
    ) -> Result<Self, ContextError> {
        let session = universe
            .session(binding.session_id)
            .ok_or(ContextError::UnknownSession)?;
        if session.state() != SessionState::Online {
            return Err(ContextError::SessionNotOnline(session.state()));
        }
        if universe.universe_epoch() != binding.expected_universe_epoch
            || session.universe_epoch() != binding.expected_universe_epoch
        {
            return Err(ContextError::StaleUniverseEpoch);
        }
        if session.session_epoch() != binding.expected_session_epoch {
            return Err(ContextError::StaleSessionEpoch);
        }
        let instance = universe
            .controller(session.controller_id())
            .ok_or(ContextError::TargetUnavailable)?;
        let runtime = instance.runtime();
        if runtime.controller_epoch() != binding.expected_controller_epoch
            || session.controller_epoch() != Some(binding.expected_controller_epoch)
        {
            return Err(ContextError::StaleControllerEpoch);
        }
        if instance.semantic_state_hash() != binding.expected_target_state_hash {
            return Err(ContextError::StaleTargetState);
        }
        let package = instance
            .loaded_package()
            .ok_or(ContextError::NoLoadedPackage)?;
        if session.loaded_package_fingerprint() != Some(package.fingerprint()) {
            return Err(ContextError::SessionPackageMismatch);
        }
        let artifact_fingerprint = runtime
            .loaded_fingerprint()
            .ok_or(ContextError::NoLoadedPackage)?;

        Ok(Self {
            universe_id: universe.universe_id(),
            universe_epoch: universe.universe_epoch(),
            controller_id: runtime.controller_id(),
            controller_epoch: runtime.controller_epoch(),
            session_id: binding.session_id,
            session_epoch: binding.expected_session_epoch,
            package_fingerprint: package.fingerprint(),
            artifact_fingerprint,
            profile_fingerprint: package.profile_fingerprint(),
            target_state_hash: binding.expected_target_state_hash,
            cpu_state: runtime.cpu_state(),
            virtual_timestamp_ms: runtime.virtual_time_ms(),
            scan_sequence: runtime.scan_sequence(),
            event_sequence: runtime.event_sequence(),
            publication_boundary,
        })
    }

    pub fn same_runtime_epoch(self, other: Self) -> bool {
        self.universe_id.0 == other.universe_id.0
            && self.universe_epoch == other.universe_epoch
            && self.controller_id.0 == other.controller_id.0
            && self.controller_epoch == other.controller_epoch
            && self.session_id.0 == other.session_id.0
            && self.session_epoch == other.session_epoch
            && self.artifact_fingerprint.0 == other.artifact_fingerprint.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContextError {
    UnknownSession,
    SessionNotOnline(SessionState),
    StaleUniverseEpoch,
    StaleControllerEpoch,
    StaleSessionEpoch,
    StaleTargetState,
    TargetUnavailable,
    NoLoadedPackage,
    SessionPackageMismatch,
}

impl fmt::Display for ContextError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "observation context rejected: {self:?}")
    }
}

impl Error for ContextError {}
