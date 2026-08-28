#![allow(clippy::missing_errors_doc)]

//! Deterministic, hash-chained autosave journal and recovery.

use std::collections::BTreeSet;
use std::fmt;

use crate::engine::Engine;
use crate::hash::{Sha256Digest, sha256};
use crate::json::{
    JsonLimits, JsonValue, canonical_json, parse_json, require_only_fields, required,
};
use crate::model::{
    CommandEnvelope, CommandOutcome, DomainCommandResult, ObjectId, Project, TransactionId, Uuid,
    envelope_from_json, envelope_to_json,
};

const MAGIC: &[u8; 8] = b"VLABJNL1";
const FORMAT_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct JournalLimits {
    pub max_bytes: usize,
    pub max_records: usize,
    pub max_record_bytes: usize,
}

impl Default for JournalLimits {
    fn default() -> Self {
        Self {
            max_bytes: 16 * 1024 * 1024,
            max_records: 10_000,
            max_record_bytes: 1024 * 1024,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum JournalError {
    InvalidResult,
    BaseHashMismatch,
    DuplicateIdentity,
    SequenceMismatch,
    ChainMismatch,
    ReplayRejected(u64),
    ReplayDiverged(u64),
    Truncated,
    TrailingData,
    BadMagic,
    UnsupportedVersion(u32),
    IntegrityMismatch,
    NonCanonicalRecord(u64),
    InvalidRecord(u64),
    LimitExceeded(&'static str),
    IntegerOverflow,
    InvalidBaseProject,
}

impl fmt::Display for JournalError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidResult => {
                formatter.write_str("only committed command results can be journaled")
            }
            Self::BaseHashMismatch => formatter.write_str("journal base hash does not match"),
            Self::DuplicateIdentity => {
                formatter.write_str("duplicate command or transaction identity")
            }
            Self::SequenceMismatch => formatter.write_str("journal sequence is not contiguous"),
            Self::ChainMismatch => formatter.write_str("journal hash chain does not match"),
            Self::ReplayRejected(sequence) => {
                write!(formatter, "journal replay rejected record {sequence}")
            }
            Self::ReplayDiverged(sequence) => {
                write!(formatter, "journal replay diverged at record {sequence}")
            }
            Self::Truncated => formatter.write_str("truncated autosave journal"),
            Self::TrailingData => formatter.write_str("trailing autosave journal data"),
            Self::BadMagic => formatter.write_str("invalid autosave journal magic"),
            Self::UnsupportedVersion(version) => {
                write!(formatter, "unsupported journal version {version}")
            }
            Self::IntegrityMismatch => {
                formatter.write_str("autosave journal integrity digest does not match")
            }
            Self::NonCanonicalRecord(sequence) => {
                write!(formatter, "record {sequence} is not canonical JSON")
            }
            Self::InvalidRecord(sequence) => write!(formatter, "record {sequence} is invalid"),
            Self::LimitExceeded(limit) => write!(formatter, "journal limit exceeded: {limit}"),
            Self::IntegerOverflow => formatter.write_str("journal integer overflow"),
            Self::InvalidBaseProject => {
                formatter.write_str("base project violates kernel invariants")
            }
        }
    }
}

impl std::error::Error for JournalError {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JournalRecord {
    pub sequence: u64,
    pub schema_version: u32,
    pub previous_record_hash: Sha256Digest,
    pub before_project_hash: Sha256Digest,
    pub after_project_hash: Sha256Digest,
    pub transaction_id: TransactionId,
    pub affected_object_ids: Vec<ObjectId>,
    pub envelope: CommandEnvelope,
    pub record_hash: Sha256Digest,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Journal {
    pub base_project_hash: Sha256Digest,
    records: Vec<JournalRecord>,
}

impl Journal {
    #[must_use]
    pub const fn new(base_project_hash: Sha256Digest) -> Self {
        Self {
            base_project_hash,
            records: Vec::new(),
        }
    }

