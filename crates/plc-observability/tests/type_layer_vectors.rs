use plc_observability::{
    CanonicalLayerBundle, CanonicalLayerSnapshot, CanonicalValue as RuntimeValue,
    EngineeringValueLayers, ForceProvenance, LayerCodecLimits, LayerError, LayerForce,
    LayerTargetKind, PublishedTargetValue, Quality, RuntimeValueLayers, SampleFreshness,
    ScalarEngineeringValueLayers, ScalarRuntimeValueLayers, StableTargetId, ValueType,
    scalar_layer_snapshot_from_publication,
};
use plc_types::{
    ArrayBound, CanonicalF32, CanonicalType, PlcValue, PrimitiveType, ScalarValue, StableUuid,
    StructFieldValue, StructMember, TypeError, TypedScalar,
};

fn id(discriminator: u8) -> StableUuid {
    let mut bytes = [0_u8; 16];
    bytes[6] = 0x40;
    bytes[8] = 0x80;
    bytes[15] = discriminator;
    StableUuid::from_bytes(bytes).unwrap()
}

fn scalar(data_type: PrimitiveType, value: ScalarValue) -> PlcValue {
    PlcValue::Scalar(TypedScalar::new(data_type, value).unwrap())
}

fn dint(value: i32) -> PlcValue {
    scalar(PrimitiveType::Dint, ScalarValue::Signed(i64::from(value)))
}

fn memory_record() -> CanonicalLayerSnapshot {
    CanonicalLayerSnapshot {
        target_id: StableTargetId(1),
        target_kind: LayerTargetKind::Memory,
        data_type: CanonicalType::Primitive(PrimitiveType::Dint),
        engineering: EngineeringValueLayers {
            declared_default: Some(dint(1)),
            declared_start: Some(dint(2)),
            current_offline: Some(dint(3)),
            constant: Some(dint(4)),
            loaded_start: Some(dint(5)),
            working: Some(dint(6)),
        },
        runtime: RuntimeValueLayers {
            actual: Some(dint(7)),
            retained: Some(dint(8)),
            snapshot: Some(dint(9)),
            raw_input: None,
            natural: Some(dint(10)),
            effective: Some(dint(11)),
            committed_output: None,
            delivered_output: None,
        },
        quality: Quality::Good,
        freshness: SampleFreshness::Current,
        force: Some(LayerForce {
            value: dint(11),
            provenance: ForceProvenance {
                force_id: 0xfeed,
                registry_version: 7,
            },
        }),
    }
}

fn input_record() -> CanonicalLayerSnapshot {
    CanonicalLayerSnapshot {
        target_id: StableTargetId(2),
        target_kind: LayerTargetKind::Input,
        data_type: CanonicalType::Primitive(PrimitiveType::Dint),
        engineering: EngineeringValueLayers::default(),
        runtime: RuntimeValueLayers {
            raw_input: Some(dint(12)),
            natural: Some(dint(13)),
            effective: Some(dint(14)),
            ..RuntimeValueLayers::default()
        },
        quality: Quality::Uncertain,
        freshness: SampleFreshness::Stale,
        force: None,
    }
}

fn output_record() -> CanonicalLayerSnapshot {
    CanonicalLayerSnapshot {
        target_id: StableTargetId(3),
        target_kind: LayerTargetKind::Output,
        data_type: CanonicalType::Primitive(PrimitiveType::Dint),
        engineering: EngineeringValueLayers::default(),
        runtime: RuntimeValueLayers {
            natural: Some(dint(15)),
            effective: Some(dint(16)),
            committed_output: Some(dint(17)),
            delivered_output: Some(dint(18)),
            ..RuntimeValueLayers::default()
        },
        quality: Quality::Bad,
        freshness: SampleFreshness::Unknown,
        force: None,
    }
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(char::from(DIGITS[usize::from(*byte >> 4)]));
        encoded.push(char::from(DIGITS[usize::from(*byte & 0x0f)]));
    }
    encoded
}

