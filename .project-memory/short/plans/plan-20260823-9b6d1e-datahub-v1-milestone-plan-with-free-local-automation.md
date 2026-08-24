<!-- PROJECT_MEMORY
{
  "blockers": [],
  "confidence": "high",
  "created_at": "2026-08-23T18:53:11Z",
  "derived_from": [
    "PLAN-20260823-686A1E"
  ],
  "event_id": "datahub-plan-full-v1-local-automation-v1",
  "id": "PLAN-20260823-9B6D1E",
  "kind": "plan",
  "next_actions": [
    "Integrate verified M8, complete the GitFlow v1 release to main and record final integrated evidence."
  ],
  "review_after": "2026-09-06",
  "schema_version": 1,
  "scope": [
    "apps",
    "crates",
    "deploy",
    "migrations",
    "plugins",
    "tests",
    "tools",
    "web"
  ],
  "sensitivity": "internal",
  "sources": [
    "Explicit user scope change on 2026-08-24 and accepted DataHub v1 plans.",
    "TASK-20260823-2A43C1 and DEC-20260823-C69FFA.",
    "RPT-20260823-FE85CD",
    "RPT-20260823-118D95",
    "RPT-20260823-CA61E0",
    "RPT-20260823-E99D5D",
    "RPT-20260823-38DC17",
    "RPT-20260823-EE0875",
    "RPT-20260823-1A4BFC",
    "RPT-20260823-9F7FDD",
    "RPT-20260824-242227",
    "RPT-20260824-61086A"
  ],
  "status": "active",
  "summary": "M0-M8 are functionally complete and the full local gate passes; M8 feature and v1 release integration remain before plan completion.",
  "supersedes": [],
  "tags": [
    "automated-testing",
    "docker",
    "local-ci",
    "plan",
    "v1"
  ],
  "task_id": "TASK-20260823-2A43C1",
  "tier": "short",
  "title": "DataHub v1 milestone plan with free local automation",
  "type_version": 1,
  "updated_at": "2026-08-24T15:38:24Z",
  "valid_as_of": "2026-08-24"
}
-->

# DataHub v1 milestone plan with free local automation

## Progress

- M0 transition: completed on develop at squash commit 026cb6f; local quality gate passed.
- M1 domain/kernel: completed, including resolved UUIDv7/type/target/testing gaps.
- M2 persistence/auth: completed through migrations 0002-0003, revisions, audit/outbox, auth/RBAC, builds and projection foundations.
- M3 schema/configuration UX: completed with multi-field design, typed rows, inline VTable editing, block prefetch/cache, optimistic saves, filter/sort and browser acceptance.
- M4 formula/XLSX: completed with stable FieldId formulas, Native/Wasmtime parity, cached-value-only XLSX round trips and atomic persistence.
- M5 deterministic build/export: completed and merged to develop at 2aaf6c8 with the full built-in artifact matrix and generated-code compilation.
- M6 plugin platform: completed and merged to develop at 636a131 with WIT Components, immutable exact-version installation and deny-by-default Wasmtime limits.
- M7 synchronization/release/rollback: completed and integrated through PR #7 at a986c5d.
- M8 hardening/final acceptance: functionally completed and fully verified on feature/m8-hardening-acceptance; GitHub/release integration remains.

## M0 Transition - Completed

Remove required GitHub status checks and disable/cancel billing-dependent auto-merge. Run the local gate against the existing M0 commit, integrate it into develop with the approved squash strategy, delete the feature branch, and keep GitHub Actions optional/non-blocking. Add a single documented local quality-gate entrypoint that fails fast and writes inspectable results.

## M1 Domain and Compilation Kernel - Completed

Implement stable UUIDv7 identities, TypeAst, ConfigValue, table/field/custom-type models, TargetRule, validation diagnostics, reference and target-leak checks, and deterministic TargetCompilation IR. Add unit, snapshot, property, invalid-input and determinism tests.

All planned M1 contract gaps are resolved: UUIDv7 typed IDs, the accepted recursive type/value surface, language and C/S/E audience TargetRule, deterministic IR, target-leak validation, and snapshot/permutation coverage are present and verified.

## M2 Persistence, Revisions, Auth and Audit - Completed

Add PostgreSQL migrations and SQLx repositories for projects, workspaces, schema/data revisions, row head/history, change sets, jobs, audit and outbox. Implement local Argon2id accounts, secure sessions, CSRF and project RBAC. Test empty-database migration, transaction rollback, optimistic conflicts, history, outbox idempotency and authorization boundaries.

Migrations 0002-0003, repository/API/auth layers and the worker implement the accepted M2 core: local Argon2id accounts, hashed bearer/CSRF, RBAC, projects/schema/rows, optimistic 409 conflicts, immutable schema/row/data revisions, audit/outbox, build artifacts and idempotent PostgreSQL projection processing.

## M3 Schema and Configuration UX - Completed

Implement /api/v1 contracts and OpenAPI for schema and data workflows. Build Vue schema/type/target designers and a VTable grid with server-side filtering/sorting, 512-row blocks, sparse caching, typed editors, batch paste, undo/redo and field-level conflict presentation. Automate API integration and browser E2E scenarios.