    #[must_use]
    pub fn records(&self) -> &[JournalRecord] {
        &self.records
    }

    pub fn append(
        &mut self,
        envelope: CommandEnvelope,
        result: &DomainCommandResult,
    ) -> Result<&JournalRecord, JournalError> {
        if result.outcome != CommandOutcome::Committed
            || result.transaction_id != envelope.transaction_id
            || result.after_project_hash.is_none()
        {
            return Err(JournalError::InvalidResult);
        }
        if result
            .affected_object_ids
            .windows(2)
            .any(|pair| pair[0] >= pair[1])
        {
            return Err(JournalError::InvalidResult);
        }
        if self.records.iter().any(|record| {
            record.transaction_id == envelope.transaction_id
                || record.envelope.command_id == envelope.command_id
        }) {
            return Err(JournalError::DuplicateIdentity);
        }
        let expected_before = self
            .records
            .last()
            .map_or(self.base_project_hash, |record| record.after_project_hash);
        if result.before_project_hash != expected_before {
            return Err(JournalError::ChainMismatch);
        }
        let sequence = u64::try_from(self.records.len())
            .map_err(|_| JournalError::IntegerOverflow)?
            .checked_add(1)
            .ok_or(JournalError::IntegerOverflow)?;
        let previous_record_hash = self
            .records
            .last()
            .map_or(Sha256Digest([0; 32]), |record| record.record_hash);
        let mut record = JournalRecord {
            sequence,
            schema_version: 1,
            previous_record_hash,
            before_project_hash: result.before_project_hash,
            after_project_hash: result
                .after_project_hash
                .ok_or(JournalError::InvalidResult)?,
            transaction_id: envelope.transaction_id,
            affected_object_ids: result.affected_object_ids.clone(),
            envelope,
            record_hash: Sha256Digest([0; 32]),
        };
        record.record_hash = record_identity_hash(&record);
        self.records.push(record);
        self.records.last().ok_or(JournalError::IntegerOverflow)
    }

    pub fn encode(&self) -> Result<Vec<u8>, JournalError> {
        self.validate_chain()?;
        let count = u32::try_from(self.records.len()).map_err(|_| JournalError::IntegerOverflow)?;
        let mut output = Vec::new();
        output.extend_from_slice(MAGIC);
        output.extend_from_slice(&FORMAT_VERSION.to_le_bytes());
        output.extend_from_slice(&count.to_le_bytes());
        output.extend_from_slice(&self.base_project_hash.0);
        for record in &self.records {
            let bytes = canonical_json(&record_json(record));
            let size = u32::try_from(bytes.len()).map_err(|_| JournalError::IntegerOverflow)?;
            output.extend_from_slice(&size.to_le_bytes());
            output.extend_from_slice(&bytes);
        }
        output.extend_from_slice(&sha256(&output).0);
        Ok(output)
    }

