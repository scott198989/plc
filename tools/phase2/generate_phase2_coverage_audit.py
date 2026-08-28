#!/usr/bin/env python3
"""Generate the honest Phase 2 static coverage audit.

This is a static implementation/test readiness audit, not executable evidence
and never a verification verdict.  The curated assessments below are reviewed
against the exact Appendix H minimum-proof text.  Generation fails closed when
the extracted inventories drift, a cited path disappears, or an assessment is
missing.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import sys
from collections import Counter, defaultdict
from pathlib import Path, PurePosixPath
from typing import Any, Iterable, Mapping, Sequence

import reviewed_requirement_mapping


READY = "IMPLEMENTED_EVIDENCE_READY"
PARTIAL = "PARTIAL"
MISSING = "MISSING"
ALLOWED_CLASSIFICATIONS = {READY, PARTIAL, MISSING}
EXPECTED_REQUIREMENTS = 937
EXPECTED_VERIFICATIONS = 44


def assessment(
    classification: str,
    support: str,
    implementation: Sequence[str],
    tests: Sequence[str],
    uncovered: Sequence[str] = (),
    lane: str | None = None,
) -> dict[str, Any]:
    return {
        "classification": classification,
        "support": support,
        "implementationPaths": list(implementation),
        "testPaths": list(tests),
        "uncoveredProofClauses": list(uncovered),
        "gapLaneId": lane,
    }


# Static assessments are intentionally conservative.  READY means that the
# complete Appendix H clause has a production path and a directly applicable
# executable test surface ready for current-candidate evidence collection.  It
# does not mean the proof has been run, bound, reviewed, or VERIFIED.
ASSESSMENTS: dict[str, dict[str, Any]] = {
    "VER-KRN-0001": assessment(
        READY,
        "The kernel executes preconditioned commands against a clone, commits atomically, emits domain events, maintains revisioned undo/redo history, and preserves stable object identities.",
        ["crates/plc-core/src/engine.rs", "crates/plc-core/src/model.rs", "crates/plc-core/src/protocol.rs"],
        ["crates/plc-core/src/engine.rs", "crates/plc-core/tests/journey_d.rs", "packages/plc-contract/test/contract.test.ts"],
    ),
    "VER-KRN-0002": assessment(
        READY,
        "Semantic and presentation revisions are separated, dependency invalidation is explicit, and the system session preserves current build/runtime state for presentation-only refreshes.",
        ["crates/plc-core/src/engine.rs", "crates/plc-program/src/invalidation.rs", "crates/plc-system/src/session.rs"],
        ["crates/plc-core/src/engine.rs", "crates/plc-program/tests/program_model.rs", "crates/plc-system/tests/canonical_hardware.rs", "crates/plc-system/tests/system_journeys.rs"],
    ),
    "VER-PST-0001": assessment(
        READY,
        "Canonical packages, document/package hashes, Save As identity, journal recovery, and transactional sequential migrations are covered by adversarial round-trip and checked-in migration-chain golden vectors.",
        ["crates/plc-core/src/package.rs", "crates/plc-core/src/journal.rs", "crates/plc-core/src/migration.rs", "apps/foundation-shell/src/file-access-broker.ts"],
        ["crates/plc-core/tests/journey_d.rs", "crates/plc-core/tests/persistence_adversarial.rs", "crates/plc-core/tests/goldens/migration_chain_1_to_3.txt", "apps/foundation-shell/test/file-access-broker.test.ts"],
    ),
    "VER-PST-0002": assessment(
        READY,
        "The bounded pure parser and adversarial corpus reject archive bombs, path confusion, unknown schemas, corruption, and downgrades without execution or partial mutation while preserving approved opaque extensions.",
        ["crates/plc-core/src/package.rs", "crates/plc-core/src/json.rs"],
        ["crates/plc-core/tests/persistence_adversarial.rs", "crates/plc-core/tests/journey_d.rs", "apps/foundation-shell/test/engineering-worker-handler.test.ts"],
    ),
    "VER-PROF-0001": assessment(
        READY,
        "The sole shipped EDU-21 profile is constructed in production, hashed and allowlisted, while system build projection rejects tampered, unknown, or project-supplied profile authority.",
        ["crates/plc-hardware/src/profile.rs", "crates/plc-system/src/hardware_projection.rs", "crates/plc-compiler/src/build.rs"],
        ["crates/plc-hardware/tests/edu21_contract.rs", "crates/plc-system/tests/system_journeys.rs", "crates/plc-compiler/tests/compiler_pipeline.rs"],
    ),
    "VER-PROF-0002": assessment(
        READY,
        "A machine-independent directive inventory proves every required EDU-21 capability, limit, scheduling, Modify/Force, trace, restart, diagnostic, and retention field is unique, bounded, fail-closed, and fingerprint-bound.",
        ["crates/plc-hardware/src/profile.rs", "crates/plc-hardware/src/hardware.rs", "crates/plc-compiler/src/build.rs"],
        ["crates/plc-hardware/tests/profile_manifest_completeness.rs", "crates/plc-hardware/tests/edu21_contract.rs", "crates/plc-compiler/tests/compiler_pipeline.rs", "crates/plc-system/tests/system_journeys.rs"],
    ),
    "VER-HWD-0001": assessment(
        READY,
        "EDU-21 device/module legality, capacities, maps, overlap rejection, every channel representation and byte order, missing-module quality, and ordinary-output suppression are covered by generated and table-driven vectors.",
        ["crates/plc-hardware/src/hardware.rs", "crates/plc-hardware/src/profile.rs", "crates/plc-hardware/src/canonical.rs", "crates/plc-hardware/src/condition.rs"],
        ["crates/plc-hardware/tests/edu21_contract.rs", "crates/plc-hardware/tests/hardware_condition_matrix.rs", "crates/plc-system/tests/canonical_hardware.rs"],
    ),
    "VER-HWD-0002": assessment(
        PARTIAL,
        "The complete physical-condition action matrix is causal and replayable, projects declared values/quality/suppression, crosses the runtime delivery boundary, and emits lifecycle diagnostics.",
        ["crates/plc-hardware/src/condition.rs", "crates/plc-runtime/src/controller.rs", "crates/plc-observability/src/hardware_diagnostics.rs", "crates/plc-system/src/session.rs"],
        ["crates/plc-hardware/tests/hardware_condition_matrix.rs", "crates/plc-runtime/tests/hardware_delivery_boundary.rs", "crates/plc-observability/tests/hardware_diagnostic_vectors.rs", "crates/plc-system/tests/system_journeys.rs"],
        [
            "The complete physical-condition matrix is not yet asserted through monitoring and trace while preserving the same causal event into the aggregate snapshot/replay boundary.",
        ],
        "LANE-PROFILE-HARDWARE-SYMBOLS",
    ),
    "VER-NET-0001": assessment(
        READY,
        "Virtual topology, subnet/name legality, deterministic discovery, reachability, and link faults are exercised inside a packaged adapters-off workflow with browser, worker, process, CDP, and NetLog host-network instrumentation.",
        ["crates/plc-hardware/src/network.rs", "tools/phase2/source_policy.py", "tools/phase2/run_isolation_counterfactual.mjs"],
        ["crates/plc-hardware/tests/edu21_contract.rs", "crates/plc-hardware/tests/hardware_condition_matrix.rs", "tests/phase2/isolation-counterfactual.unit.mjs"],
    ),
    "VER-SYM-0001": assessment(
        READY,
        "A generated symbol/address contract covers grammar and case rules, namespaces, scopes and shadowing, constants, identity-preserving rename/delete references, canonical I/Q/M forms, unsupported wide I/Q rejection, allocation, and overlaps.",
        ["crates/plc-hardware/src/symbols.rs", "crates/plc-hardware/src/ids.rs"],
        ["crates/plc-hardware/tests/symbol_address_matrix.rs", "crates/plc-language-tools/tests/scl_language_service.rs"],
    ),
    "VER-TYP-0001": assessment(
        READY,
        "The canonical scalar and recursive aggregate authorities cover every literal/conversion/arithmetic boundary named by the proof, including IEEE specials through compiler/runtime execution and exact array/structure assignment and serialization.",
        ["crates/plc-types/src/lib.rs", "crates/plc-types/src/aggregate.rs", "crates/plc-program/src/types.rs", "crates/plc-compiler/src/scl/semantics.rs"],
        ["crates/plc-types/tests/scalar_boundary_matrix.rs", "crates/plc-types/tests/aggregate_literal_matrix.rs", "crates/plc-program/tests/canonical_literal_matrix.rs", "crates/plc-compiler/tests/type_runtime_boundary.rs"],
    ),
    "VER-TYP-0002": assessment(
        READY,
        "One bounded canonical value-layer codec and checked-in golden distinguish declared/start/offline/loaded/working, actual/retained/snapshot, raw/natural/effective, committed/delivered, quality, freshness, and force provenance for scalar and aggregate values.",
        ["crates/plc-runtime/src/model.rs", "crates/plc-runtime/src/boundary.rs", "crates/plc-observability/src/layers.rs", "crates/plc-observability/src/target.rs"],
        ["crates/plc-observability/tests/type_layer_vectors.rs", "crates/plc-runtime/tests/observation_boundary.rs", "crates/plc-runtime/tests/runtime_vectors.rs"],
    ),
    "VER-PRG-0001": assessment(
        READY,
        "Canonical OB/FC/FB/DB interfaces, call binding, required actuals, copy-in/copy-out, FB instance and multi-instance paths, and retention-aware runtime frames are implemented and exercised.",
        ["crates/plc-program/src/model.rs", "crates/plc-program/src/validation.rs", "crates/plc-runtime/src/model.rs", "crates/plc-runtime/src/controller.rs"],
        ["crates/plc-program/tests/program_model.rs", "crates/plc-runtime/tests/invocation_calls.rs", "crates/plc-compiler/tests/runtime_vertical_slice.rs"],
    ),
    "VER-PRG-0002": assessment(
        READY,
        "Call/dependency graphs, canonical cycle rejection, state-instance ownership, call-depth/work budgets, and deterministic execution ordering have direct production and test coverage.",
        ["crates/plc-program/src/validation.rs", "crates/plc-program/src/invalidation.rs", "crates/plc-runtime/src/controller.rs"],
        ["crates/plc-program/tests/program_model.rs", "crates/plc-runtime/tests/invocation_calls.rs", "crates/plc-runtime/tests/runtime_vectors.rs"],
    ),
    "VER-LAD-0001": assessment(
        READY,
        "The coordinate-free LAD model validates and lowers power flow, branches, contacts, coils, boxes, calls, stateful instructions, semantic order, and writer conflicts into shared verified IR.",
        ["crates/plc-lad/src/model.rs", "crates/plc-lad/src/validate.rs", "crates/plc-lad/src/lowering.rs", "crates/plc-lad/src/edit.rs"],
        ["crates/plc-lad/tests/lad_contract.rs", "crates/plc-compiler/tests/mixed_language_runtime.rs"],
    ),
    "VER-FBD-0001": assessment(
        READY,
        "FBD typed ports, data/effect ordering, disabled outputs, calls, stateful instances, cycle rejection, editing, and shared-IR lowering have a bounded positive/negative contract suite.",
        ["crates/plc-language-tools/src/fbd.rs", "crates/plc-language-tools/src/fbd_editor.rs", "crates/plc-language-tools/src/fbd_diagnostic.rs", "crates/plc-language-tools/src/fbd_lowering.rs"],
        ["crates/plc-language-tools/tests/fbd_contract.rs", "crates/plc-compiler/tests/mixed_language_runtime.rs"],
    ),
    "VER-SCL-0001": assessment(
        READY,
        "The real lexer/parser preserves source and handles precedence, literals, statements, calls, loops, CASE, unsupported declarations, bounded recovery, and malformed structures in a golden corpus.",
        ["crates/plc-compiler/src/source.rs", "crates/plc-compiler/src/scl/lexer.rs", "crates/plc-compiler/src/scl/parser.rs", "crates/plc-compiler/src/scl/semantics.rs"],
        ["crates/plc-compiler/tests/scl_grammar_golden.rs", "crates/plc-compiler/tests/language_service_snapshot.rs"],
    ),
    "VER-SCL-0002": assessment(
        READY,
        "Expressions, assignment, CASE and every structured loop form, FC and FB calls, work-budget faults, and stable source occurrence mappings execute through verified control-flow graphs in the production runtime. The SCL FB vector binds an explicit instance identity, persists state across scans, snapshots it, restores it, and advances from the restored value.",
        ["crates/plc-compiler/src/lowering.rs", "crates/plc-compiler/src/runtime_adapter.rs", "crates/plc-runtime/src/controller.rs"],
        ["crates/plc-compiler/tests/scl_control_flow.rs", "crates/plc-compiler/tests/runtime_vertical_slice.rs", "crates/plc-compiler/tests/mixed_language_runtime.rs"],
    ),
    "VER-INS-0001": assessment(
        READY,
        "The generated registry matrix proves completeness, unique families/formals, every admitted type binding and disabled-output policy; LIMIT, FILL, and BLKMOVE lower into verified IR and execute with deterministic identities, state/fault policy, and atomic budget behavior.",
        ["crates/plc-program/src/instruction.rs", "crates/plc-compiler/src/ir.rs", "crates/plc-compiler/src/runtime_adapter.rs", "crates/plc-runtime/src/model.rs"],
        ["crates/plc-program/tests/instruction_registry_matrix.rs", "crates/plc-compiler/tests/limit_instruction_runtime.rs", "crates/plc-compiler/tests/aggregate_instruction_runtime.rs", "crates/plc-runtime/tests/aggregate_instructions.rs", "crates/plc-runtime/tests/invocation_calls.rs"],
    ),
    "VER-CMP-0001": assessment(
        READY,
        "The compiler captures bounded snapshots, performs real parse/bind/type/lower/verify/package stages, supports cancellation and caps, recovers diagnostics deterministically, and withholds partial artifacts.",
        ["crates/plc-compiler/src/build.rs", "crates/plc-compiler/src/diagnostic.rs", "crates/plc-compiler/src/lowering.rs", "crates/plc-compiler/src/composition.rs"],
        ["crates/plc-compiler/tests/compiler_pipeline.rs", "crates/plc-compiler/tests/frontend_composition.rs"],
    ),
    "VER-DEP-0001": assessment(
        READY,
        "A deterministic generated dependency graph and independent change oracle prove exact closure and narrow compile, reporting both under- and over-invalidation while preserving unresolved deleted uses.",
        ["crates/plc-program/src/dependency.rs", "crates/plc-program/src/invalidation.rs", "crates/plc-compiler/src/build.rs"],
        ["crates/plc-program/tests/dependency_invalidation_matrix.rs", "crates/plc-compiler/tests/build_mode_matrix.rs"],
    ),
    "VER-BLD-0001": assessment(
        READY,
        "Cold, warm, incremental, and Rebuild All execute as explicit modes; identical complete snapshots remain byte-identical across disposable cache state while nonsemantic source changes preserve semantic IR/fingerprints.",
        ["crates/plc-compiler/src/build.rs", "crates/plc-compiler/src/hash.rs", "crates/plc-system/src/build_product.rs", "crates/plc-system/src/session.rs"],
        ["crates/plc-compiler/tests/build_mode_matrix.rs", "crates/plc-compiler/tests/compiler_pipeline.rs", "crates/plc-system/tests/system_journeys.rs"],
    ),
    "VER-IR-0001": assessment(
        READY,
        "The independent typed-IR verifier validates schema, types, control flow, effects, source maps, probes, and ordering, and is re-run after mixed-frontend composition with tamper rejection tests.",
        ["crates/plc-compiler/src/ir.rs", "crates/plc-compiler/src/composition.rs"],
        ["crates/plc-compiler/tests/compiler_pipeline.rs", "crates/plc-compiler/tests/frontend_composition.rs", "crates/plc-lad/tests/lad_contract.rs", "crates/plc-language-tools/tests/fbd_contract.rs"],
    ),
    "VER-SMAP-0001": assessment(
        READY,
        "SCL definitions, diagnostics, probes, call effects, and runtime faults relocate by semantic identity across formatting-only edits. LAD and FBD graph anchors relocate by stable graph identity and fail closed after removal; the FBD runtime-fault vector loads and executes a faulting graph, maps the emitted runtime source identity, then follows that exact anchor through an offline graph edit.",
        ["crates/plc-compiler/src/source.rs", "crates/plc-compiler/src/ir.rs", "crates/plc-observability/src/navigation.rs", "crates/plc-system/src/session.rs"],
        ["crates/plc-compiler/tests/source_map_navigation.rs", "crates/plc-compiler/tests/diagnostic_source_mapping.rs", "crates/plc-language-tools/tests/fbd_contract.rs", "crates/plc-lad/tests/lad_contract.rs"],
    ),
    "VER-RTM-0001": assessment(
        READY,
        "The virtual controller implements typed boundary commands/state, scan input sampling, timed/OB execution, output commit and fault policy, virtual time, memory, timer/counter/edge state, and bounded deterministic work without process objects.",
        ["crates/plc-runtime/src/controller.rs", "crates/plc-runtime/src/model.rs", "crates/plc-runtime/src/boundary.rs"],
        ["crates/plc-runtime/tests/runtime_vectors.rs", "crates/plc-runtime/tests/observation_boundary.rs", "crates/plc-compiler/tests/runtime_vertical_slice.rs"],
    ),
    "VER-CPU-0001": assessment(
        READY,
        "A table-driven oracle classifies every externally reachable steady CPU state/action pair and separately proves startup failure, explicit fault reset, STOP output policy, warm restart, power cycle, and memory reset behavior.",
        ["crates/plc-runtime/src/controller.rs", "crates/plc-commissioning/src/universe.rs"],
        ["crates/plc-runtime/tests/cpu_state_matrix.rs", "crates/plc-runtime/tests/runtime_vectors.rs", "crates/plc-commissioning/tests/commissioning_vectors.rs"],
    ),
    "VER-FLT-0001": assessment(
        PARTIAL,
        "Divide, timer overflow, and work-budget faults have declared fatal CPU boundary behavior with source occurrence context; real provider events bridge into causal diagnostics.",
        ["crates/plc-runtime/src/controller.rs", "crates/plc-runtime/src/model.rs", "crates/plc-observability/src/runtime_diagnostics.rs"],
        ["crates/plc-runtime/tests/runtime_vectors.rs", "crates/plc-runtime/tests/fault_policy_vectors.rs", "crates/plc-runtime/tests/invocation_calls.rs", "crates/plc-compiler/tests/scl_control_flow.rs", "crates/plc-observability/tests/execution_vectors.rs"],
        [
            "Bounds-fault CPU response and causal-diagnostic vector is absent.",
            "Invariant-fault CPU response and causal-diagnostic vector is absent.",
            "Timer- and budget-fault vectors are not asserted through the diagnostic-provider seam.",
        ],
        "LANE-FAULT-SNAPSHOT-OBSERVATION",
    ),
    "VER-SNP-0001": assessment(
        READY,
        "Controller, force, trace, diagnostic, hardware, and complete virtual-I/O snapshots are content-addressed and atomically restored. Restore preview exposes and hash-binds complete ForceRegistry before/after records, canonical order, provenance, and complete VirtualIOBoundary state. The production replay executor reconstructs an EngineeringSession from the referenced aggregate snapshot, admits only the closed simulator ingress language, drives every recorded ingress and generated event, and compares independently calculated boundary regions. Canonical replay packages remain byte-stable, typed, causally ordered, branchable, and first-divergence reporting.",
        ["crates/plc-runtime/src/controller.rs", "crates/plc-observability/src/force.rs", "crates/plc-observability/src/trace.rs", "crates/plc-observability/src/diagnostics.rs", "crates/plc-system/src/session.rs", "crates/plc-system/src/replay_package.rs", "crates/plc-system/src/replay_executor.rs"],
        ["crates/plc-runtime/tests/runtime_vectors.rs", "crates/plc-observability/tests/observability_vectors.rs", "crates/plc-system/tests/system_journeys.rs", "crates/plc-system/tests/replay_verification.rs"],
    ),
    "VER-COM-0001": assessment(
        READY,
        "Commissioning classifies compatibility, produces deterministic preservation/invalidation previews, blocks active forces, commits atomically, and restores exact target state on injected failure.",
        ["crates/plc-commissioning/src/model.rs", "crates/plc-commissioning/src/universe.rs"],
        ["crates/plc-commissioning/tests/commissioning_vectors.rs", "crates/plc-commissioning/tests/observation_integration.rs"],
    ),
    "VER-ONL-0001": assessment(
        READY,
        "The virtual online-session machine binds epochs and comparison state, rejects stale commands, reconnects explicitly, preserves offline edits while RUN, and never implicitly mutates the loaded target.",
        ["crates/plc-commissioning/src/model.rs", "crates/plc-commissioning/src/universe.rs"],
        ["crates/plc-commissioning/tests/commissioning_vectors.rs", "crates/plc-system/tests/system_journeys.rs"],
    ),
    "VER-ONL-0002": assessment(
        READY,
        "One generated CPU-state oracle covers monitor publication, Modify, Force create/remove, trace arm/sample/abort and rejection receipts, with separate warm-restart and destructive reset-preview lifecycle vectors.",
        ["crates/plc-system/src/session.rs", "crates/plc-observability/src/monitor.rs", "crates/plc-observability/src/modify.rs", "crates/plc-observability/src/force.rs", "crates/plc-observability/src/trace.rs"],
        ["crates/plc-observability/tests/online_monitor_policy_matrix.rs", "crates/plc-observability/tests/execution_vectors.rs", "crates/plc-commissioning/tests/observation_integration.rs"],
    ),
    "VER-MON-0001": assessment(
        READY,
        "Watch persistence, canonical target resolution, quality/freshness/layer display, stale behavior, throttling neutrality, and session-loss/reconnect semantics share one authoritative publication-stream test surface.",
        ["crates/plc-observability/src/monitor.rs", "crates/plc-observability/src/target.rs", "crates/plc-system/src/session.rs"],
        ["crates/plc-observability/tests/online_monitor_policy_matrix.rs", "crates/plc-observability/tests/observability_vectors.rs", "crates/plc-observability/tests/execution_vectors.rs"],
    ),
    "VER-MOD-0001": assessment(
        READY,
        "Modify performs typed validation, boundary scheduling, idempotent receipt handling, aggregate atomicity, force-conflict rejection, cancellation, and explicit natural-writer overwrite behavior.",
        ["crates/plc-observability/src/modify.rs", "crates/plc-observability/src/execution.rs", "crates/plc-system/src/session.rs"],
        ["crates/plc-observability/tests/observability_vectors.rs", "crates/plc-observability/tests/execution_vectors.rs", "crates/plc-commissioning/tests/observation_integration.rs"],
    ),
    "VER-FRC-0001": assessment(
        READY,
        "The force registry is CAS/overlap aware, preserves natural values beneath effective forced values, follows approved lifecycle clearing, projects globally, and snapshots/rebinds deterministically.",
        ["crates/plc-observability/src/force.rs", "crates/plc-runtime/src/boundary.rs", "crates/plc-system/src/session.rs"],
        ["crates/plc-observability/tests/observability_vectors.rs", "crates/plc-observability/tests/execution_vectors.rs", "crates/plc-runtime/tests/observation_boundary.rs"],
    ),
    "VER-TRC-0001": assessment(
        READY,
        "Trace vectors execute immediate, Boolean edge, every numeric comparator, REAL/NaN, compound expression, and exact diagnostic triggers plus cadence, pre/post buffers, metadata, limits, gaps, abort, snapshot, export, save, and replay equality.",
        ["crates/plc-observability/src/trace.rs", "crates/plc-system/src/session.rs"],
        ["crates/plc-observability/tests/trace_trigger_matrix.rs", "crates/plc-observability/tests/observability_vectors.rs", "crates/plc-observability/tests/execution_vectors.rs", "tests/phase2/workbench-browser.e2e.mjs"],
    ),
    "VER-DIA-0001": assessment(
        READY,
        "The fixed diagnostic registry, DIA-ID-TLV-1 IDs, RootOccurrence convention, lifecycle, reserved fatal capacity, deterministic EVENT_GAP compaction, causal provider bridge, ordering, snapshot, and replay are directly implemented and tested.",
        ["crates/plc-observability/src/diagnostics.rs", "crates/plc-observability/src/runtime_diagnostics.rs", "crates/plc-system/src/session.rs"],
        ["crates/plc-observability/tests/observability_vectors.rs", "crates/plc-observability/tests/execution_vectors.rs"],
    ),
    "VER-NAV-0001": assessment(
        READY,
        "The identity-based navigation matrix covers definitions, uses, calls, address overlaps, primary/related locations, tombstones, invalid editable relations, and separately bound loaded/offline artifacts; the SCL projection preserves authored semantic roles.",
        ["crates/plc-observability/src/navigation.rs", "crates/plc-system/src/session.rs", "crates/plc-compiler/src/scl/semantics.rs"],
        ["crates/plc-observability/tests/navigation_matrix.rs", "crates/plc-language-tools/tests/semantic_navigation_matrix.rs", "crates/plc-system/tests/system_journeys.rs"],
    ),
    "VER-ISO-0001": assessment(
        READY,
        "The packaged adapters-off workbench runs the complete virtual build/load/scan workflow under browser, worker, child-process, Windows endpoint, CDP, and NetLog capture with counterfactual causal attribution and zero-attempt fail-closed assertions.",
        ["tools/phase2/run_isolation_counterfactual.mjs", "tools/phase2/isolation-counterfactual-lib.mjs"],
        ["tests/phase2/isolation-counterfactual.unit.mjs"],
    ),
    "VER-ISO-0002": assessment(
        READY,
        "Candidate-bound source/dependency capability policy and packaged inline-artifact scans cover native/FFI/device/process/network APIs, dynamic execution/imports, zero WASM imports, CSP, remote assets, local servers, updater strings, and background-network behavior.",
        ["tools/phase2/source_policy.py", "tools/phase2/run_isolation_counterfactual.mjs", "tools/phase2/isolation-counterfactual-lib.mjs", "tools/foundation/verify-isolation.mjs"],
        ["tools/phase2/test_source_policy.py", "tests/phase2/isolation-counterfactual.unit.mjs", "tests/phase1/policy-contract.json"],
    ),
    "VER-ISO-0003": assessment(
        PARTIAL,
        "The deterministic 27-case endpoint, URL, protocol, UNC, device, print, escape, and malformed-value corpus is bound by exact hashes and routed through file metadata open/create/replace, project display name, saved-project decode, SCL source, semantic navigation, both trace exports, and the typed Virtual Download target. Browser/worker/CDP/NetLog/Windows endpoint instrumentation remains fail-closed.",
        [
            "apps/foundation-shell/src/file-access-broker.ts",
            "apps/foundation-shell/src/runtime-wire.ts",
            "crates/plc-compiler/src/source.rs",
            "crates/plc-core/src/engine.rs",
            "crates/plc-core/src/model.rs",
            "crates/plc-core/src/package.rs",
            "crates/plc-observability/src/trace.rs",
            "tools/phase2/isolation-fuzz-corpus.tsv",
            "tools/phase2/isolation-counterfactual-lib.mjs",
            "tools/phase2/run_isolation_counterfactual.mjs",
        ],
        [
            "apps/foundation-shell/test/isolation-boundary-fuzz.test.ts",
            "crates/plc-compiler/tests/isolation_boundary_fuzz.rs",
            "crates/plc-core/tests/isolation_boundary_fuzz.rs",
            "crates/plc-observability/tests/isolation_boundary_fuzz.rs",
            "crates/windows-project-broker/tests/isolation_boundary_fuzz.rs",
            "tests/phase2/isolation-counterfactual.unit.mjs",
            "tests/support/isolation_fuzz.rs",
        ],
        [
            "Two genuine operator-controlled live-LAN topology runs with stable pre/post fingerprints and invariant product output have not been collected.",
            "The concrete native shell host needed to exercise the admitted adapters-on renderer/broker configuration is not implemented.",
        ],
        "LANE-ISOLATION-COUNTERFACTUAL",
    ),
    "VER-ISO-0004": assessment(
        PARTIAL,
        "Virtual Download accepts only canonical controller identity. The versioned Windows project broker exposes only typed open/create/replace/revoke operations, validates fixed-native-local backing and bounded names before selected-byte I/O, keeps paths and storage tokens private, and byte-verifies replacement. Project save, canonical replay verification, canonical trace JSON, and trace CSV have closed fail-closed vendor/deployable tests.",
        [
            "apps/foundation-shell/src/file-access-broker.ts",
            "crates/plc-core/src/package.rs",
            "crates/plc-observability/src/trace.rs",
            "crates/plc-system/src/replay_package.rs",
            "crates/windows-project-broker/src/lib.rs",
            "crates/windows-project-broker/src/windows.rs",
        ],
        [
            "apps/foundation-shell/test/file-access-broker.test.ts",
            "apps/foundation-shell/test/isolation-boundary-fuzz.test.ts",
            "crates/plc-core/tests/isolation_boundary_fuzz.rs",
            "crates/plc-observability/tests/isolation_boundary_fuzz.rs",
            "crates/plc-system/tests/system_journeys.rs",
            "crates/windows-project-broker/tests/isolation_boundary_fuzz.rs",
            "crates/windows-project-broker/tests/shell_request_flow.rs",
        ],
        [
            "No concrete native shell host installs the approved bridge into the production renderer, so open/create/replace cannot yet receive productionPathExercised credit.",
            "Complete exact-candidate Windows runtime logs have not yet bound the native backing attestations and four export-surface results into one evidence record.",
        ],
        "LANE-ISOLATION-COUNTERFACTUAL",
    ),
    "VER-ISO-0005": assessment(
        PARTIAL,
        "The exit gate and isolation runner bind evidence to exact commit/tree/source/catalog hashes, require the exact approved two-row Windows configuration set with measured runtime product/version/executable hashes, validate exact corpus/boundary/export/native-backing objects, require stable distinct topology fingerprints, and reject skipped, flaky, unavailable, stale, inconclusive, crash, canned, or string-only platform records.",
        [
            "tools/phase2/isolation-closure-evidence.schema.json",
            "tools/phase2/isolation-counterfactual-lib.mjs",
            "tools/phase2/LIVE_LAN_TOPOLOGY_PROTOCOL.md",
            "tools/phase2/run_isolation_counterfactual.mjs",
            "tools/phase2/transform_isolation_closure.mjs",
            "tools/phase2/verify_phase2.py",
        ],
        ["tests/phase2/isolation-counterfactual.unit.mjs", "tools/phase2/test_verify_phase2_gate.py"],
        [
            "No current machine-readable runtime isolation package binds complete real logs for both approved Windows configuration rows.",
            "The adapters-on topology pair and adapters-off before/after host-state runs remain unexecuted.",
        ],
        "LANE-ISOLATION-COUNTERFACTUAL",
    ),
    "VER-GOV-0001": assessment(
        READY,
        "A deterministic machine audit binds source authority and enumerates normative vocabulary, all 937 unique requirements, Appendix H coverage, clarification dispositions, phase reservations, exclusions, open decisions, and terminal stop rules.",
        ["tools/phase2/extract_phase2_requirements.py", "tools/phase2/governance_audit.py", "tools/phase2/verify_phase2.py"],
        ["tools/phase2/test_extract_phase2_requirements.py", "tools/phase2/test_governance_audit.py", "tools/phase2/test_verify_phase2_gate.py"],
    ),
    "VER-ACC-0001": assessment(
        READY,
        "The production browser workflow edits one canonical project; builds and commits Virtual Download; goes online/RUN; monitors, modifies, forces, traces, diagnoses, and navigates; captures/restores the aggregate snapshot; exports and executes a non-empty canonical closed replay package through the production Rust/WASM/worker path with one reproduced state boundary and a content-bound receipt; then saves, closes, and reopens through the typed broker with a fresh runtime.",
        [
            "crates/plc-system/src/replay_executor.rs",
            "crates/plc-engineering-wasm/src/system_bridge.rs",
            "crates/plc-engineering-wasm/src/kernel_bridge.rs",
            "apps/foundation-shell/src/engineering-worker-handler.ts",
            "apps/foundation-shell/src/wasm-kernel.ts",
            "apps/foundation-shell/src/App.tsx",
            "apps/foundation-shell/src/EngineeringWorkbench.tsx",
            "apps/foundation-shell/src/RuntimeWorkbench.tsx",
        ],
        [
            "tests/phase2/workbench-browser.e2e.mjs",
            "crates/plc-engineering-wasm/src/kernel_bridge.rs",
            "apps/foundation-shell/test/engineering-worker-handler.test.ts",
            "apps/foundation-shell/test/runtime-workbench.test.ts",
            "crates/plc-system/tests/system_journeys.rs",
        ],
    ),
}


LANES: dict[str, dict[str, Any]] = {
    "LANE-PROFILE-HARDWARE-SYMBOLS": {
        "title": "Profile completeness and physical-universe fault matrix",
        "priority": 2,
        "deliverable": "Generate the EDU-21 field oracle and complete hardware, channel, byte-order, fault-propagation, and address-form matrices.",
    },
    "LANE-LANGUAGE-BUILD-SOURCEMAP": {
        "title": "Full SCL runtime, instruction registry, dependency properties, and build modes",
        "priority": 4,
        "deliverable": "Execute structured control flow, add LIMIT/FILL/BLKMOVE parity, property-test invalidation, expose all build modes, and close cross-language source-map vectors.",
    },
    "LANE-FAULT-SNAPSHOT-OBSERVATION": {
        "title": "Fault, aggregate snapshot, online matrix, monitor, and trace closure",
        "priority": 5,
        "deliverable": "Build generated CPU/fault/online matrices and one aggregate replayable snapshot covering runtime, I/O, force, trace, and diagnostics.",
    },
    "LANE-ISOLATION-COUNTERFACTUAL": {
        "title": "Packaged counterfactual isolation instrumentation",
        "priority": 6,
        "deliverable": "Capture application/child/OS attempts, fuzz every typed boundary, scan the packaged candidate, and emit multi-platform exact-candidate isolation evidence.",
    },
    "LANE-GOVERNANCE-ENDTOEND": {
        "title": "Governance completeness and causal end-to-end replay",
        "priority": 7,
        "deliverable": "Complete vocabulary/clarification/reservation audits and extend the real UI journey through causal diagnosis and deterministic replay.",
    },
}


class AuditError(RuntimeError):
    """The audit cannot be generated honestly."""


def sha256_bytes(value: bytes) -> str:
    return hashlib.sha256(value).hexdigest().upper()


def sha256_file(path: Path) -> str:
    return sha256_bytes(path.read_bytes())


def load_json(path: Path) -> dict[str, Any]:
    try:
        value = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, UnicodeError, json.JSONDecodeError) as exc:
        raise AuditError(f"cannot read {path}: {exc}") from exc
    if not isinstance(value, dict):
        raise AuditError(f"{path} must contain a JSON object")
    return value


def evidence_surface(root: Path) -> tuple[list[str], str]:
    # Walk only the audited source/test roots.  A repository-wide rglob would
    # descend into node_modules and target before it could filter them, making
    # a read-only audit unnecessarily expensive and nondeterministically slow.
    surface_roots = (
        "crates",
        "apps/foundation-shell/src",
        "apps/foundation-shell/test",
        "packages/foundation-contract/src",
        "packages/foundation-contract/test",
        "packages/plc-contract/src",
        "packages/plc-contract/test",
        "tests/phase2",
        "tools/phase2",
    )
    suffixes = {
        ".rs",
        ".toml",
        ".ts",
        ".tsx",
        ".js",
        ".jsx",
        ".mjs",
        ".cjs",
        ".py",
        ".json",
    }
    paths: list[str] = [
        value
        for value in ("Cargo.toml", "Cargo.lock", "package.json", "pnpm-lock.yaml", "pnpm-workspace.yaml")
        if (root / value).is_file()
    ]
    for relative_root in surface_roots:
        directory = root / PurePosixPath(relative_root)
        if not directory.is_dir():
            continue
        for path in directory.rglob("*"):
            if not path.is_file() or path.suffix.lower() not in suffixes:
                continue
            relative = path.relative_to(root).as_posix()
            if "target/" in relative or "__pycache__/" in relative:
                continue
            paths.append(relative)
    paths = sorted(set(paths))
    manifest = b"".join(
        f"{sha256_file(root / PurePosixPath(path))}  {path}\n".encode("utf-8") for path in paths
    )
    return paths, sha256_bytes(manifest)


def validate_assessments(root: Path, verification_ids: set[str]) -> None:
    if set(ASSESSMENTS) != verification_ids:
        missing = sorted(verification_ids - set(ASSESSMENTS))
        extra = sorted(set(ASSESSMENTS) - verification_ids)
        raise AuditError(f"assessment inventory drift: missing={missing}, extra={extra}")
    lane_members: dict[str, set[str]] = defaultdict(set)
    for verification_id, record in ASSESSMENTS.items():
        classification = record["classification"]
        uncovered = record["uncoveredProofClauses"]
        lane = record["gapLaneId"]
        if classification not in ALLOWED_CLASSIFICATIONS:
            raise AuditError(f"{verification_id} has invalid classification {classification}")
        if classification == READY and (uncovered or lane is not None):
            raise AuditError(f"{verification_id} READY must have no gaps or lane")
        if classification in {PARTIAL, MISSING} and (not uncovered or lane not in LANES):
            raise AuditError(f"{verification_id} incomplete classification needs gaps and one known lane")
        if classification == MISSING and (record["implementationPaths"] or record["testPaths"]):
            raise AuditError(f"{verification_id} MISSING cannot cite supporting paths")
        for family in ("implementationPaths", "testPaths"):
            values = record[family]
            if len(values) != len(set(values)):
                raise AuditError(f"{verification_id} has duplicate {family}")
            for relative in values:
                normalized = PurePosixPath(relative)
                if normalized.is_absolute() or ".." in normalized.parts:
                    raise AuditError(f"{verification_id} cites unsafe path {relative}")
                if not (root / normalized).is_file():
                    raise AuditError(f"{verification_id} cites missing path {relative}")
        if lane is not None:
            lane_members[lane].add(verification_id)
    if not set(lane_members).issubset(LANES):
        raise AuditError("an incomplete verification names an undeclared gap lane")


def requirement_signal(classifications: Sequence[str]) -> str:
    if not classifications:
        return "NO_REVIEWED_VERIFICATION"
    if all(value == READY for value in classifications):
        return "ALL_REVIEWED_MAPPINGS_EVIDENCE_READY"
    if any(value in {READY, PARTIAL} for value in classifications):
        return "SOME_STATIC_SUPPORT_IN_REVIEWED_MAPPING"
    return "NO_IMPLEMENTED_REVIEWED_PROOF"


def build_audit(root: Path) -> dict[str, Any]:
    requirement_path = root / "requirements" / "phase2-requirements.json"
    catalog_path = root / "requirements" / "phase2-verification-catalog.json"
    reviewed_mapping_path = root / reviewed_requirement_mapping.REVIEWED_MAPPING_PATH
    requirements = load_json(requirement_path)
    catalog = load_json(catalog_path)
    reviewed_mapping = load_json(reviewed_mapping_path)
    requirement_records = requirements.get("requirements")
    verification_records = catalog.get("verificationRecords")
    mapping_records = catalog.get("requirementMappingSkeleton")
    if not isinstance(requirement_records, list) or len(requirement_records) != EXPECTED_REQUIREMENTS:
        raise AuditError(f"expected exactly {EXPECTED_REQUIREMENTS} requirements")
    if not isinstance(verification_records, list) or len(verification_records) != EXPECTED_VERIFICATIONS:
        raise AuditError(f"expected exactly {EXPECTED_VERIFICATIONS} Appendix H records")
    if not isinstance(mapping_records, list) or len(mapping_records) != EXPECTED_REQUIREMENTS:
        raise AuditError("requirement mapping skeleton must enumerate all requirements")

    directive_path_value = requirements.get("directive", {}).get("path")
    if not isinstance(directive_path_value, str):
        raise AuditError("requirement registry has no directive path")
    directive_path = root / PurePosixPath(directive_path_value.replace("\\", "/"))
    try:
        reviewed_by_requirement = reviewed_requirement_mapping.validate_reviewed_mapping(
            reviewed_mapping,
            requirements,
            catalog,
            requirement_registry_sha256=sha256_file(requirement_path),
            verification_catalog_sha256=sha256_file(catalog_path),
            directive_sha256=sha256_file(directive_path),
            expected_requirement_count=EXPECTED_REQUIREMENTS,
            expected_verification_count=EXPECTED_VERIFICATIONS,
        )
    except reviewed_requirement_mapping.ReviewedMappingError as exc:
        raise AuditError(str(exc)) from exc

    verification_by_id = {
        record.get("verificationId"): record for record in verification_records if isinstance(record, dict)
    }
    if len(verification_by_id) != EXPECTED_VERIFICATIONS or not all(
        isinstance(key, str) for key in verification_by_id
    ):
        raise AuditError("Appendix H IDs must be unique strings")
    verification_ids = set(verification_by_id)
    validate_assessments(root, verification_ids)

    output_verifications: list[dict[str, Any]] = []
    for verification_id in sorted(verification_ids):
        source = verification_by_id[verification_id]
        assessed = ASSESSMENTS[verification_id]
        covered = [source["minimumProof"]] if assessed["classification"] == READY else [assessed["support"]]
        if assessed["classification"] == MISSING:
            covered = []
        output_verifications.append(
            {
                "verificationId": verification_id,
                "classification": assessed["classification"],
                "minimumProof": source["minimumProof"],
                "primaryRequirementAreas": source["primaryRequirementAreas"],
                "appendixHSourcePointer": source["sourcePointer"],
                "appendixHRowSha256": source["rowSha256"],
                "staticSupportAssessment": assessed["support"],
                "coveredProofClauses": covered,
                "uncoveredProofClauses": assessed["uncoveredProofClauses"],
                "implementationPaths": assessed["implementationPaths"],
                "testPaths": assessed["testPaths"],
                "gapLaneId": assessed["gapLaneId"],
                "executionEvidenceIds": [],
                "verificationCredit": "NONE",
            }
        )

    mapping_by_requirement: dict[str, dict[str, Any]] = {}
    for record in mapping_records:
        if not isinstance(record, dict) or not isinstance(record.get("requirementId"), str):
            raise AuditError("invalid requirement mapping record")
        requirement_id = record["requirementId"]
        if requirement_id in mapping_by_requirement:
            raise AuditError(f"duplicate requirement mapping {requirement_id}")
        mapping_by_requirement[requirement_id] = record

    output_requirements: list[dict[str, Any]] = []
    area_counts: dict[str, Counter[str]] = defaultdict(Counter)
    truth_counts: Counter[str] = Counter()
    for record in sorted(requirement_records, key=lambda item: item["id"]):
        requirement_id = record["id"]
        mapping = mapping_by_requirement.get(requirement_id)
        if mapping is None:
            raise AuditError(f"missing mapping for {requirement_id}")
        candidate_ids = mapping.get("candidateVerificationIds")
        if not isinstance(candidate_ids, list) or any(value not in verification_ids for value in candidate_ids):
            raise AuditError(f"invalid candidate mapping for {requirement_id}")
        candidate_ids = sorted(set(candidate_ids))
        reviewed_row = reviewed_by_requirement[requirement_id]
        selected_ids = list(reviewed_row["selectedVerificationIds"])
        classifications = [ASSESSMENTS[value]["classification"] for value in selected_ids]
        signal = requirement_signal(classifications)
        area_counts[record["area"]][signal] += 1
        truth_counts[str(record["truthState"])] += 1
        output_requirements.append(
            {
                "requirementId": requirement_id,
                "area": record["area"],
                "truthState": record["truthState"],
                "requirementTextSha256": record["textSha256"],
                "candidateVerificationIds": candidate_ids,
                "selectedVerificationIds": selected_ids,
                "candidateClassifications": {
                    value: ASSESSMENTS[value]["classification"] for value in candidate_ids
                },
                "selectedClassifications": {
                    value: ASSESSMENTS[value]["classification"] for value in selected_ids
                },
                "mappingStatus": "REVIEWED",
                "mappingDisposition": reviewed_row["disposition"],
                "mappingReviewerRationale": reviewed_row["reviewerRationale"],
                "coverageSignal": signal,
                "executionEvidenceIds": [],
                "verificationCredit": "NONE",
            }
        )

    lane_members: dict[str, list[str]] = defaultdict(list)
    for verification_id, record in ASSESSMENTS.items():
        if record["gapLaneId"] is not None:
            lane_members[record["gapLaneId"]].append(verification_id)
    lanes = []
    for lane_id, definition in sorted(LANES.items(), key=lambda item: item[1]["priority"]):
        members = sorted(lane_members[lane_id])
        if not members:
            continue
        lanes.append(
            {
                "laneId": lane_id,
                **definition,
                "verificationIds": members,
                "uncoveredClauseCount": sum(
                    len(ASSESSMENTS[verification_id]["uncoveredProofClauses"])
                    for verification_id in members
                ),
            }
        )

    surface_paths, surface_hash = evidence_surface(root)
    classification_counts = Counter(
        record["classification"] for record in output_verifications
    )
    signal_counts = Counter(record["coverageSignal"] for record in output_requirements)
    generator_path = Path(__file__).resolve()
    return {
        "schemaVersion": 1,
        "auditKind": "PHASE_2_STATIC_IMPLEMENTATION_TEST_COVERAGE_AUDIT",
        "generatedBy": "tools/phase2/generate_phase2_coverage_audit.py",
        "generatorSha256": sha256_file(generator_path),
        "truthPolicy": {
            "staticAuditOnly": True,
            "grantsVerificationCredit": False,
            "readyMeaning": "Production and directly applicable executable test paths exist for every stated minimum-proof clause; current-candidate execution evidence is still required.",
            "partialMeaning": "Some production/test support exists, but the named minimum-proof clauses remain uncovered.",
            "missingMeaning": "No directly applicable implementation/evidence harness exists for the minimum proof.",
            "requirementMappingsRemainUnreviewed": False,
            "reviewedMappingDoesNotGrantVerificationCredit": True,
        },
        "binding": {
            "basis": "WORKTREE_BYTES_EXCLUDING_GENERATED_EVIDENCE_OUTPUTS",
            "directiveSha256": sha256_file(directive_path),
            "requirementRegistrySha256": sha256_file(requirement_path),
            "verificationCatalogSha256": sha256_file(catalog_path),
            "reviewedRequirementMappingSha256": sha256_file(reviewed_mapping_path),
            "reviewedMappingRowsSha256": reviewed_mapping["binding"]["reviewedRowsSha256"],
            "evidenceSurfaceSha256": surface_hash,
            "evidenceSurfaceFileCount": len(surface_paths),
        },
        "summary": {
            "requirementsExpected": EXPECTED_REQUIREMENTS,
            "requirementsEnumerated": len(output_requirements),
            "verificationsExpected": EXPECTED_VERIFICATIONS,
            "verificationsEnumerated": len(output_verifications),
            "verificationClassificationCounts": dict(sorted(classification_counts.items())),
            "requirementTruthStateCounts": dict(sorted(truth_counts.items())),
            "requirementCoverageSignalCounts": dict(sorted(signal_counts.items())),
            "gapLaneCount": len(lanes),
            "uncoveredProofClauseCount": sum(
                len(record["uncoveredProofClauses"]) for record in output_verifications
            ),
            "verificationCreditGranted": 0,
            "reviewedRequirementMappingCount": len(reviewed_by_requirement),
        },
        "verificationAssessments": output_verifications,
        "requirementCoverage": output_requirements,
        "requirementAreaCoverage": [
            {
                "area": area,
                "requirementCount": sum(counts.values()),
                "coverageSignalCounts": dict(sorted(counts.items())),
            }
            for area, counts in sorted(area_counts.items())
        ],
        "independentGapLanes": lanes,
    }


def render_json(audit: Mapping[str, Any]) -> str:
    return json.dumps(audit, indent=2, ensure_ascii=False) + "\n"


def render_report(audit: Mapping[str, Any]) -> str:
    summary = audit["summary"]
    counts = summary["verificationClassificationCounts"]
    lines = [
        "# Phase 2 Static Coverage Audit",
        "",
        "This report is a static implementation/test readiness assessment. It grants no verification credit and contains no executable evidence claim.",
        "",
        "## Result",
        "",
        f"All {summary['requirementsEnumerated']} extracted requirements and all {summary['verificationsEnumerated']} Appendix H minimum proofs are enumerated. Classifications: {counts.get(READY, 0)} `IMPLEMENTED_EVIDENCE_READY`, {counts.get(PARTIAL, 0)} `PARTIAL`, and {counts.get(MISSING, 0)} `MISSING`. There are {summary['uncoveredProofClauseCount']} explicitly recorded uncovered proof clauses.",
        "",
        "`IMPLEMENTED_EVIDENCE_READY` means only that production and directly applicable tests exist for the full static clause. Candidate-bound execution, logs, negative/integration/isolation evidence, review, and Scott's acceptance remain outstanding.",
        "",
        "Exact implementation and test paths for every row are recorded in `PHASE2_COVERAGE_AUDIT.json`; this concise view focuses on disposition and uncovered proof.",
        "",
        "## Appendix H assessments",
        "",
        "| Verification | Classification | Uncovered proof |",
        "|---|---|---|",
    ]
    for record in audit["verificationAssessments"]:
        uncovered = "<br>".join(record["uncoveredProofClauses"]) or "None found in this static pass; executable evidence remains required."
        lines.append(f"| `{record['verificationId']}` | `{record['classification']}` | {uncovered} |")
    lines.extend(["", "## Highest-leverage independent work lanes", ""])
    for lane in audit["independentGapLanes"]:
        members = ", ".join(f"`{value}`" for value in lane["verificationIds"])
        lines.extend(
            [
                f"### {lane['priority']}. {lane['title']}",
                "",
                f"Scope: {members}",
                "",
                f"Deliverable: {lane['deliverable']}",
                "",
            ]
        )
    lines.extend(
        [
            "## Requirement inventory posture",
            "",
            "Every requirement record remains at its extracted truth state. Its explicit reviewed Appendix H selection is source-bound to the exact directive, registry, catalog, requirement text hash, and reviewed-row inventory. The machine-readable audit lists all requirement IDs, candidate and selected proof IDs, bounded review rationale, static signals, empty execution-evidence IDs, and zero verification credit.",
            "",
            f"Evidence-surface binding: `{audit['binding']['evidenceSurfaceSha256']}` across {audit['binding']['evidenceSurfaceFileCount']} production/test/governance files. Exact candidate commit/tree binding remains the responsibility of the Phase 2 exit gate.",
        ]
    )
    return "\n".join(lines) + "\n"


def parse_args(argv: Sequence[str] | None = None) -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path.cwd())
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--json-output", type=Path, default=Path("evidence/phase2/PHASE2_COVERAGE_AUDIT.json"))
    parser.add_argument("--report-output", type=Path, default=Path("evidence/phase2/PHASE2_COVERAGE_AUDIT.md"))
    return parser.parse_args(argv)


def resolved_output(root: Path, value: Path) -> Path:
    return value.resolve(strict=False) if value.is_absolute() else (root / value).resolve(strict=False)


def check_or_write(path: Path, expected: str, check: bool) -> bool:
    if check:
        if not path.is_file():
            print(f"STALE missing {path}")
            return False
        actual = path.read_text(encoding="utf-8")
        if actual != expected:
            print(f"STALE {path} expected={sha256_bytes(expected.encode())} actual={sha256_bytes(actual.encode())}")
            return False
        print(f"CURRENT {path} SHA256={sha256_bytes(expected.encode())}")
        return True
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(expected, encoding="utf-8", newline="")
    print(f"WROTE {path} SHA256={sha256_bytes(expected.encode())}")
    return True


def main(argv: Sequence[str] | None = None) -> int:
    args = parse_args(argv)
    try:
        root = args.root.resolve(strict=True)
        audit = build_audit(root)
        json_output = resolved_output(root, args.json_output)
        report_output = resolved_output(root, args.report_output)
        current = check_or_write(json_output, render_json(audit), args.check)
        current = check_or_write(report_output, render_report(audit), args.check) and current
        print(
            "PHASE2_COVERAGE_AUDIT "
            f"requirements={audit['summary']['requirementsEnumerated']} "
            f"verifications={audit['summary']['verificationsEnumerated']} "
            f"ready={audit['summary']['verificationClassificationCounts'].get(READY, 0)} "
            f"partial={audit['summary']['verificationClassificationCounts'].get(PARTIAL, 0)} "
            f"missing={audit['summary']['verificationClassificationCounts'].get(MISSING, 0)} "
            "credit=0"
        )
        return 0 if current else 1
    except (AuditError, OSError, UnicodeError, ValueError, KeyError, TypeError) as exc:
        print(f"PHASE2_COVERAGE_AUDIT_TOOL_ERROR {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
