#![allow(clippy::too_many_lines)]

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::sync::atomic::{AtomicBool, Ordering};

use plc_core::{
    CommandContext, CommandEnvelope, CommandOutcome, DecodeLimits, DomainCommand, Engine, Journal,
    JournalError, JournalLimits, LogicalPackage, MigrationDefault, MigrationError, MigrationReport,
    MigrationStep, MigrationStepOutput, ObjectId, PackageError, PayloadValue, ProfilePin, Project,
    ProjectArchive, Sha256Digest, SimulatorExtension, TransactionId, Uuid, decode_project_package,
    encode_project_package, inspect_package_manifest, migrate_project, recover_from_journal,
    sha256,
};

fn fixture(label: &str) -> Project {
    Project::new(
        Uuid::deterministic_v4(b"persistence-document", 1),
        ObjectId(Uuid::deterministic_v4(b"persistence-root", 1)),
        label,
        ProfilePin {
            id: "EDU-21 Core".to_owned(),
            version: "1.0.0".to_owned(),
            manifest_hash: Sha256Digest([0x21; 32]),
        },
    )
}

fn rename_envelope(project: &Project, ordinal: u64, display_name: &str) -> CommandEnvelope {
    let root = project.root_id();
    CommandEnvelope {
        command_id: Uuid::deterministic_v4(b"persistence-command", ordinal),
        transaction_id: TransactionId(Uuid::deterministic_v4(b"persistence-transaction", ordinal)),
        expected_document_revision: project.document_revision(),
        expected_object_revisions: BTreeMap::from([(
            root,
            project.object(root).expect("root").object_revision,
        )]),
        context: CommandContext {
            actor_id: "persistence-test".to_owned(),
            can_mutate: true,
        },
        command: DomainCommand::Rename {
            object_id: root,
            display_name: display_name.to_owned(),
        },
    }
}

#[allow(clippy::unnecessary_wraps)]
fn no_op(_: &mut Project) -> Result<MigrationStepOutput, String> {
    Ok(MigrationStepOutput::default())
}

fn rename_for_migration(
    project: &mut Project,
    old_name: &str,
    new_name: &str,
    ordinal: u64,
) -> Result<MigrationStepOutput, String> {
    let root = project.root_id();
    let current = &project.object(root).ok_or("missing root")?.display_name;
    if current == new_name {
        return Ok(MigrationStepOutput::default());
    }
    if current != old_name {
        return Err(format!("unexpected source name {current}"));
    }
    let mut engine = Engine::new(project.clone()).map_err(|error| format!("{error:?}"))?;
    let envelope = rename_envelope(engine.project(), ordinal, new_name);
    let result = engine.execute(&envelope);
    if result.outcome != CommandOutcome::Committed {
        return Err(format!("rename rejected: {:?}", result.diagnostics));
    }
    *project = engine.into_project();
    Ok(MigrationStepOutput {
        affected_object_ids: vec![root],
        defaults_introduced: vec![MigrationDefault {
            object_id: root,
            field_path: "displayName".to_owned(),
            canonical_value: new_name.to_owned(),
        }],
        warnings: vec![format!("normalized legacy project label to {new_name}")],
        ..MigrationStepOutput::default()
    })
}

fn one_to_two(project: &mut Project) -> Result<MigrationStepOutput, String> {
    rename_for_migration(project, "Legacy", "Schema Two", 102)
}

fn two_to_three(project: &mut Project) -> Result<MigrationStepOutput, String> {
    rename_for_migration(project, "Schema Two", "Schema Three", 203)
}

fn always_fails(_: &mut Project) -> Result<MigrationStepOutput, String> {
    Err("injected blocking failure".to_owned())
}

fn changes_without_reporting(project: &mut Project) -> Result<MigrationStepOutput, String> {
    let output = rename_for_migration(project, "Legacy", "Unreported", 404)?;
    if output == MigrationStepOutput::default() {
        Ok(output)
    } else {
        Ok(MigrationStepOutput::default())
    }
}

static NONDETERMINISTIC_TOGGLE: AtomicBool = AtomicBool::new(false);

#[allow(clippy::unnecessary_wraps)]
fn nondeterministic_report(_: &mut Project) -> Result<MigrationStepOutput, String> {
    let prior = NONDETERMINISTIC_TOGGLE.fetch_xor(true, Ordering::SeqCst);
    Ok(MigrationStepOutput {
        warnings: vec![if prior { "second" } else { "first" }.to_owned()],
        ..MigrationStepOutput::default()
    })
}

