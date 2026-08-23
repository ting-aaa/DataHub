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
    "Integrate completed M3, then implement and verify M4 formula and XLSX capabilities."
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
    "RPT-20260823-E99D5D"
  ],
  "status": "active",
  "summary": "M0-M3 are complete; M4-M8 remain pending under the free automated local gate.",
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
  "updated_at": "2026-08-23T20:11:04Z",
  "valid_as_of": "2026-08-24"
}
-->

# DataHub v1 milestone plan with free local automation

## Progress

- M0 transition: completed on develop at squash commit 026cb6f; local quality gate passed.
- M1 domain/kernel: completed, including resolved UUIDv7/type/target/testing gaps.
- M2 persistence/auth: completed through migrations 0002-0003, revisions, audit/outbox, auth/RBAC, builds and projection foundations.
- M3 schema/configuration UX: completed with multi-field design, typed rows, inline VTable editing, block prefetch/cache, optimistic saves, filter/sort and browser acceptance.
- M4-M8: pending; partial export/build/projection foundations do not complete M5 or M7.

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

## M4 Formula and XLSX - Pending

Implement FieldId-based formula AST, dependency graph, cycle detection, native/WASM evaluation, computed fields and auditable bulk commands. Add XLSX template/export/import preview/diff/atomic commit with hidden stable metadata; read only cached Excel formula values and reject missing caches. Test round trips, renames, stale templates, cycles and rollback.

## M5 Deterministic Build and Export - Pending

Implement build orchestration pinned to schema/data revisions, targets and plugin versions. Add Rust, C# and TypeScript generation plus JSON, CSV, XML, BSON, Protobuf and Lua data output, hashes and manifests. Golden-test all formats, parse round trips, deterministic rebuilds, Protobuf wire-ID stability, and actual generated-code compilation.

Rust/C#/TypeScript plus JSON/CSV deterministic SHA-256 artifacts are an implemented foundation; XML/BSON/Protobuf/Lua and the full M5 acceptance matrix remain.

## M6 Plugin Platform - Pending

Define plugin manifest, WIT/component interfaces, installation/version pinning and Wasmtime host limits. Restrict third-party plugins to declared read-only inputs and output directories with no credentials or network by default. Test path traversal, time, memory, fuel, output quotas, malformed packages and a compiling example plugin.

## M7 PostgreSQL Sync, Release and Rollback - Pending

Implement PostgreSQL projection planning, compatible DDL, approval for destructive changes, outbox consumption, retry/dead-letter/checkpoints and full resync. Add immutable artifacts/releases, environment policy, approval, publish and rollback. Test retry idempotency, checkpoint recovery, failed migration plans and historical release reproducibility.

The local idempotent schema/row projection worker is an implemented foundation; migration planning, retry/dead-letter/checkpoints, approvals, publishing, releases and rollback remain.

## M8 Hardening and Acceptance - Pending

Complete audit search, rate limiting, secret handling, observability, backup/restore and runbooks. Exercise a full demo from schema creation through editing, formulas, XLSX, builds, releases and synchronization. Add large-table, concurrency, restart/persistence, security and recovery suites, then run the full local acceptance gate from a clean checkout and fresh Docker volumes.

## Automated Quality Gate

Provide one local command, backed by repository scripts and Docker Compose profiles, that runs Rust format/Clippy/tests, ESLint, Vue typecheck/Vitest/build, migration and Compose smoke checks, integration/E2E/golden/generated-code tests relevant to the milestone, and secret/dependency checks that do not require payment. Python helpers, if any, run only through uv. Record exact command outcomes at each milestone checkpoint.
