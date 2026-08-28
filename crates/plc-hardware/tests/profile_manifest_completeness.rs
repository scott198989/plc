#![allow(clippy::too_many_lines)]

use std::collections::BTreeSet;

use plc_hardware::{
    Capability, EDU21_REQUIRED_MANIFEST_FIELD_COUNT, ManifestScalar, TrainingProfile,
};

fn expected_manifest_paths() -> BTreeSet<String> {
    let mut paths = BTreeSet::from([
        "profile.id".to_owned(),
        "profile.version".to_owned(),
        "profile.catalogVersion".to_owned(),
    ]);
    for capability in Capability::ALL {
        paths.insert(format!("capability.{}", capability.key()));
    }
    for name in [
        "controllersPerProject",
        "projectObjects",
        "tagsPerController",
        "namedTypesPerProject",
        "sourceBytesPerBlock",
        "typeNesting",
        "membersPerType",
        "arrayDimensions",
        "arrayElements",
        "syntaxNesting",
        "networksPerBlock",
        "nodesPerNetwork",
        "edgesPerNetwork",
        "dependencyEdges",
        "diagnosticsPerBuild",
        "constantEvaluationOperations",
        "semanticWorkUnitsPerBuild",
        "artifactPackageBytes",
        "packageBytes",
        "packageMemberBytes",
        "expandedPackageBytes",
        "packageMembers",
        "packageNesting",
        "normalizedPathBytes",
        "stringFieldBytes",
        "expansionRatio",
        "watchTablesPerProject",
        "watchRowsPerTable",
        "activeSubscriptionsPerController",
        "retainedSamplesPerWatchRow",
        "traceConfigurationsPerProject",
        "traceChannelsPerCapture",
        "traceSamplesPerCapture",
        "concurrentTracesPerController",
        "traceDurationVirtualMs",
        "traceTriggerDepth",
        "traceTriggerNodes",
        "activeConditionsPerController",
        "ordinaryConditionsPerController",
        "retainedDiagnosticEvents",
        "snapshotBytes",
    ] {
        paths.insert(format!("limit.{name}"));
    }
    for name in [
        "scanQuantumMs",
        "cyclicMainCount",
        "startupCount",
        "timedCyclicCount",
        "workUnitsPerScan",
        "callDepth",
    ] {
        paths.insert(format!("scheduling.{name}"));
    }
    for name in [
        "unreachableSclIsBlocking",
        "multipleWriterIsBlocking",
        "unsafeTempIsBlocking",
        "missingConsumedReturnIsBlocking",
    ] {
        paths.insert(format!("diagnostic.{name}"));
    }
    for transition in [
        "stopToRun",
        "runToStop",
        "warmRestart",
        "simulatedPowerCycle",
        "memoryReset",
        "compatibleCodeLoad",
        "schemaChangingLoad",
    ] {
        for field in ["nonRetentive", "retentive", "io", "artifact", "forces"] {
            paths.insert(format!("restart.{transition}.{field}"));
        }
    }
    for controller in ["VCTRL-C1", "VCTRL-M1", "VCTRL-P1"] {
        for field in [
            "id",
            "displayName",
            "localFirstExpansionSlot",
            "localLastSlot",
            "controllerSlot",
            "distributedStations",
            "inputBytes",
            "outputBytes",
            "markerBytes",
            "dbDataBytes",
            "blockCapacity",
            "integratedInterfaces",
            "requiresPowerSlotZero",
        ] {
            paths.insert(format!("controller.{controller}.{field}"));
        }
    }
    for module in [
        "VPWR-1", "VSTN-H1", "VDI-16", "VDO-16", "VAI-4", "VAO-4", "VMIX-8", "VRTD-4", "VLINK-2",
    ] {
        for field in [
            "id",
            "displayName",
            "channels",
            "placement",
            "inputBytes",
            "outputBytes",
            "virtualPorts",
            "supportsWireBreak",
        ] {
            paths.insert(format!("module.{module}.{field}"));
        }
    }
    paths
}