#[test]
fn one_golden_bundle_keeps_every_engineering_runtime_io_and_metadata_layer_distinct() {
    let limits = LayerCodecLimits::edu21();
    let bundle = CanonicalLayerBundle::new(
        vec![output_record(), input_record(), memory_record()],
        limits,
    )
    .unwrap();
    let bytes = bundle.canonical_bytes(limits).unwrap();
    assert_eq!(
        hex(&bytes),
        concat!(
            "5045532d4f42532d42554e444c452d310001000000030000029b5045532d4f42532d4c41594552532d31000100000000",
            "00000000000000000000000101000000125045532d5459502d545950452d310000010401010000002a5045532d545950",
            "2d56414c55452d3100000000125045532d5459502d545950452d31000001040000000102010000002a5045532d545950",
            "2d56414c55452d3100000000125045532d5459502d545950452d31000001040000000203010000002a5045532d545950",
            "2d56414c55452d3100000000125045532d5459502d545950452d31000001040000000304010000002a5045532d545950",
            "2d56414c55452d3100000000125045532d5459502d545950452d31000001040000000405010000002a5045532d545950",
            "2d56414c55452d3100000000125045532d5459502d545950452d31000001040000000506010000002a5045532d545950",
            "2d56414c55452d3100000000125045532d5459502d545950452d31000001040000000607010000002a5045532d545950",
            "2d56414c55452d3100000000125045532d5459502d545950452d31000001040000000708010000002a5045532d545950",
            "2d56414c55452d3100000000125045532d5459502d545950452d31000001040000000809010000002a5045532d545950",
            "2d56414c55452d3100000000125045532d5459502d545950452d3100000104000000090a000b010000002a5045532d54",
            "59502d56414c55452d3100000000125045532d5459502d545950452d31000001040000000a0c010000002a5045532d54",
            "59502d56414c55452d3100000000125045532d5459502d545950452d31000001040000000b0d000e000f011001110100",
            "00000000000000000000000000feed00000000000000070000002a5045532d5459502d56414c55452d31000000001250",
            "45532d5459502d545950452d31000001040000000b000000e55045532d4f42532d4c41594552532d3100010000000000",
            "000000000000000000000202000000125045532d5459502d545950452d31000001040100020003000400050006000700",
            "080009000a010000002a5045532d5459502d56414c55452d3100000000125045532d5459502d545950452d3100000104",
            "0000000c0b010000002a5045532d5459502d56414c55452d3100000000125045532d5459502d545950452d3100000104",
            "0000000d0c010000002a5045532d5459502d56414c55452d3100000000125045532d5459502d545950452d3100000104",
            "0000000e0d000e000f0210021100000001135045532d4f42532d4c41594552532d310001000000000000000000000000",
            "0000000303000000125045532d5459502d545950452d31000001040100020003000400050006000700080009000a000b",
            "010000002a5045532d5459502d56414c55452d3100000000125045532d5459502d545950452d31000001040000000f0c",
            "010000002a5045532d5459502d56414c55452d3100000000125045532d5459502d545950452d3100000104000000100d",
            "010000002a5045532d5459502d56414c55452d3100000000125045532d5459502d545950452d3100000104000000110e",
            "010000002a5045532d5459502d56414c55452d3100000000125045532d5459502d545950452d3100000104000000120f",
            "0310031100",
        )
    );
    assert_eq!(
        bundle
            .records()
            .map(|record| record.target_id)
            .collect::<Vec<_>>(),
        vec![StableTargetId(1), StableTargetId(2), StableTargetId(3)]
    );

    let base = memory_record();
    let base_bytes = base.canonical_bytes(limits).unwrap();
    assert!(base.verify_canonical_bytes(&base_bytes, limits).unwrap());
    let variants = [
        {
            let mut value = base.clone();
            value.engineering.declared_default = Some(dint(99));
            value
        },
        {
            let mut value = base.clone();
            value.engineering.declared_start = Some(dint(99));
            value
        },
        {
            let mut value = base.clone();
            value.engineering.current_offline = Some(dint(99));
            value
        },
        {
            let mut value = base.clone();
            value.engineering.constant = Some(dint(99));
            value
        },
        {
            let mut value = base.clone();
            value.engineering.loaded_start = Some(dint(99));
            value
        },
        {
            let mut value = base.clone();
            value.engineering.working = Some(dint(99));
            value
        },
        {
            let mut value = base.clone();
            value.runtime.actual = Some(dint(99));
            value
        },
        {
            let mut value = base.clone();
            value.runtime.retained = Some(dint(99));
            value
        },
        {
            let mut value = base.clone();
            value.runtime.snapshot = Some(dint(99));
            value
        },
        {
            let mut value = base.clone();
            value.runtime.natural = Some(dint(99));
            value
        },
        {
            let mut value = base.clone();
            value.runtime.effective = Some(dint(99));
            value.force.as_mut().unwrap().value = dint(99);
            value
        },
        {
            let mut value = base.clone();
            value.quality = Quality::Bad;
            value
        },
        {
            let mut value = base.clone();
            value.freshness = SampleFreshness::Stale;
            value
        },
        {
            let mut value = base.clone();
            value.force.as_mut().unwrap().provenance.registry_version = 8;
            value
        },
    ];
    for variant in variants {
        assert_ne!(variant.canonical_bytes(limits).unwrap(), base_bytes);
    }
    assert_ne!(input_record().canonical_bytes(limits).unwrap(), base_bytes);
    assert_ne!(output_record().canonical_bytes(limits).unwrap(), base_bytes);
}

