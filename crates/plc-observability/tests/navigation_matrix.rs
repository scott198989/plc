use plc_observability::{
    ArtifactSide, LoadedArtifactBinding, NavigationAnchor, NavigationDomainProjection,
    NavigationError, NavigationIndexBuilder, NavigationKind, NavigationRelationshipKind,
    NavigationValidity, SemanticIdentity, SourceAnchor, StableTargetId,
};
use plc_runtime::{Hash32, Sha256};

fn hash(label: &str) -> Hash32 {
    Sha256::digest(label.as_bytes())
}

fn source(artifact_fingerprint: Hash32, semantic_identity: u128, start_utf16: u32) -> SourceAnchor {
    SourceAnchor {
        artifact_fingerprint,
        file_identity: 70,
        semantic_identity,
        start_utf16,
        end_utf16: start_utf16 + 3,
    }
}

fn source_anchor(
    identity: u128,
    side: ArtifactSide,
    artifact_fingerprint: Hash32,
    controller_epoch: Option<u64>,
    start_utf16: u32,
    validity: NavigationValidity,
) -> NavigationAnchor {
    NavigationAnchor {
        identity: SemanticIdentity(identity),
        kind: NavigationKind::SourceSpan,
        side,
        artifact_fingerprint,
        controller_epoch,
        source: Some(source(artifact_fingerprint, identity, start_utf16)),
        domain_projection: Some(NavigationDomainProjection::ProgramMember {
            owner_identity: 10,
            member_identity: identity,
        }),
        probe_target: None,
        relationship_kind: NavigationRelationshipKind::Selected,
        validity,
        tombstone_reason_hash: None,
    }
}

#[test]
fn every_semantic_relationship_is_identity_based_and_role_preserving() {
    let offline = hash("offline-navigation-artifact");
    let loaded = hash("loaded-navigation-artifact");
    let loaded_binding = LoadedArtifactBinding {
        fingerprint: loaded,
        controller_epoch: 41,
    };
    let mut builder = NavigationIndexBuilder::new(1, offline, Some(loaded_binding)).unwrap();
    builder
        .insert_anchor(source_anchor(
            1,
            ArtifactSide::CurrentOffline,
            offline,
            None,
            100,
            NavigationValidity::Valid,
        ))
        .unwrap();
    builder
        .insert_anchor(source_anchor(
            1,
            ArtifactSide::Loaded,
            loaded,
            Some(41),
            7,
            NavigationValidity::Valid,
        ))
        .unwrap();

    let relationships = [
        NavigationRelationshipKind::Definition,
        NavigationRelationshipKind::Use,
        NavigationRelationshipKind::Call,
        NavigationRelationshipKind::Assignment,
        NavigationRelationshipKind::AddressOverlap,
        NavigationRelationshipKind::TypeDependency,
        NavigationRelationshipKind::HardwareBinding,
        NavigationRelationshipKind::ProbeReference,
        NavigationRelationshipKind::ForceReference,
        NavigationRelationshipKind::TraceReference,
    ];
    for (offset, relationship) in relationships.into_iter().enumerate() {
        let identity = u128::try_from(offset).unwrap() + 2;
        let mut anchor = source_anchor(
            identity,
            ArtifactSide::CurrentOffline,
            offline,
            None,
            u32::try_from(identity * 10).unwrap(),
            NavigationValidity::Valid,
        );
        if relationship == NavigationRelationshipKind::HardwareBinding {
            anchor.kind = NavigationKind::HardwareObject;
            anchor.domain_projection = Some(NavigationDomainProjection::HardwareObject {
                object_identity: identity,
            });
        } else if relationship == NavigationRelationshipKind::ProbeReference {
            anchor.kind = NavigationKind::ProbeTarget;
            anchor.probe_target = Some(StableTargetId(identity));
            anchor.domain_projection = Some(NavigationDomainProjection::ProbeTarget {
                target: StableTargetId(identity),
            });
        }
        builder.insert_anchor(anchor).unwrap();
        builder
            .relate_kind(
                SemanticIdentity(1),
                SemanticIdentity(identity),
                relationship,
            )
            .unwrap();
    }

    let tombstone = SemanticIdentity(99);
    builder
        .insert_anchor(NavigationAnchor {
            identity: tombstone,
            kind: NavigationKind::Tombstone,
            side: ArtifactSide::CurrentOffline,
            artifact_fingerprint: offline,
            controller_epoch: None,
            source: None,
            domain_projection: None,
            probe_target: None,
            relationship_kind: NavigationRelationshipKind::Selected,
            validity: NavigationValidity::TargetRemoved,
            tombstone_reason_hash: Some(hash("TARGET_REMOVED")),
        })
        .unwrap();
    builder
        .route_diagnostic_with_roles(
            500,
            SemanticIdentity(1),
            vec![
                (
                    NavigationRelationshipKind::AddressOverlap,
                    SemanticIdentity(6),
                ),
                (
                    NavigationRelationshipKind::HardwareBinding,
                    SemanticIdentity(8),
                ),
                (NavigationRelationshipKind::DiagnosticReference, tombstone),
            ],
        )
        .unwrap();

    let index = builder.commit().unwrap();
    let before = (index.revision(), index.index_hash());
    let offline_result = index
        .resolve(SemanticIdentity(1), ArtifactSide::CurrentOffline)
        .unwrap();
    assert_eq!(offline_result.primary.validity, NavigationValidity::Valid);
    assert_eq!(offline_result.primary.source.unwrap().start_utf16, 100);
    let observed_relationships = offline_result
        .related
        .iter()
        .map(|target| target.relationship_kind)
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(observed_relationships, relationships.into_iter().collect());

    let loaded_result = index
        .resolve(SemanticIdentity(1), ArtifactSide::Loaded)
        .unwrap();
    assert_eq!(
        loaded_result.primary.validity,
        NavigationValidity::StaleArtifact
    );
    assert_eq!(loaded_result.primary.controller_epoch, Some(41));
    assert_eq!(loaded_result.primary.source.unwrap().start_utf16, 7);

    let diagnostic = index
        .resolve_diagnostic(500, ArtifactSide::CurrentOffline)
        .unwrap();
    assert_eq!(
        diagnostic.primary.relationship_kind,
        NavigationRelationshipKind::DiagnosticPrimary
    );
    for expected in [
        NavigationRelationshipKind::AddressOverlap,
        NavigationRelationshipKind::HardwareBinding,
        NavigationRelationshipKind::DiagnosticReference,
    ] {
        assert!(
            diagnostic
                .related
                .iter()
                .any(|target| target.relationship_kind == expected)
        );
    }
    assert!(diagnostic.related.iter().any(|target| {
        target.identity == tombstone && target.validity == NavigationValidity::TargetRemoved
    }));
    assert_eq!((index.revision(), index.index_hash()), before);
}