fn assert_unsigned(
    fields: &std::collections::BTreeMap<String, ManifestScalar>,
    path: &str,
    expected: u64,
) {
    assert_eq!(
        fields.get(path),
        Some(&ManifestScalar::Unsigned(expected)),
        "{path}"
    );
}

#[test]
fn directive_required_manifest_fields_exist_exactly_once_and_are_fail_closed() {
    let profile = TrainingProfile::edu21();
    let fields = profile.manifest_fields().expect("validated inventory");
    let actual_paths: BTreeSet<_> = fields.keys().cloned().collect();
    let expected_paths = expected_manifest_paths();
    assert_eq!(
        expected_paths.len(),
        EDU21_REQUIRED_MANIFEST_FIELD_COUNT,
        "independent directive inventory count"
    );
    assert_eq!(actual_paths, expected_paths);
    assert_eq!(fields.len(), EDU21_REQUIRED_MANIFEST_FIELD_COUNT);
    assert!(profile.validate().is_ok());

    for capability in Capability::ALL {
        assert_eq!(
            fields.get(&format!("capability.{}", capability.key())),
            Some(&ManifestScalar::Bool(true))
        );
    }

    for (path, value) in [
        ("limit.controllersPerProject", 8),
        ("limit.projectObjects", 100_000),
        ("limit.tagsPerController", 32_768),
        ("limit.namedTypesPerProject", 2_048),
        ("limit.sourceBytesPerBlock", 1_048_576),
        ("limit.typeNesting", 32),
        ("limit.membersPerType", 4_096),
        ("limit.arrayDimensions", 6),
        ("limit.arrayElements", 1_000_000),
        ("limit.syntaxNesting", 256),
        ("limit.networksPerBlock", 10_000),
        ("limit.nodesPerNetwork", 10_000),
        ("limit.edgesPerNetwork", 20_000),
        ("limit.dependencyEdges", 1_000_000),
        ("limit.diagnosticsPerBuild", 10_000),
        ("limit.constantEvaluationOperations", 1_000_000),
        ("limit.semanticWorkUnitsPerBuild", 10_000_000),
        ("limit.artifactPackageBytes", 268_435_456),
        ("limit.packageBytes", 536_870_912),
        ("limit.packageMemberBytes", 268_435_456),
        ("limit.expandedPackageBytes", 1_073_741_824),
        ("limit.packageMembers", 100_000),
        ("limit.packageNesting", 32),
        ("limit.normalizedPathBytes", 512),
        ("limit.stringFieldBytes", 1_048_576),
        ("limit.expansionRatio", 100),
        ("limit.watchTablesPerProject", 64),
        ("limit.watchRowsPerTable", 512),
        ("limit.activeSubscriptionsPerController", 2_048),
        ("limit.retainedSamplesPerWatchRow", 1_024),
        ("limit.traceConfigurationsPerProject", 64),
        ("limit.traceChannelsPerCapture", 64),
        ("limit.traceSamplesPerCapture", 1_000_000),
        ("limit.concurrentTracesPerController", 4),
        ("limit.traceDurationVirtualMs", 86_400_000),
        ("limit.traceTriggerDepth", 32),
        ("limit.traceTriggerNodes", 256),
        ("limit.activeConditionsPerController", 10_000),
        ("limit.ordinaryConditionsPerController", 9_999),
        ("limit.retainedDiagnosticEvents", 100_000),
        ("limit.snapshotBytes", 268_435_456),
        ("scheduling.scanQuantumMs", 10),
        ("scheduling.cyclicMainCount", 1),
        ("scheduling.startupCount", 1),
        ("scheduling.timedCyclicCount", 8),
        ("scheduling.workUnitsPerScan", 100_000),
        ("scheduling.callDepth", 64),
    ] {
        assert_unsigned(&fields, path, value);
    }

    for path in fields.keys() {
        let normalized = path.to_ascii_lowercase();
        assert!(
            [
                "executable",
                "externalresource",
                "physicalendpoint",
                "transport",
                "hostcapability",
                "socket",
                "protocol",
            ]
            .iter()
            .all(|forbidden| !normalized.contains(forbidden)),
            "forbidden authority-shaped manifest field: {path}"
        );
    }
}
