# DataHub v1 delivery plan

## Current status

- M0-M7 are implemented and pass the isolated local quality gate.
- M3 includes a multi-field primitive/enum/array/reference Schema designer,
  per-field C/S/E policies, 256-row VTable blocks with one-block prefetch,
  server-side exact filtering and sorting, and optimistic inline cell writes.
- M4 adds stable FieldId formulas, cycle diagnostics, Native/Wasmtime parity,
  computed-field previews/applies, stable-ID XLSX round trips and atomic rollback.
- M5 pins builds to repeatable-read schema/data snapshots and target/plugin
  inputs, emits deterministic manifests, and provides Rust/C#/TypeScript plus
  JSON/CSV/XML/BSON/Protobuf/Lua output with stable Protobuf wire IDs.
- M6 defines the WIT Component contract, hash-pinned package manifest and exact
  version registry, with no ambient WASI capabilities and bounded fuel, time,
  memory, input, and output.
- M8 remains acceptance scope and must not be reported as complete.

1. M0: Docker-first repository, PostgreSQL, process boundaries, CI, and GitFlow.
2. M1: stable IDs, `TypeAst`, `ConfigValue`, validation, and Target IR.
3. M2: PostgreSQL persistence, revisions, outbox, audit, local accounts, and RBAC.
4. M3: Schema designer and VTable block-based configuration editing. Complete.
5. M4: FieldId formula AST, Native/WASM evaluation, and XLSX round trips. Complete.
6. M5: Rust/C#/TypeScript code generation and six built-in data codecs. Complete.
7. M6: Wasmtime Component/WIT plugin platform and capability sandbox. Complete.
8. M7: PostgreSQL projection sync, release approval, publishing, and rollback. Complete.
9. M8: security, operations, recovery, performance, and final acceptance.

Every milestone is delivered through a feature branch and pull request after
`scripts/quality-gate.ps1` passes locally. Paid cloud CI is optional and is not
a merge dependency.