#[test]
fn invalid_editable_relations_remain_explicit_and_updates_fail_closed() {
    let offline = hash("editable-offline");
    let loaded = hash("editable-loaded");
    let binding = LoadedArtifactBinding {
        fingerprint: loaded,
        controller_epoch: 2,
    };
    let mut builder = NavigationIndexBuilder::new(3, offline, Some(binding)).unwrap();
    for (identity, validity) in [
        (1, NavigationValidity::Valid),
        (2, NavigationValidity::Unresolved),
        (3, NavigationValidity::Ambiguous),
    ] {
        builder
            .insert_anchor(source_anchor(
                identity,
                ArtifactSide::CurrentOffline,
                offline,
                None,
                u32::try_from(identity * 4).unwrap(),
                validity,
            ))
            .unwrap();
    }
    builder
        .relate_kind(
            SemanticIdentity(1),
            SemanticIdentity(2),
            NavigationRelationshipKind::Use,
        )
        .unwrap();
    builder
        .relate_kind(
            SemanticIdentity(1),
            SemanticIdentity(3),
            NavigationRelationshipKind::Use,
        )
        .unwrap();
    let index = builder.commit().unwrap();
    let result = index
        .resolve(SemanticIdentity(1), ArtifactSide::CurrentOffline)
        .unwrap();
    assert!(
        result
            .related
            .iter()
            .any(|target| target.validity == NavigationValidity::Unresolved)
    );
    assert!(
        result
            .related
            .iter()
            .any(|target| target.validity == NavigationValidity::Ambiguous)
    );
    assert!(matches!(
        index.begin_update(3),
        Err(NavigationError::RevisionNotMonotonic)
    ));

    let mut wrong_epoch = NavigationIndexBuilder::new(4, offline, Some(binding)).unwrap();
    assert_eq!(
        wrong_epoch.insert_anchor(source_anchor(
            7,
            ArtifactSide::Loaded,
            loaded,
            Some(3),
            0,
            NavigationValidity::Valid,
        )),
        Err(NavigationError::AnchorArtifactMismatch)
    );

    let mut dangling = NavigationIndexBuilder::new(5, offline, None).unwrap();
    dangling
        .insert_anchor(source_anchor(
            1,
            ArtifactSide::CurrentOffline,
            offline,
            None,
            0,
            NavigationValidity::Valid,
        ))
        .unwrap();
    dangling
        .relate_kind(
            SemanticIdentity(1),
            SemanticIdentity(404),
            NavigationRelationshipKind::Call,
        )
        .unwrap();
    assert_eq!(
        dangling.commit(),
        Err(NavigationError::DanglingRelationship(SemanticIdentity(1)))
    );
}
