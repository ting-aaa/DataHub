<!-- PROJECT_MEMORY
{
  "blockers": [],
  "confidence": "confirmed",
  "created_at": "2026-08-23T19:01:52Z",
  "derived_from": [
    "HANDOFF-20260823-D301EA"
  ],
  "event_id": "datahub-handoff-m1-to-m2-v1",
  "id": "HANDOFF-20260823-DE8966",
  "kind": "handoff",
  "next_actions": [],
  "review_after": "2026-09-06",
  "schema_version": 1,
  "scope": [
    "apps",
    "crates/datahub-kernel",
    "crates/datahub-persistence-pg",
    "migrations"
  ],
  "sensitivity": "internal",
  "sources": [
    "RPT-20260823-FE85CD",
    "PLAN-20260823-9B6D1E"
  ],
  "status": "superseded",
  "summary": "M0 transition is complete and the verified M1 kernel slice is ready for local integration; M2 persistence/auth is next with explicit M1 contract gaps retained.",
  "supersedes": [
    "HANDOFF-20260823-D301EA"
  ],
  "tags": [
    "auth",
    "handoff",
    "m1",
    "m2",
    "persistence"
  ],
  "task_id": "TASK-20260823-2A43C1",
  "tier": "short",
  "title": "M1 kernel to M2 persistence and auth handoff",
  "type_version": 1,
  "updated_at": "2026-08-23T19:43:06Z",
  "valid_as_of": "2026-08-24"
}
-->

# M1 kernel to M2 persistence and auth handoff

## Completed

M0 is on develop at squash commit 026cb6f. The payment-dependent GitHub workflow and required status contexts are removed. scripts/quality-gate.ps1 passed the complete isolated Rust/Vue/Docker/PostgreSQL local gate, and the old M0 feature branch is deleted.

The M1 kernel slice is implemented on feature/m1-domain-kernel and passes format, Clippy and all 7 workspace tests. It provides typed IDs, recursive schema/value models, canonicalization, validation diagnostics, and deterministic three-language Target IR.

## Working Tree

M1 Cargo manifest/lock changes and crates/datahub-kernel source additions are uncommitted on feature/m1-domain-kernel. No secrets were encountered.

## Exact Next Actions

1. Resolve or explicitly schedule the M1 contract gaps recorded in RPT-20260823-FE85CD, especially UUIDv7 versus current UUIDv4.
2. Run scripts/quality-gate.ps1, commit M1, and integrate it into develop under the free local policy.
3. Create the M2 feature branch from updated develop.
4. Implement PostgreSQL domain persistence and migrations for projects, schemas, rows, revisions, audit and outbox.
5. Add local Argon2id accounts, sessions, CSRF and project RBAC with transaction, migration, conflict, idempotency and authorization tests.

## Acceptance Caveat

The current M1 slice does not satisfy every item in PLAN-20260823-9B6D1E: UUID generation is v4 rather than v7, and several planned type/target/reference policies remain absent. They must remain visible until implemented or explicitly superseded by a user decision.
