use windows_project_broker::ProjectFileName;

#[path = "../../../tests/support/isolation_fuzz.rs"]
mod isolation_fuzz;

#[test]
fn complete_endpoint_corpus_fails_at_the_native_file_name_boundary() {
    for case in isolation_fuzz::cases() {
        assert!(
            ProjectFileName::parse(&case.value).is_err(),
            "{} escaped the native metadata boundary",
            case.id,
        );
    }
}
