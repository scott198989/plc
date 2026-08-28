use std::fmt::Write;

use plc_core::Project;
use plc_system::{ProjectDiagnosticPhase, project_hardware};

pub(crate) fn project_system_query(project: &Project) -> Vec<u8> {
    let projection = project_hardware(project);
    let artifact = projection.artifact();
    let mut output = String::with_capacity(512 + projection.diagnostics().len() * 192);
    output.push_str(r#"{"allocationChangeCount":"#);
    write!(
        output,
        "{}",
        projection
            .allocation_preview()
            .map_or(0, |preview| preview.changes.len())
    )
    .expect("write to String");
    output.push_str(r#","artifactFingerprint":"#);
    if let Some(artifact) = artifact {
        push_json_string(&mut output, &artifact.hardware_fingerprint.to_hex());
    } else {
        output.push_str("null");
    }
    output.push_str(r#","canBuild":"#);
    output.push_str(if projection.can_build() {
        "true"
    } else {
        "false"
    });
    output.push_str(r#","channelBindingCount":"#);
    write!(
        output,
        "{}",
        artifact.map_or(0, |artifact| artifact.channel_bindings.len())
    )
    .expect("write to String");
    output.push_str(r#","diagnostics":["#);
    for (index, diagnostic) in projection.diagnostics().iter().enumerate() {
        if index > 0 {
            output.push(',');
        }
        output.push_str(r#"{"blocking":"#);
        output.push_str(if diagnostic.blocking { "true" } else { "false" });
        output.push_str(r#","code":"#);
        push_json_string(&mut output, &diagnostic.code);
        output.push_str(r#","message":"#);
        push_json_string(&mut output, &diagnostic.message);
        output.push_str(r#","phase":"#);
        push_json_string(
            &mut output,
            match diagnostic.phase {
                ProjectDiagnosticPhase::CanonicalProjection => "canonical-projection",
                ProjectDiagnosticPhase::Hardware => "hardware",
            },
        );
        output.push_str(r#","primaryObjectId":"#);
        push_json_string(&mut output, &diagnostic.primary_object_id.to_string());
        output.push_str(r#","relatedObjectIds":["#);
        for (related_index, related) in diagnostic.related_object_ids.iter().enumerate() {
            if related_index > 0 {
                output.push(',');
            }
            push_json_string(&mut output, &related.to_string());
        }
        output.push_str("]}");
    }
    output.push_str(r#"],"profile":{"id":"#);
    push_json_string(&mut output, projection.profile().id());
    output.push_str(r#","manifestHash":"#);
    push_json_string(&mut output, &projection.profile().manifest_hash().to_hex());
    output.push_str(r#","version":"#);
    push_json_string(&mut output, projection.profile().version());
    output.push_str(r#"},"schemaVersion":1,"sourceDocumentHash":"#);
    push_json_string(&mut output, &projection.source_document_hash().to_hex());
    output.push_str(r#","sourceSemanticFingerprint":"#);
    push_json_string(
        &mut output,
        &projection.source_semantic_fingerprint().to_hex(),
    );
    output.push('}');
    output.into_bytes()
}

fn push_json_string(output: &mut String, value: &str) {
    output.push('"');
    for character in value.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\u{08}' => output.push_str("\\b"),
            '\u{0c}' => output.push_str("\\f"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            character if character <= '\u{1f}' => {
                write!(output, "\\u{:04x}", u32::from(character)).expect("write to String");
            }
            character => output.push(character),
        }
    }
    output.push('"');
}

#[cfg(test)]
mod tests {
    use super::push_json_string;

    #[test]
    fn json_string_encoding_is_complete_and_deterministic() {
        let mut output = String::new();
        push_json_string(
            &mut output,
            "quote \" slash \\ line\n tab\t null\0 degree °",
        );
        assert_eq!(
            output,
            "\"quote \\\" slash \\\\ line\\n tab\\t null\\u0000 degree °\""
        );
    }
}
