<!-- PROJECT_MEMORY
{
  "blockers": [],
  "confidence": "confirmed",
  "created_at": "2026-08-23T19:42:47Z",
  "derived_from": [
    "RPT-20260823-FE85CD"
  ],
  "event_id": "datahub-report-m1-m2-m3-checkpoint-v1",
  "id": "RPT-20260823-118D95",
  "kind": "report",
  "next_actions": [],
  "review_after": "2026-09-06",
  "schema_version": 1,
  "scope": [
    "apps",
    "crates/datahub-auth",
    "crates/datahub-export",
    "crates/datahub-kernel",
    "crates/datahub-persistence-pg",
    "migrations",
    "web"
  ],
  "sensitivity": "internal",
  "sources": [
    "Main-agent implementation and verification delta for TASK-20260823-2A43C1 at index generation 26.",
    "Curator audit on 2026-08-24: source and migration search, cargo fmt/clippy/test, Vue typecheck/Vitest/build, git status/log, and scripts/quality-gate.ps1 inspection."
  ],
  "status": "superseded",
  "summary": "Historical M1/M2/M3 checkpoint superseded by RPT-20260823-CA61E0 after TableView read APIs and gate hardening were completed.",
  "supersedes": [
    "RPT-20260823-FE85CD"
  ],
  "tags": [
    "auth",
    "m1",
    "m2",
    "m3",
    "persistence",
    "verification",
    "web"
  ],
  "task_id": "TASK-20260823-2A43C1",
  "tier": "short",
  "title": "M1 closure, M2 completion, and M3 progress checkpoint",
  "type_version": 1,
  "updated_at": "2026-08-23T19:51:14Z",
  "valid_as_of": "2026-08-24"
}
-->

# M1 closure, M2 completion, and M3 progress checkpoint

## M1 Closure

The previously recorded M1 contract gaps are resolved. Every typed ID uses UUIDv7 through Uuid::now_v7. TypeAst now covers bool, integer, float, string, bytes, date, datetime, optional, list, fixed array, set, map, struct, enum, union, hard/soft references, and custom types. TargetRule separates output language from client/server/editor audience. Deterministic IR, hard-reference target-leak validation, JSON snapshot coverage, and field-permutation determinism tests are implemented.

## M2 Completed

Migration 0002 and the Rust API/persistence/auth layers implement Argon2id local accounts, hashed bearer and CSRF tokens, project RBAC, projects, schema/row persistence, optimistic conflict handling with HTTP 409, immutable schema/row revisions, audit events, and transactional outbox records. Migration 0003 adds aggregate data revisions, immutable build artifacts, and PostgreSQL projection tables.

The worker polls once per second, claims outbox work with FOR UPDATE SKIP LOCKED, and idempotently projects schema and row state. Reference values are checked against the target row before save.

## M3 In Progress

The Vue console and API client support bootstrap, login, project creation, schema/row flows, VTable display, builds, artifact downloads, and synchronization status. C/S/E audience selection is independent from Rust/C#/TypeScript output language. The UI type builder currently exposes integer, float, string, bool, enum, list, and hard-reference fields.

M3 is not complete: the block-oriented VTable data API, direct cell editing, sparse cache/conflict UX, and richer multi-field/type design surface remain.

## Early Later-Milestone Capabilities

datahub-export already generates deterministic SHA-256 artifacts for Rust, C#, TypeScript, JSON, and CSV. Build persistence and local PostgreSQL projection synchronization are present. These are useful partial M5/M7 capabilities, but XML, BSON, Protobuf, Lua, the complete build/release model, full sync planning/retry policy, approvals, publishing and rollback are not complete.

## Verification

The full scripts/quality-gate.ps1 run passed with Rust 16 and Web 7 tests at that checkpoint, migrations 1-3, revision/build/artifact/projection/audit/outbox counts, persistence restart, isolated Compose smoke and cleanup. After audience/reference changes, Rust format/Clippy/tests passed with 17 tests (kernel 10) and Web typecheck/7 tests/build passed. Curator reruns confirmed Clippy and all 17 Rust tests plus Web typecheck, 7 tests and production build.

A browser smoke run completed bootstrap, project, schema, row, Rust build and sync, ending with pending 0, failed 0, projected schema 1 and row 1.

## Remaining Scope

Finish M3 block APIs, editing and schema designer depth. M4 formula/XLSX, remaining M5 formats and deterministic build breadth, M6 Wasmtime plugins, M7 release/rollback and full synchronization semantics, and M8 hardening/acceptance remain pending.