    pub fn decode(input: &[u8], limits: JournalLimits) -> Result<Self, JournalError> {
        if input.len() > limits.max_bytes {
            return Err(JournalError::LimitExceeded("journal bytes"));
        }
        if input.len() < MAGIC.len() + 8 + 32 + 32 {
            return Err(JournalError::Truncated);
        }
        let payload_len = input.len().checked_sub(32).ok_or(JournalError::Truncated)?;
        let mut trailer = [0_u8; 32];
        trailer.copy_from_slice(&input[payload_len..]);
        if sha256(&input[..payload_len]) != Sha256Digest(trailer) {
            return Err(JournalError::IntegrityMismatch);
        }
        let mut reader = Reader::new(&input[..payload_len]);
        if reader.take(MAGIC.len())? != MAGIC {
            return Err(JournalError::BadMagic);
        }
        let version = reader.u32()?;
        if version != FORMAT_VERSION {
            return Err(JournalError::UnsupportedVersion(version));
        }
        let count = usize::try_from(reader.u32()?).map_err(|_| JournalError::IntegerOverflow)?;
        if count > limits.max_records {
            return Err(JournalError::LimitExceeded("record count"));
        }
        let mut base = [0_u8; 32];
        base.copy_from_slice(reader.take(32)?);
        let mut records = Vec::with_capacity(count);
        for index in 0..count {
            let sequence = u64::try_from(index)
                .map_err(|_| JournalError::IntegerOverflow)?
                .checked_add(1)
                .ok_or(JournalError::IntegerOverflow)?;
            let size = usize::try_from(reader.u32()?).map_err(|_| JournalError::IntegerOverflow)?;
            if size > limits.max_record_bytes {
                return Err(JournalError::LimitExceeded("record bytes"));
            }
            let bytes = reader.take(size)?;
            let value = parse_json(bytes, JsonLimits::default())
                .map_err(|_| JournalError::InvalidRecord(sequence))?;
            if canonical_json(&value) != bytes {
                return Err(JournalError::NonCanonicalRecord(sequence));
            }
            records.push(record_from_json(&value, sequence)?);
        }
        if !reader.is_empty() {
            return Err(JournalError::TrailingData);
        }
        let journal = Self {
            base_project_hash: Sha256Digest(base),
            records,
        };
        journal.validate_chain()?;
        Ok(journal)
    }

