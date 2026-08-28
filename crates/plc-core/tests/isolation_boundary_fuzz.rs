use std::collections::BTreeMap;

use plc_core::{
    DecodeLimits, Engine, NativeImportPreview, ObjectId, PayloadValue, ProfilePin, Project,
    Sha256Digest, SimulatorExtension, Uuid, decode_project_package, encode_project_package,
    preview_native_import,
};

#[path = "../../../tests/support/isolation_fuzz.rs"]
mod isolation_fuzz;

fn project_with_note(note: &str) -> Project {
    Project::new_with_simulator_extensions(
        Uuid::deterministic_v4(b"isolation-fuzz-document", 1),
        ObjectId(Uuid::deterministic_v4(b"isolation-fuzz-root", 1)),
        "Isolation boundary corpus",
        ProfilePin {
            id: "EDU-21 Core".to_owned(),
            version: "1.0.0".to_owned(),
            manifest_hash: Sha256Digest([0x21; 32]),
        },
        vec![
            SimulatorExtension::new(
                "edu.lesson-notes",
                1,
                PayloadValue::Record(BTreeMap::from([(
                    "text".to_owned(),
                    PayloadValue::from(note),
                )])),
            )
            .expect("inert simulator note"),
        ],
    )
    .expect("project with inert note")
}

#[test]
fn complete_utf8_endpoint_corpus_round_trips_as_inert_saved_project_data() {
    let cases = isolation_fuzz::cases();
    let utf8_cases = cases
        .iter()
        .filter(|case| case.id != "lone-surrogate")
        .collect::<Vec<_>>();
    assert_eq!(utf8_cases.len(), 26);
    for case in utf8_cases {
        let project = project_with_note(&case.value);
        let encoded = encode_project_package(&project, "phase2-isolation-fuzz/1")
            .unwrap_or_else(|error| panic!("{}: encode failed: {error}", case.id));
        let (decoded, manifest) = decode_project_package(&encoded, DecodeLimits::default())
            .unwrap_or_else(|error| panic!("{}: decode failed: {error}", case.id));
        assert_eq!(
            decoded.document_hash(),
            project.document_hash(),
            "{}",
            case.id
        );
        assert_eq!(
            decoded.saved_document_hash(),
            Some(manifest.package_hash),
            "{}",
            case.id
        );
        assert_eq!(
            decoded
                .simulator_extensions()
                .next()
                .expect("preserved note")
                .data(),
            &PayloadValue::Record(BTreeMap::from([(
                "text".to_owned(),
                PayloadValue::from(case.value.as_str()),
            )])),
            "{}",
            case.id
        );
        assert!(matches!(
            preview_native_import(case.value.as_bytes()),
            NativeImportPreview::Unsupported { .. }
        ));
    }
}

#[test]
fn invalid_utf8_saved_project_and_vendor_inputs_fail_closed_or_remain_unsupported() {
    let invalid_utf8_surrogate = b"\xed\xa0\x80";
    assert!(decode_project_package(invalid_utf8_surrogate, DecodeLimits::default()).is_err());
    assert!(matches!(
        preview_native_import(invalid_utf8_surrogate),
        NativeImportPreview::Unsupported { .. }
    ));
}

#[test]
fn project_display_names_route_the_complete_corpus_as_data_or_fail_validation() {
    for (ordinal, case) in isolation_fuzz::cases().iter().enumerate() {
        let root_id = ObjectId(Uuid::deterministic_v4(
            b"isolation-fuzz-display-name",
            ordinal as u64,
        ));
        let project = Project::new(
            Uuid::deterministic_v4(b"isolation-fuzz-display-document", ordinal as u64),
            root_id,
            case.value.clone(),
            ProfilePin {
                id: "EDU-21 Core".to_owned(),
                version: "1.0.0".to_owned(),
                manifest_hash: Sha256Digest([0x21; 32]),
            },
        );
        let invalid = case.value.is_empty()
            || case.value.len() > 256
            || case.value.chars().any(char::is_control);
        match Engine::new(project) {
            Ok(engine) => {
                assert!(!invalid, "{} escaped display-name validation", case.id);
                assert_eq!(
                    engine
                        .project()
                        .object(root_id)
                        .expect("project root")
                        .display_name,
                    case.value,
                    "{}",
                    case.id,
                );
            }
            Err(_) => assert!(invalid, "{} was rejected unexpectedly", case.id),
        }
    }
}