fn render_migration_report(report: &MigrationReport) -> String {
    let mut rendered = format!(
        "sourceVersion={}\ntargetVersion={}\nbackupHash={}\nresultingHash={}\n",
        report.source_version, report.target_version, report.backup_hash, report.resulting_hash
    );
    for (index, change) in report.changes.iter().enumerate() {
        write!(
            rendered,
            "step[{index}]={}->{}:{}\nbefore={}\nafter={}\naffected={:?}\n",
            change.from_version,
            change.to_version,
            change.name,
            change.before_hash,
            change.after_hash,
            change.affected_object_ids
        )
        .expect("writing to a String is infallible");
        write!(
            rendered,
            "mappings={:?}\ndefaults={:?}\nwarnings={:?}\nunsupported={:?}\n",
            change.identity_mappings,
            change.defaults_introduced,
            change.warnings,
            change.unsupported_features
        )
        .expect("writing to a String is infallible");
    }
    rendered
}

fn logical_container(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut output = Vec::new();
    output.extend_from_slice(b"VLABPKG1");
    output.extend_from_slice(&1_u32.to_le_bytes());
    output.extend_from_slice(
        &u32::try_from(entries.len())
            .expect("test entry count")
            .to_le_bytes(),
    );
    for (path, data) in entries {
        output.extend_from_slice(&u32::try_from(path.len()).expect("test path").to_le_bytes());
        output.extend_from_slice(&u64::try_from(data.len()).expect("test data").to_le_bytes());
        output.extend_from_slice(path.as_bytes());
        output.extend_from_slice(data);
    }
    output.extend_from_slice(&sha256(&output).0);
    output
}

fn replace_manifest(package: &[u8], transform: impl FnOnce(String) -> String) -> Vec<u8> {
    let logical =
        LogicalPackage::decode(package, DecodeLimits::default()).expect("logical package");
    let mut entries = logical.entries().clone();
    let manifest = String::from_utf8(entries["manifest.json"].clone()).expect("manifest UTF-8");
    entries.insert("manifest.json".to_owned(), transform(manifest).into_bytes());
    LogicalPackage::new(entries)
        .expect("safe entries")
        .encode()
        .expect("repacked container")
}

fn refresh_manifest_hash(manifest: String) -> String {
    let marker = "\"packageHash\":\"";
    let field_start = manifest.find(marker).expect("package hash field");
    let value_start = field_start + marker.len();
    let value_end = value_start + 64;
    let field_end = value_end + 2;
    assert_eq!(&manifest[value_end..field_end], "\",");
    let mut identity = manifest.clone();
    identity.replace_range(field_start..field_end, "");
    let digest = sha256(identity.as_bytes()).to_hex();
    let mut refreshed = manifest;
    refreshed.replace_range(value_start..value_end, &digest);
    refreshed
}

fn journal_frames(bytes: &[u8]) -> (Vec<u8>, Vec<Vec<u8>>) {
    let payload_end = bytes.len() - 32;
    let mut header = bytes[..48].to_vec();
    let count = u32::from_le_bytes(bytes[12..16].try_into().expect("count")) as usize;
    let mut offset = 48;
    let mut frames = Vec::new();
    for _ in 0..count {
        let size = u32::from_le_bytes(bytes[offset..offset + 4].try_into().expect("size")) as usize;
        let end = offset + 4 + size;
        frames.push(bytes[offset..end].to_vec());
        offset = end;
    }
    assert_eq!(offset, payload_end);
    header[12..16].copy_from_slice(
        &u32::try_from(frames.len())
            .expect("frame count")
            .to_le_bytes(),
    );
    (header, frames)
}

fn build_journal(mut header: Vec<u8>, frames: &[Vec<u8>]) -> Vec<u8> {
    header[12..16].copy_from_slice(
        &u32::try_from(frames.len())
            .expect("frame count")
            .to_le_bytes(),
    );
    for frame in frames {
        header.extend_from_slice(frame);
    }
    header.extend_from_slice(&sha256(&header).0);
    header
}

