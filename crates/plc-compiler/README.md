# plc-compiler

`plc-compiler` is the capability-free Phase 2 compiler boundary. It owns
immutable build attempts, deterministic reports and artifact identity, the
original build-diagnostic registry, dependency-aware software scopes, verified
typed PLC IR, source maps/probes, and the first genuine SCL vertical slice.

The SCL slice lexes, recovery-parses, binds against the canonical
`plc-program` block interface, type-checks and performs definite-assignment
analysis, then lowers assignments, expressions, `IF` control flow, and
`RETURN` into verified simulator data. Unsupported syntax is retained and
diagnosed; it never becomes runnable IR.

Language services consume `plc_compiler::scl::analyze_scl`, which returns an
immutable semantic snapshot of canonical interface symbols, resolved and
unresolved occurrences, expression types, diagnostics, recovery tokens, and
folding spans. The snapshot is produced by the same lexer, parser, binder,
type-checker, and definite-assignment analysis used by compilation; it is not a
second language implementation.

For the admitted linear SCL subset, a deterministic adapter materializes typed
SSA values into bounded runtime memory, preserves operation/source/probe
identity, seals the production `plc-runtime` artifact package, and exposes
stable block/member mappings for observation. Compiler IR that the production
runtime cannot execute is rejected explicitly instead of being approximated.

This crate is `no_std`, performs no I/O, uses no host clock or entropy, and has
no network, filesystem, process, device, FFI, `eval`, host-code generation,
LAD/FBD, UI, or runtime execution path.
