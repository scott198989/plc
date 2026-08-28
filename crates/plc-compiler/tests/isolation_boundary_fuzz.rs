use plc_compiler::{ResourceLimits, SclSource, scl::parse_scl};
use plc_program::BlockId;

#[path = "../../../tests/support/isolation_fuzz.rs"]
mod isolation_fuzz;

#[test]
fn scl_source_text_routes_the_complete_corpus_deterministically() {
    for (ordinal, case) in isolation_fuzz::cases().iter().enumerate() {
        let owner = BlockId::new(10_000 + ordinal as u128);
        let source = SclSource::new(owner, case.value.clone());
        let repeated = SclSource::new(owner, case.value.clone());

        assert_eq!(source.text(), case.value, "{}", case.id);
        assert_eq!(
            source.revision_hash(),
            repeated.revision_hash(),
            "{}",
            case.id
        );
        assert_eq!(
            parse_scl(&source, ResourceLimits::default()),
            parse_scl(&repeated, ResourceLimits::default()),
            "{}",
            case.id,
        );
    }
}