Bootstrap/login/project/schema/row/build/sync APIs and console flows work. Migration 0004 and the API provide bounded TableView blocks, safe server filters/sorts, expiry and data-revision snapshots. Multi-field schema design, typed row creation, independent C/S/E targeting, hard-reference checks, inline VTable editing, optimistic versions, block prefetch/cache deduplication and browser filter/sort acceptance are implemented and verified.

## M4 Formula and XLSX - Completed

Implement FieldId-based formula AST, dependency graph, cycle detection, native/WASM evaluation, computed fields and auditable bulk commands. Add XLSX template/export/import preview/diff/atomic commit with hidden stable metadata; read only cached Excel formula values and reject missing caches. Test round trips, renames, stale templates, cycles and rollback.

The datahub-formula and datahub-xlsx crates, migration 0005, PostgreSQL repositories, API and Vue console complete the planned M4 flow. Formula parsing binds display names to stable FieldIds, reports full dependency cycles and evaluates with Native/Wasmtime parity. XLSX hidden metadata preserves schema/revision/field/row/version identity, rejects foreign or stale workbooks and missing formula caches, and commits atomically with audit/outbox coverage. The free gate passed 26 Rust tests, 10 Web tests and Docker/PostgreSQL acceptance.

## M5 Deterministic Build and Export - Completed

Implement build orchestration pinned to schema/data revisions, targets and plugin versions. Add Rust, C# and TypeScript generation plus JSON, CSV, XML, BSON, Protobuf and Lua data output, hashes and manifests. Golden-test all formats, parse round trips, deterministic rebuilds, Protobuf wire-ID stability, and actual generated-code compilation.

Rust/C#/TypeScript plus JSON/CSV/XML/BSON/Protobuf/Lua artifacts, stable FieldId-derived Protobuf tags, timestamp-free revision/plugin/artifact manifests, repeatable-read build snapshots and immutable PostgreSQL persistence are implemented. The gate parses all codecs, checks exact manifests and artifact hashes, compares identical rebuilds and compiles generated Rust/C#/TypeScript. The final gate passed 29 Rust tests, 10 Web tests, migrations 0001-0006 and five builds with 45 artifacts.

## M6 Plugin Platform - Completed

Define plugin manifest, WIT/component interfaces, installation/version pinning and Wasmtime host limits. Restrict third-party plugins to declared read-only inputs and output directories with no credentials or network by default. Test path traversal, time, memory, fuel, output quotas, malformed packages and a compiling example plugin.

The WIT datahub-plugin world, strict hash-verified manifest/registry, safe path/capability validation and Wasmtime Component host are implemented. Guests receive only declared virtual inputs and one contained output, with no WASI ambient authority. Fuel, epoch timeout, memory, input and output bounds are verified using the compiling/componentized example. The final gate passed 33 Rust tests, 10 Web tests and all Docker/PostgreSQL regression checks.

## M7 PostgreSQL Sync, Release and Rollback - Completed, Pending Integration

Implement PostgreSQL projection planning, compatible DDL, approval for destructive changes, outbox consumption, retry/dead-letter/checkpoints and full resync. Add immutable artifacts/releases, environment policy, approval, publish and rollback. Test retry idempotency, checkpoint recovery, failed migration plans and historical release reproducibility.

Migration 0007, persistence/API/worker/UI changes and the local gate complete deterministic compatible/destructive projection plans, approval, bounded retry/dead-letter/checkpoints, full resync, environment policy and immutable release creation/approval/publish/rollback. The gate proved destructive 409/approval, poison-event isolation at five attempts, two-row resync while retaining dead letters, production approval, three immutable historical releases and restart persistence. M7 is integrated through PR #7 at a986c5d.

## M8 Hardening and Acceptance - Completed, Pending Integration

Complete audit search, rate limiting, secret handling, observability, backup/restore and runbooks. Exercise a full demo from schema creation through editing, formulas, XLSX, builds, releases and synchronization. Add large-table, concurrency, restart/persistence, security and recovery suites, then run the full local acceptance gate from a clean checkout and fresh Docker volumes.

Migration 0008 and the API/kernel/persistence/plugin/UI/operations changes implement project-scoped audit search, fixed-window limits, external secret-file validation and redaction, tracked-file secret scanning, correlation IDs, bounded metrics, Docker backup/restore and operator runbooks. The final gate exits 0 with 38 Rust tests, 10 Web tests, a 142-file secret scan, five images, migrations 0001-0008, a 1,024-row fixture/1,027 projected rows, 200/409 concurrency, adversarial plugin and recovery suites, matching fresh-volume restore fingerprints, post-restore writes and restart persistence. JSON request-span tracing and three negative secret-file tests pass. Quality containers/volumes are cleaned up. The feature and final v1 release still require integration.

## Automated Quality Gate

Provide one local command, backed by repository scripts and Docker Compose profiles, that runs Rust format/Clippy/tests, ESLint, Vue typecheck/Vitest/build, migration and Compose smoke checks, integration/E2E/golden/generated-code tests relevant to the milestone, and secret/dependency checks that do not require payment. Python helpers, if any, run only through uv. Record exact command outcomes at each milestone checkpoint.
