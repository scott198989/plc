# `@govs/plc-contract`

This package is the versioned browser/worker transport contract for the Phase 2
PLC engineering core. It contains TypeScript wire types and strict runtime
validation only. It does not execute PLC instructions, compile programs,
invent domain results, persist projects, or maintain a second copy of runtime
state.

## Boundary rules

- Phase 2 messages use `schemaVersion: 2` and one of the exact
  `plc.command`, `plc.query`, `plc.result`, or `plc.event` envelopes.
- The frozen Phase 1 `foundation.health` command/result remains accepted
  exactly for compatibility.
- Every record is exact-key and every discriminated variant is closed. Unknown
  kinds, versions, fields, receipt domains, and canonical value types fail
  validation.
- UUIDs, hashes, decimal integers, floating-point bit patterns, source anchors,
  revision preconditions, transaction IDs, idempotency keys, event sequences,
  and serialized message sizes have canonical bounded representations.
- Successful queries carry a query-kind-specific canonical snapshot receipt.
  Project, hardware, and program snapshots contain displayable worker-owned
  state so the UI does not need to maintain competing domain truth.
- Successful commands carry a receipt from the command's domain. Rejected or
  blocked commands cannot claim mutation, events, undo state, or a receipt.

## Public entry points

```ts
import {
  decodePlcMessage,
  encodePlcMessage,
  validateCanonicalTypedValue,
  validateDomainCommand,
  validateDomainQuery,
  validateDomainReceipt,
  validatePlcMessage,
} from "@govs/plc-contract";
```

`decodePlcMessage` is the boundary entry point for untrusted serialized JSON.
`encodePlcMessage` validates before serialization. The narrower validators are
available for domain code and focused tests.

## Verification

```text
pnpm --filter @govs/plc-contract typecheck
pnpm --filter @govs/plc-contract test
```