#[test]
fn aggregate_layers_use_the_same_bounded_type_value_codec() {
    let limits = LayerCodecLimits::edu21();
    let member = StructMember {
        id: id(10),
        name: "Samples".to_owned(),
        declared_order: 0,
        data_type: CanonicalType::Array {
            dimensions: vec![ArrayBound {
                lower: -1,
                upper: 0,
            }],
            element_type: Box::new(CanonicalType::Primitive(PrimitiveType::Real)),
        },
        reusable_default: None,
        comment: String::new(),
    };
    let data_type = CanonicalType::NamedStruct {
        id: id(9),
        members: vec![member],
    };
    let value = PlcValue::Struct(vec![StructFieldValue {
        member_id: id(10),
        value: PlcValue::Array(vec![
            scalar(
                PrimitiveType::Real,
                ScalarValue::Real(CanonicalF32::new(-0.0)),
            ),
            scalar(
                PrimitiveType::Real,
                ScalarValue::Real(CanonicalF32::new(f32::NAN)),
            ),
        ]),
    }]);
    let snapshot = CanonicalLayerSnapshot {
        target_id: StableTargetId(50),
        target_kind: LayerTargetKind::Memory,
        data_type,
        engineering: EngineeringValueLayers {
            declared_start: Some(value.clone()),
            loaded_start: Some(value.clone()),
            ..EngineeringValueLayers::default()
        },
        runtime: RuntimeValueLayers {
            actual: Some(value.clone()),
            retained: Some(value.clone()),
            snapshot: Some(value),
            ..RuntimeValueLayers::default()
        },
        quality: Quality::Good,
        freshness: SampleFreshness::Current,
        force: None,
    };
    let first = snapshot.canonical_bytes(limits).unwrap();
    let second = snapshot.canonical_bytes(limits).unwrap();
    assert_eq!(first, second);

    let constrained = LayerCodecLimits {
        max_record_bytes: first.len() - 1,
        max_bundle_bytes: first.len(),
        ..limits
    };
    assert_eq!(
        snapshot.canonical_bytes(constrained),
        Err(LayerError::CapacityExceeded)
    );
}