#[test]
fn canonical_round_trip_preserves_approved_extensions_and_inert_text() {
    let hostile_text = PayloadValue::Record(BTreeMap::from([
        (
            "commandLike".to_owned(),
            PayloadValue::from("powershell -Command Remove-Item C:\\\\training"),
        ),
        (
            "hostnameLike".to_owned(),
            PayloadValue::from("plc-controller.local"),
        ),
        (
            "protocolLike".to_owned(),
            PayloadValue::from("s7://127.0.0.1:102/rack/0"),
        ),
        (
            "urlLike".to_owned(),
            PayloadValue::from("https://example.invalid/project?id=1"),
        ),
    ]));
    let extension = SimulatorExtension::new("edu.lesson-notes", 1, hostile_text.clone())
        .expect("approved extension");
    let project = Project::new_with_simulator_extensions(
        Uuid::deterministic_v4(b"extension-document", 1),
        ObjectId(Uuid::deterministic_v4(b"extension-root", 1)),
        "Extension Round Trip",
        ProfilePin {
            id: "EDU-21 Core".to_owned(),
            version: "1.0.0".to_owned(),
            manifest_hash: Sha256Digest([0x21; 32]),
        },
        vec![extension],
    )
    .expect("project extensions");
    let original_document_hash = project.document_hash();
    let first = encode_project_package(&project, "persistence-test/1").expect("encode");
    let (opened, manifest) =
        decode_project_package(&first, DecodeLimits::default()).expect("decode");
    assert_eq!(opened.document_hash(), original_document_hash);
    assert_eq!(opened.saved_document_hash(), Some(manifest.package_hash));
    assert_eq!(
        opened
            .simulator_extensions()
            .next()
            .expect("preserved extension")
            .data(),
        &hostile_text
    );
    assert_eq!(
        encode_project_package(&opened, "persistence-test/1").expect("second encode"),
        first
    );

    let registry = [MigrationStep {
        from_version: 1,
        to_version: 2,
        name: "preserve-inert-extension",
        apply: no_op,
    }];
    let (migrated, report) = migrate_project(&opened, 1, 2, &registry).expect("migration");
    assert_eq!(migrated.document_hash(), original_document_hash);
    assert_eq!(report.backup.restore(), opened);

    let mut archive = ProjectArchive::default();
    let reference = archive
        .archive_verified_package(&first, DecodeLimits::default())
        .expect("archive");
    let new_document_id = Uuid::deterministic_v4(b"extension-retrieve", 1);
    let retrieved = archive
        .retrieve_as(reference, new_document_id, DecodeLimits::default())
        .expect("retrieve")
        .expect("archived entry");
    assert_eq!(retrieved.document_id(), new_document_id);
    assert_eq!(retrieved.root_id(), project.root_id());
    assert_eq!(
        retrieved
            .simulator_extensions()
            .next()
            .expect("retrieved extension")
            .data(),
        &hostile_text
    );
}

#[test]
fn migration_chain_matches_golden_and_rolls_back_every_failure() {
    let source = fixture("Legacy");
    let source_hash = source.document_hash();
    let registry = [
        MigrationStep {
            from_version: 1,
            to_version: 2,
            name: "legacy-to-two",
            apply: one_to_two,
        },
        MigrationStep {
            from_version: 2,
            to_version: 3,
            name: "two-to-three",
            apply: two_to_three,
        },
    ];
    let (migrated, report) = migrate_project(&source, 1, 3, &registry).expect("complete chain");
    let (repeated, repeated_report) =
        migrate_project(&source, 1, 3, &registry).expect("deterministic repeat");
    assert_eq!(migrated, repeated);
    assert_eq!(report, repeated_report);
    assert_eq!(report.backup_hash, source_hash);
    assert_eq!(report.backup.project(), &source);
    assert_eq!(report.backup.restore(), source);
    assert_eq!(report.changes.len(), 2);
    assert_eq!(
        render_migration_report(&report),
        include_str!("goldens/migration_chain_1_to_3.txt")
    );

    let before_downgrade = migrated.document_hash();
    assert_eq!(
        migrate_project(&migrated, 3, 2, &registry),
        Err(MigrationError::BackwardMigration)
    );
    assert_eq!(migrated.document_hash(), before_downgrade);
    assert_eq!(source.document_hash(), source_hash);

    let incomplete = &registry[..1];
    assert_eq!(
        migrate_project(&source, 1, 3, incomplete),
        Err(MigrationError::MissingStep(2))
    );
    assert_eq!(source.document_hash(), source_hash);

    let failing_registry = [
        registry[0],
        MigrationStep {
            from_version: 2,
            to_version: 3,
            name: "blocking-failure",
            apply: always_fails,
        },
    ];
    assert!(matches!(
        migrate_project(&source, 1, 3, &failing_registry),
        Err(MigrationError::StepFailed { .. })
    ));
    assert_eq!(source.document_hash(), source_hash);

    let unreported = [MigrationStep {
        from_version: 1,
        to_version: 2,
        name: "unreported-change",
        apply: changes_without_reporting,
    }];
    assert!(matches!(
        migrate_project(&source, 1, 2, &unreported),
        Err(MigrationError::InvalidResult(_))
    ));
    assert_eq!(source.document_hash(), source_hash);

    NONDETERMINISTIC_TOGGLE.store(false, Ordering::SeqCst);
    let nondeterministic = [MigrationStep {
        from_version: 1,
        to_version: 2,
        name: "nondeterministic-report",
        apply: nondeterministic_report,
    }];
    assert_eq!(
        migrate_project(&source, 1, 2, &nondeterministic),
        Err(MigrationError::NonDeterministic(
            "nondeterministic-report".to_owned()
        ))
    );
    assert_eq!(source.document_hash(), source_hash);
}

