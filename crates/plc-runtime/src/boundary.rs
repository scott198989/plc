use alloc::collections::BTreeMap;

use crate::{
    ChannelDefinition, ChannelDirection, ChannelId, Hash32,
    hash::SemanticHasher,
    model::{CanonicalValue, ValueType},
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UniverseId(pub u128);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VirtualControllerId(pub u128);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CommandId(pub u128);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Quality {
    Good = 1,
    Uncertain = 2,
    Bad = 3,
    NotPresent = 4,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum DeliveryReason {
    RunOutputCommit = 1,
    CpuModeDefault = 2,
    FatalFaultDefault = 3,
    HardwareSuppressed = 4,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RawInput {
    pub channel_id: ChannelId,
    pub value_type: ValueType,
    pub canonical_value: CanonicalValue,
    pub accepted_event_sequence: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeliveredOutput {
    pub channel_id: ChannelId,
    pub value_type: ValueType,
    pub canonical_value: CanonicalValue,
    pub quality: Quality,
    pub suppressed: bool,
    pub delivery_reason: DeliveryReason,
    pub delivery_event_sequence: u64,
    pub output_commit_scan_sequence: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InputCommand {
    pub command_id: CommandId,
    pub idempotency_key: u128,
    pub controller_id: VirtualControllerId,
    pub expected_controller_epoch: u64,
    pub channel_id: ChannelId,
    pub value: CanonicalValue,
    pub audit_provenance_hash: Hash32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InputReceipt {
    pub command_id: CommandId,
    pub accepted_event_sequence: u64,
    pub virtual_timestamp_ms: u64,
    pub duplicate: bool,
    pub result_hash: Hash32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VirtualIoBoundary {
    pub controller_id: VirtualControllerId,
    pub schema_version: u32,
    raw_inputs: BTreeMap<ChannelId, RawInput>,
    delivered_outputs: BTreeMap<ChannelId, DeliveredOutput>,
}

impl VirtualIoBoundary {
    pub(crate) fn configured(
        controller_id: VirtualControllerId,
        channels: &[ChannelDefinition],
        event_sequence: u64,
    ) -> Self {
        let mut raw_inputs = BTreeMap::new();
        let mut delivered_outputs = BTreeMap::new();
        for channel in channels {
            match channel.direction {
                ChannelDirection::Input => {
                    raw_inputs.insert(
                        channel.id,
                        RawInput {
                            channel_id: channel.id,
                            value_type: channel.value_type,
                            canonical_value: channel.canonical_default,
                            accepted_event_sequence: event_sequence,
                        },
                    );
                }
                ChannelDirection::Output => {
                    delivered_outputs.insert(
                        channel.id,
                        DeliveredOutput {
                            channel_id: channel.id,
                            value_type: channel.value_type,
                            canonical_value: channel.canonical_default,
                            quality: Quality::Good,
                            suppressed: false,
                            delivery_reason: DeliveryReason::CpuModeDefault,
                            delivery_event_sequence: event_sequence,
                            output_commit_scan_sequence: None,
                        },
                    );
                }
            }
        }
        Self {
            controller_id,
            schema_version: 1,
            raw_inputs,
            delivered_outputs,
        }
    }

    pub(crate) fn empty(controller_id: VirtualControllerId) -> Self {
        Self {
            controller_id,
            schema_version: 1,
            raw_inputs: BTreeMap::new(),
            delivered_outputs: BTreeMap::new(),
        }
    }

    pub fn raw_input(&self, id: ChannelId) -> Option<&RawInput> {
        self.raw_inputs.get(&id)
    }

    pub fn delivered_output(&self, id: ChannelId) -> Option<&DeliveredOutput> {
        self.delivered_outputs.get(&id)
    }

    pub fn raw_inputs(&self) -> impl ExactSizeIterator<Item = &RawInput> {
        self.raw_inputs.values()
    }

    pub fn delivered_outputs(&self) -> impl ExactSizeIterator<Item = &DeliveredOutput> {
        self.delivered_outputs.values()
    }

    pub(crate) fn input_value_type(&self, id: ChannelId) -> Option<ValueType> {
        self.raw_inputs.get(&id).map(|input| input.value_type)
    }

    pub(crate) fn set_raw(&mut self, id: ChannelId, value: CanonicalValue, event_sequence: u64) {
        let input = self
            .raw_inputs
            .get_mut(&id)
            .expect("validated input channel must exist");
        input.canonical_value = value;
        input.accepted_event_sequence = event_sequence;
    }

    pub(crate) fn reset_raw_defaults(
        &mut self,
        channels: &[ChannelDefinition],
        event_sequence: u64,
    ) {
        for channel in channels {
            if channel.direction == ChannelDirection::Input {
                self.set_raw(channel.id, channel.canonical_default, event_sequence);
            }
        }
    }

    pub(crate) fn deliver_defaults(
        &mut self,
        channels: &[ChannelDefinition],
        event_sequence: u64,
        reason: DeliveryReason,
    ) {
        debug_assert!(matches!(
            reason,
            DeliveryReason::CpuModeDefault | DeliveryReason::FatalFaultDefault
        ));
        for channel in channels {
            if channel.direction == ChannelDirection::Output {
                let output = self
                    .delivered_outputs
                    .get_mut(&channel.id)
                    .expect("validated output channel must exist");
                output.canonical_value = channel.canonical_default;
                output.quality = Quality::Good;
                output.suppressed = false;
                output.delivery_reason = reason;
                output.delivery_event_sequence = event_sequence;
                output.output_commit_scan_sequence = None;
            }
        }
    }

    pub(crate) fn commit_outputs(
        &mut self,
        effective: &BTreeMap<ChannelId, CanonicalValue>,
        event_sequence: u64,
        scan_sequence: u64,
    ) {
        for (id, output) in &mut self.delivered_outputs {
            output.canonical_value = *effective
                .get(id)
                .expect("effective output must cover every configured output");
            output.quality = Quality::Good;
            output.suppressed = false;
            output.delivery_reason = DeliveryReason::RunOutputCommit;
            output.delivery_event_sequence = event_sequence;
            output.output_commit_scan_sequence = Some(scan_sequence);
        }
    }

    pub(crate) fn apply_output_delivery_override(
        &mut self,
        channel_id: ChannelId,
        delivered_value: CanonicalValue,
        quality: Quality,
        suppressed: bool,
        event_sequence: u64,
    ) {
        let output = self
            .delivered_outputs
            .get_mut(&channel_id)
            .expect("validated output channel must exist");
        output.canonical_value = delivered_value;
        output.quality = quality;
        output.suppressed = suppressed;
        output.delivery_reason = DeliveryReason::HardwareSuppressed;
        output.delivery_event_sequence = event_sequence;
    }

    pub(crate) fn encode(&self, hasher: &mut SemanticHasher) {
        hasher.u128(self.controller_id.0);
        hasher.u32(self.schema_version);
        hasher.u64(self.raw_inputs.len() as u64);
        for input in self.raw_inputs.values() {
            hasher.u32(input.channel_id.0);
            hasher.u8(input.value_type as u8);
            input.canonical_value.encode(hasher);
            hasher.u64(input.accepted_event_sequence);
        }
        hasher.u64(self.delivered_outputs.len() as u64);
        for output in self.delivered_outputs.values() {
            hasher.u32(output.channel_id.0);
            hasher.u8(output.value_type as u8);
            output.canonical_value.encode(hasher);
            hasher.u8(output.quality as u8);
            hasher.bool(output.suppressed);
            hasher.u8(output.delivery_reason as u8);
            hasher.u64(output.delivery_event_sequence);
            match output.output_commit_scan_sequence {
                Some(sequence) => {
                    hasher.bool(true);
                    hasher.u64(sequence);
                }
                None => hasher.bool(false),
            }
        }
    }
}