#[test]
fn invalid_target_type_force_and_bundle_shapes_are_rejected_without_bytes() {
    let limits = LayerCodecLimits::edu21();
    let mut memory = memory_record();
    memory.runtime.raw_input = Some(dint(1));
    assert_eq!(
        memory.canonical_bytes(limits),
        Err(LayerError::LayerUnavailableForTarget)
    );

    let mut force_mismatch = memory_record();
    force_mismatch.force.as_mut().unwrap().value = dint(12);
    assert_eq!(
        force_mismatch.canonical_bytes(limits),
        Err(LayerError::ForceEffectiveMismatch)
    );

    let mut type_mismatch = memory_record();
    type_mismatch.runtime.actual = Some(scalar(PrimitiveType::Bool, ScalarValue::Bool(true)));
    assert_eq!(
        type_mismatch.canonical_bytes(limits),
        Err(LayerError::Type(TypeError::ValueTypeMismatch))
    );

    assert_eq!(
        CanonicalLayerBundle::new(vec![memory_record(), memory_record()], limits),
        Err(LayerError::DuplicateTarget(StableTargetId(1)))
    );
    assert_eq!(
        CanonicalLayerBundle::new(Vec::new(), limits),
        Err(LayerError::NoRecords)
    );

    let bundle = CanonicalLayerBundle::new(
        vec![memory_record(), input_record(), output_record()],
        limits,
    )
    .unwrap();
    assert_eq!(
        bundle.canonical_bytes(LayerCodecLimits {
            max_records: 2,
            ..limits
        }),
        Err(LayerError::RecordLimit)
    );
    assert_eq!(
        memory_record().canonical_bytes(LayerCodecLimits {
            max_records: 4_097,
            ..limits
        }),
        Err(LayerError::InvalidLimits)
    );
}

#[test]
fn production_scalar_publication_adapter_preserves_float_bits_and_metadata() {
    let limits = LayerCodecLimits::edu21();
    let negative_zero = RuntimeValue::F32(CanonicalF32::new(-0.0));
    let positive_zero = RuntimeValue::F32(CanonicalF32::new(0.0));
    let publication = PublishedTargetValue {
        target_id: StableTargetId(70),
        value_type: ValueType::F32,
        natural_value: negative_zero,
        effective_value: positive_zero,
        raw_input_value: None,
        committed_output_value: Some(negative_zero),
        delivered_output_value: Some(positive_zero),
        quality: Quality::Uncertain,
        force: None,
    };
    let snapshot = scalar_layer_snapshot_from_publication(
        StableTargetId(70),
        LayerTargetKind::Output,
        ScalarEngineeringValueLayers {
            declared_start: Some(RuntimeValue::F32(CanonicalF32::new(f32::NAN))),
            loaded_start: Some(negative_zero),
            ..ScalarEngineeringValueLayers::default()
        },
        ScalarRuntimeValueLayers {
            actual: Some(positive_zero),
            retained: Some(negative_zero),
            snapshot: Some(positive_zero),
        },
        publication,
        SampleFreshness::Stale,
        limits,
    )
    .unwrap();

    assert_eq!(snapshot.quality, Quality::Uncertain);
    assert_eq!(snapshot.freshness, SampleFreshness::Stale);
    assert_ne!(snapshot.runtime.natural, snapshot.runtime.effective);
    assert!(snapshot.canonical_bytes(limits).is_ok());

    let mismatched = PublishedTargetValue {
        natural_value: RuntimeValue::Bool(true),
        ..publication
    };
    assert_eq!(
        scalar_layer_snapshot_from_publication(
            StableTargetId(70),
            LayerTargetKind::Output,
            ScalarEngineeringValueLayers::default(),
            ScalarRuntimeValueLayers::default(),
            mismatched,
            SampleFreshness::Current,
            limits,
        ),
        Err(LayerError::RuntimeTypeMismatch)
    );

    assert_eq!(
        scalar_layer_snapshot_from_publication(
            StableTargetId(71),
            LayerTargetKind::Output,
            ScalarEngineeringValueLayers::default(),
            ScalarRuntimeValueLayers::default(),
            publication,
            SampleFreshness::Current,
            limits,
        ),
        Err(LayerError::RuntimeTargetMismatch)
    );
}