#[test]
fn journal_power_loss_corruption_reorder_and_duplication_fail_closed() {
    let base = fixture("Journal Base");
    let base_hash = base.document_hash();
    let mut engine = Engine::new(base.clone()).expect("engine");
    let mut journal = Journal::new(base_hash);
    for (ordinal, name) in [(1, "First"), (2, "Second")] {
        let envelope = rename_envelope(engine.project(), ordinal, name);
        let result = engine.execute(&envelope);
        assert_eq!(result.outcome, CommandOutcome::Committed);
        journal.append(envelope, &result).expect("append");
    }
    let encoded = journal.encode().expect("journal bytes");
    let decoded = Journal::decode(&encoded, JournalLimits::default()).expect("journal decode");
    let recovered = recover_from_journal(&base, &decoded).expect("recovery");
    assert_eq!(recovered.document_hash(), engine.project().document_hash());
    assert_eq!(base.document_hash(), base_hash);

    for limits in [
        JournalLimits {
            max_bytes: encoded.len() - 1,
            ..JournalLimits::default()
        },
        JournalLimits {
            max_records: 1,
            ..JournalLimits::default()
        },
        JournalLimits {
            max_record_bytes: 1,
            ..JournalLimits::default()
        },
    ] {
        assert!(matches!(
            Journal::decode(&encoded, limits),
            Err(JournalError::LimitExceeded(_))
        ));
    }

    for lost_bytes in [1_usize, 8, 31, 32, 33] {
        let truncated = &encoded[..encoded.len() - lost_bytes];
        assert!(Journal::decode(truncated, JournalLimits::default()).is_err());
        assert_eq!(base.document_hash(), base_hash);
    }

    let mut corrupt = encoded.clone();
    let corrupt_index = corrupt.len() / 2;
    corrupt[corrupt_index] ^= 0x80;
    assert_eq!(
        Journal::decode(&corrupt, JournalLimits::default()),
        Err(JournalError::IntegrityMismatch)
    );
    assert_eq!(base.document_hash(), base_hash);

    let (header, frames) = journal_frames(&encoded);
    let duplicate = build_journal(header.clone(), &[frames[0].clone(), frames[0].clone()]);
    assert!(matches!(
        Journal::decode(&duplicate, JournalLimits::default()),
        Err(JournalError::SequenceMismatch | JournalError::DuplicateIdentity)
    ));
    let reordered = build_journal(header, &[frames[1].clone(), frames[0].clone()]);
    assert!(matches!(
        Journal::decode(&reordered, JournalLimits::default()),
        Err(JournalError::SequenceMismatch | JournalError::ChainMismatch)
    ));

    let wrong_base = fixture("Different Baseline");
    assert_eq!(
        recover_from_journal(&wrong_base, &decoded),
        Err(JournalError::BaseHashMismatch)
    );
    assert_eq!(
        wrong_base.document_hash(),
        fixture("Different Baseline").document_hash()
    );
}

#[test]
fn archive_bomb_and_resource_limit_corpus_is_fail_closed() {
    let ordinary = logical_container(&[("a", b"123"), ("b", b"456")]);
    let cases = [
        DecodeLimits {
            max_package_bytes: ordinary.len() - 1,
            ..DecodeLimits::default()
        },
        DecodeLimits {
            max_entries: 1,
            ..DecodeLimits::default()
        },
        DecodeLimits {
            max_entry_bytes: 2,
            ..DecodeLimits::default()
        },
        DecodeLimits {
            max_total_entry_bytes: 5,
            ..DecodeLimits::default()
        },
        DecodeLimits {
            max_path_bytes: 0,
            ..DecodeLimits::default()
        },
        DecodeLimits {
            max_expansion_ratio: 0,
            ..DecodeLimits::default()
        },
    ];
    for limits in cases {
        assert!(matches!(
            LogicalPackage::decode(&ordinary, limits),
            Err(PackageError::LimitExceeded(_))
        ));
    }

    let image = logical_container(&[("assets/images/lesson.png", b"12")]);
    assert_eq!(
        LogicalPackage::decode(
            &image,
            DecodeLimits {
                max_image_bytes: 1,
                ..DecodeLimits::default()
            }
        ),
        Err(PackageError::LimitExceeded("image bytes"))
    );

    let package = encode_project_package(&fixture("Limits"), "limits/1").expect("project package");
    for limits in [
        DecodeLimits {
            max_json_depth: 1,
            ..DecodeLimits::default()
        },
        DecodeLimits {
            max_json_string_bytes: 3,
            ..DecodeLimits::default()
        },
        DecodeLimits {
            max_json_collection_items: 0,
            ..DecodeLimits::default()
        },
        DecodeLimits {
            max_json_values: 1,
            ..DecodeLimits::default()
        },
        DecodeLimits {
            max_total_objects: 0,
            ..DecodeLimits::default()
        },
    ] {
        assert!(decode_project_package(&package, limits).is_err());
    }
}