    fn validate_chain(&self) -> Result<(), JournalError> {
        let mut prior_record_hash = Sha256Digest([0; 32]);
        let mut prior_project_hash = self.base_project_hash;
        let mut commands = BTreeSet::new();
        let mut transactions = BTreeSet::new();
        for (index, record) in self.records.iter().enumerate() {
            let sequence = u64::try_from(index)
                .map_err(|_| JournalError::IntegerOverflow)?
                .checked_add(1)
                .ok_or(JournalError::IntegerOverflow)?;
            if record.sequence != sequence || record.schema_version != 1 {
                return Err(JournalError::SequenceMismatch);
            }
            if record.previous_record_hash != prior_record_hash
                || record.before_project_hash != prior_project_hash
                || record.transaction_id != record.envelope.transaction_id
                || record.record_hash != record_identity_hash(record)
            {
                return Err(JournalError::ChainMismatch);
            }
            if record
                .affected_object_ids
                .windows(2)
                .any(|pair| pair[0] >= pair[1])
            {
                return Err(JournalError::ChainMismatch);
            }
            if !commands.insert(record.envelope.command_id)
                || !transactions.insert(record.transaction_id)
            {
                return Err(JournalError::DuplicateIdentity);
            }
            prior_record_hash = record.record_hash;
            prior_project_hash = record.after_project_hash;
        }
        Ok(())
    }
}

/// Replays all validated journal records against a clone of the saved base.
/// Any rejection or hash divergence aborts recovery without mutating `base`.
pub fn recover_from_journal(base: &Project, journal: &Journal) -> Result<Project, JournalError> {
    base.validate()
        .map_err(|_| JournalError::InvalidBaseProject)?;
    journal.validate_chain()?;
    if base.document_hash() != journal.base_project_hash {
        return Err(JournalError::BaseHashMismatch);
    }
    let mut engine = Engine::new(base.clone()).map_err(|_| JournalError::InvalidBaseProject)?;
    for record in &journal.records {
        if engine.project().document_hash() != record.before_project_hash {
            return Err(JournalError::ReplayDiverged(record.sequence));
        }
        let result = engine.execute(&record.envelope);
        if result.outcome != CommandOutcome::Committed {
            return Err(JournalError::ReplayRejected(record.sequence));
        }
        if result.after_project_hash != Some(record.after_project_hash)
            || result.affected_object_ids != record.affected_object_ids
            || engine.project().document_hash() != record.after_project_hash
        {
            return Err(JournalError::ReplayDiverged(record.sequence));
        }
    }
    Ok(engine.into_project())
}

fn record_identity_hash(record: &JournalRecord) -> Sha256Digest {
    let mut identity = record.clone();
    identity.record_hash = Sha256Digest([0; 32]);
    sha256(&canonical_json(&record_json(&identity)))
}

fn record_json(record: &JournalRecord) -> JsonValue {
    JsonValue::object([
        (
            "sequence".to_owned(),
            JsonValue::from(record.sequence.to_string()),
        ),
        (
            "schemaVersion".to_owned(),
            JsonValue::from(record.schema_version),
        ),
        (
            "previousRecordHash".to_owned(),
            JsonValue::from(record.previous_record_hash.to_hex()),
        ),
        (
            "beforeProjectHash".to_owned(),
            JsonValue::from(record.before_project_hash.to_hex()),
        ),
        (
            "afterProjectHash".to_owned(),
            JsonValue::from(record.after_project_hash.to_hex()),
        ),
        (
            "transactionId".to_owned(),
            JsonValue::from(record.transaction_id.to_string()),
        ),
        (
            "affectedObjectIds".to_owned(),
            JsonValue::Array(
                record
                    .affected_object_ids
                    .iter()
                    .map(ToString::to_string)
                    .map(JsonValue::from)
                    .collect(),
            ),
        ),
        ("envelope".to_owned(), envelope_to_json(&record.envelope)),
        (
            "recordHash".to_owned(),
            JsonValue::from(record.record_hash.to_hex()),
        ),
    ])
}

fn record_from_json(value: &JsonValue, sequence_hint: u64) -> Result<JournalRecord, JournalError> {
    let object = value
        .as_object()
        .map_err(|_| JournalError::InvalidRecord(sequence_hint))?;
    require_only_fields(
        object,
        &[
            "sequence",
            "schemaVersion",
            "previousRecordHash",
            "beforeProjectHash",
            "afterProjectHash",
            "transactionId",
            "affectedObjectIds",
            "envelope",
            "recordHash",
        ],
    )
    .map_err(|_| JournalError::InvalidRecord(sequence_hint))?;
    let digest = |name: &'static str| -> Result<Sha256Digest, JournalError> {
        let source = required(object, name)
            .map_err(|_| JournalError::InvalidRecord(sequence_hint))?
            .as_str()
            .map_err(|_| JournalError::InvalidRecord(sequence_hint))?;
        let parsed = Sha256Digest::from_hex(source)
            .map_err(|_| JournalError::InvalidRecord(sequence_hint))?;
        if parsed.to_hex() != source {
            return Err(JournalError::InvalidRecord(sequence_hint));
        }
        Ok(parsed)
    };
    let transaction_id = TransactionId(
        Uuid::parse(
            required(object, "transactionId")
                .map_err(|_| JournalError::InvalidRecord(sequence_hint))?
                .as_str()
                .map_err(|_| JournalError::InvalidRecord(sequence_hint))?,
        )
        .map_err(|_| JournalError::InvalidRecord(sequence_hint))?,
    );
    let affected_object_ids = required(object, "affectedObjectIds")
        .map_err(|_| JournalError::InvalidRecord(sequence_hint))?
        .as_array()
        .map_err(|_| JournalError::InvalidRecord(sequence_hint))?
        .iter()
        .map(|value| {
            Uuid::parse(
                value
                    .as_str()
                    .map_err(|_| JournalError::InvalidRecord(sequence_hint))?,
            )
            .map(ObjectId)
            .map_err(|_| JournalError::InvalidRecord(sequence_hint))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let sequence_text = required(object, "sequence")
        .map_err(|_| JournalError::InvalidRecord(sequence_hint))?
        .as_str()
        .map_err(|_| JournalError::InvalidRecord(sequence_hint))?;
    if sequence_text.is_empty()
        || (sequence_text.len() > 1 && sequence_text.starts_with('0'))
        || !sequence_text.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err(JournalError::InvalidRecord(sequence_hint));
    }
    Ok(JournalRecord {
        sequence: sequence_text
            .parse()
            .map_err(|_| JournalError::InvalidRecord(sequence_hint))?,
        schema_version: u32::try_from(
            required(object, "schemaVersion")
                .map_err(|_| JournalError::InvalidRecord(sequence_hint))?
                .as_u64()
                .map_err(|_| JournalError::InvalidRecord(sequence_hint))?,
        )
        .map_err(|_| JournalError::InvalidRecord(sequence_hint))?,
        previous_record_hash: digest("previousRecordHash")?,
        before_project_hash: digest("beforeProjectHash")?,
        after_project_hash: digest("afterProjectHash")?,
        transaction_id,
        affected_object_ids,
        envelope: envelope_from_json(
            required(object, "envelope").map_err(|_| JournalError::InvalidRecord(sequence_hint))?,
        )
        .map_err(|_| JournalError::InvalidRecord(sequence_hint))?,
        record_hash: digest("recordHash")?,
    })
}

struct Reader<'a> {
    bytes: &'a [u8],
    index: usize,
}

impl<'a> Reader<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, index: 0 }
    }

