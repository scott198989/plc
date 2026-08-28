# Phase 2 independent replay verifier

This executable is an external evidence producer, not a renderer or native-host
receipt parser. It reads the preserved, post-replace `.vlabproj` bytes once,
decodes and byte-for-byte re-encodes the canonical package, finds exactly one
active controller, and executes the fixed Phase 2 workflow through the real
`EngineeringSession` and `EngineeringReplayExecutor` APIs.

The admitted project is exact: UUID ordinals root `3`, Virtual network `5`,
Controller `7`, Local rack `9`, VDI16 `11`, VDO16 `13`, empty SCL
`Main_cycle` `15`, its three interface members `16..18`, and Save As document
`20`. The request-derived runtime sequence is Build, Power on, Preview load
STOP, Commit load, Go online, RUN, one scan (request ordinal `30`), STOP, and
Capture snapshot. Any graph, payload, ordinal, identity, or order drift fails.

```powershell
cargo run --locked -p phase2-independent-replay-verifier -- derive <native-committed-project.vlabproj>
```

The single stdout line is canonical, versioned JSON. `check-claim` repeats the
full derivation and accepts a claimed result only when its bytes are exactly the
canonical independently derived output. Both modes fail closed on malformed or
non-canonical projects, ambiguous controller state, wrong verification UUID
lineage, wrong workflow identity or order, scan-count drift, empty replay data,
boundary-count drift, or replay divergence.

For native-finalizer integration, build this crate from the clean exact
candidate commit, bind the executable and its reported `sourceIdentitySha256`
into the candidate manifest, run `derive` over the preserved committed project,
and compare these independently derived fields with the raw native receipt:

- `controlledInputSha256`
- `deterministicOutputSha256`
- `runtimeReplaySha256`
- `canonicalReplaySha256`
- `verifiedReplayEventCount`
- `verifiedReplayBoundaryCount`

The external result, stdout bytes, executable digest, and preserved project
digest should then be included in the native evidence inventory before the
finalizer evaluates PASS.
