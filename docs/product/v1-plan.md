# DataHub v1 delivery plan

1. M0: Docker-first repository, PostgreSQL, process boundaries, CI, and GitFlow.
2. M1: stable IDs, `TypeAst`, `ConfigValue`, validation, and Target IR.
3. M2: PostgreSQL persistence, revisions, outbox, audit, local accounts, and RBAC.
4. M3: Schema designer and VTable block-based configuration editing.
5. M4: FieldId formula AST, Native/WASM evaluation, and XLSX round trips.
6. M5: Rust/C#/TypeScript code generation and six built-in data codecs.
7. M6: Wasmtime Component/WIT plugin platform and capability sandbox.
8. M7: PostgreSQL projection sync, release approval, publishing, and rollback.

Every milestone is delivered through a feature branch and CI-gated pull request.