#[test]
fn path_confusion_link_external_and_undeclared_member_corpus_is_rejected() {
    for path in [
        "/absolute",
        "//server/share",
        "C:/device",
        "../escape",
        "safe/../escape",
        "safe/./alias",
        "safe/con",
        "safe/NUL.txt",
        "safe/COM1.log",
        "safe/trailing.",
        "safe/trailing ",
        "safe\\windows",
    ] {
        let bytes = logical_container(&[(path, b"x")]);
        assert!(matches!(
            LogicalPackage::decode(&bytes, DecodeLimits::default()),
            Err(PackageError::InvalidPath(_))
        ));
    }
    for entries in [
        vec![("A/file", b"1".as_slice()), ("a/file", b"2".as_slice())],
        vec![("same", b"1".as_slice()), ("same", b"2".as_slice())],
    ] {
        let bytes = logical_container(&entries);
        assert!(LogicalPackage::decode(&bytes, DecodeLimits::default()).is_err());
    }

    let package = encode_project_package(&fixture("Members"), "members/1").expect("package");
    let logical = LogicalPackage::decode(&package, DecodeLimits::default()).expect("logical");
    for path in [
        "project/symlink",
        "external/reference.json",
        "extensions/vendor/opaque.json",
        "undeclared/member.json",
    ] {
        let mut entries = logical.entries().clone();
        entries.insert(path.to_owned(), b"{}".to_vec());
        let attack = LogicalPackage::new(entries)
            .expect("physically safe attack")
            .encode()
            .expect("attack bytes");
        assert!(decode_project_package(&attack, DecodeLimits::default()).is_err());
    }
}

#[test]
fn unknown_closed_schema_and_corruption_cannot_replace_live_state() {
    let baseline = fixture("Live Baseline");
    let package = encode_project_package(&baseline, "schema/1").expect("package");
    let unknown_field = replace_manifest(&package, |manifest| {
        format!(
            "{},\"x-unknown\":true}}",
            manifest.strip_suffix('}').expect("object")
        )
    });
    assert_eq!(
        decode_project_package(&unknown_field, DecodeLimits::default()),
        Err(PackageError::InvalidManifest)
    );

    let future = replace_manifest(&package, |manifest| {
        let upgraded = manifest
            .replace("\"documentFormatVersion\":1", "\"documentFormatVersion\":2")
            .replace("\"minimumReaderVersion\":1", "\"minimumReaderVersion\":2");
        refresh_manifest_hash(upgraded)
    });
    assert_eq!(
        decode_project_package(&future, DecodeLimits::default()),
        Err(PackageError::UnsupportedDocumentVersion(2))
    );
    let inspection = inspect_package_manifest(&future, DecodeLimits::default())
        .expect("validated future metadata");
    assert!(!inspection.editable);
    assert_eq!(inspection.manifest.document_id, baseline.document_id());

    let mut session = plc_core::KernelSession::from_project(baseline).expect("session");
    let live_hash = session.project().document_hash();
    assert!(
        session
            .replace_from_package(&future, DecodeLimits::default())
            .is_err()
    );
    assert_eq!(session.project().document_hash(), live_hash);
    let mut corrupt = package;
    let index = corrupt.len() / 2;
    corrupt[index] ^= 0x40;
    assert!(
        session
            .replace_from_package(&corrupt, DecodeLimits::default())
            .is_err()
    );
    assert_eq!(session.project().document_hash(), live_hash);

    assert!(
        SimulatorExtension::new("vendor.payload", 1, PayloadValue::Record(BTreeMap::new()))
            .is_err()
    );
    assert!(
        SimulatorExtension::new("edu.valid", 2, PayloadValue::Record(BTreeMap::new())).is_err()
    );
    assert!(SimulatorExtension::new("edu.valid", 1, PayloadValue::from("opaque bytes")).is_err());
}