    fn take(&mut self, count: usize) -> Result<&'a [u8], JournalError> {
        let end = self
            .index
            .checked_add(count)
            .ok_or(JournalError::IntegerOverflow)?;
        let value = self
            .bytes
            .get(self.index..end)
            .ok_or(JournalError::Truncated)?;
        self.index = end;
        Ok(value)
    }

    fn u32(&mut self) -> Result<u32, JournalError> {
        let mut bytes = [0_u8; 4];
        bytes.copy_from_slice(self.take(4)?);
        Ok(u32::from_le_bytes(bytes))
    }

    const fn is_empty(&self) -> bool {
        self.index == self.bytes.len()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use crate::{
        CommandContext, CommandEnvelope, DomainCommand, Engine, ObjectId, ProfilePin, Project,
        Sha256Digest, TransactionId, Uuid,
    };

    use super::{Journal, JournalError, JournalLimits, recover_from_journal};

    fn fixture() -> Project {
        Project::new(
            Uuid::deterministic_v4(b"journal-document", 1),
            ObjectId(Uuid::deterministic_v4(b"journal-root", 1)),
            "Journal",
            ProfilePin {
                id: "training".to_owned(),
                version: "1".to_owned(),
                manifest_hash: Sha256Digest([5; 32]),
            },
        )
    }

    #[test]
    fn round_trip_and_recovery_replay_exactly() {
        let base = fixture();
        let root = base.root_id();
        let mut engine = Engine::new(base.clone()).expect("engine");
        let envelope = CommandEnvelope {
            command_id: Uuid::deterministic_v4(b"journal-command", 1),
            transaction_id: TransactionId(Uuid::deterministic_v4(b"journal-transaction", 1)),
            expected_document_revision: engine.project().document_revision(),
            expected_object_revisions: BTreeMap::from([(
                root,
                engine.project().object(root).expect("root").object_revision,
            )]),
            context: CommandContext {
                actor_id: "test".to_owned(),
                can_mutate: true,
            },
            command: DomainCommand::Rename {
                object_id: root,
                display_name: "Recovered".to_owned(),
            },
        };
        let result = engine.execute(&envelope);
        let mut journal = Journal::new(base.document_hash());
        journal.append(envelope, &result).expect("append");
        let bytes = journal.encode().expect("encode");
        let decoded = Journal::decode(&bytes, JournalLimits::default()).expect("decode");
        let recovered = recover_from_journal(&base, &decoded).expect("recover");
        assert_eq!(recovered.document_hash(), engine.project().document_hash());

        let mut corrupt = bytes;
        let middle = corrupt.len() / 2;
        corrupt[middle] ^= 1;
        assert_eq!(
            Journal::decode(&corrupt, JournalLimits::default()),
            Err(JournalError::IntegrityMismatch)
        );
    }
}
