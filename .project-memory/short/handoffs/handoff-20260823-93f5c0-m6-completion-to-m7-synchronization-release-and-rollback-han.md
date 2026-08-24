<!-- PROJECT_MEMORY
{
  "blockers": [],
  "confidence": "confirmed",
  "created_at": "2026-08-23T21:23:17Z",
  "derived_from": [
    "HANDOFF-20260823-F2BBAB"
  ],
  "event_id": "datahub-handoff-m6-to-m7-v1",
  "id": "HANDOFF-20260823-93F5C0",
  "kind": "handoff",
  "next_actions": [],
  "review_after": "2026-09-06",
  "schema_version": 1,
  "scope": [
    "apps/datahub-api",
    "apps/datahub-worker",
    "crates/datahub-persistence-pg",
    "migrations",
    "scripts",
    "tests",
    "web"
  ],
  "sensitivity": "internal",
  "sources": [
    "RPT-20260823-1A4BFC",
    "PLAN-20260823-9B6D1E",
    "Git evidence: clean feature/m7-release-sync at 636a131."
  ],
  "status": "superseded",
  "summary": "M6 is merged and fully verified; complete deterministic PostgreSQL projection planning, reliable outbox recovery and immutable release approval/publish/rollback on the clean M7 branch.",
  "supersedes": [
    "HANDOFF-20260823-F2BBAB"
  ],
  "tags": [
    "handoff",
    "m7",
    "postgresql",
    "release",
    "rollback",
    "sync"
  ],
  "task_id": "TASK-20260823-2A43C1",
  "tier": "short",
  "title": "M6 completion to M7 synchronization release and rollback handoff",
  "type_version": 1,
  "updated_at": "2026-08-23T21:42:36Z",
  "valid_as_of": "2026-08-24"
}
-->

# M6 completion to M7 synchronization, release and rollback handoff

## Completed Baseline

M0-M6 are complete on develop at squash commit 636a131. M6 provides a versioned WIT Component contract, immutable exact-version plugin registry and deny-by-default Wasmtime sandbox. The free local gate passes 33 Rust tests, 10 Web tests, plugin quota/timeout acceptance, migrations 0001-0006, five healthy images, deterministic build acceptance, projection convergence and restart persistence.

## Repository State

feature/m6-plugin-sandbox and its remote branch are deleted after PR #6. develop equals origin/develop at 636a131. A clean feature/m7-release-sync branch already exists from that exact commit. The current worker already performs idempotent schema/row projection from the PostgreSQL outbox; this is an M7 foundation only, not migration planning, checkpoint recovery, dead-letter handling, release approval or rollback.

## M7 Objective

Complete PostgreSQL projection planning and reliable outbox delivery, then layer immutable releases, environment policy, approvals, publishing and rollback over revision-pinned M5 artifacts and M6 plugin versions.

## Required Work

1. Add migration-managed persistence for projection targets/plans, checkpoints, attempts, retry scheduling and dead-letter state, plus immutable releases, environments, approvals and publication history.
2. Compare desired schemas with the target PostgreSQL catalog and generate deterministic compatible DDL plans pinned to source revisions and target state.
3. Classify destructive or incompatible operations, require explicit authorized approval before execution and preserve the approved plan hash.
4. Extend outbox processing with bounded retries, backoff, idempotency keys, checkpoints, inspectable failures and dead-letter recovery without losing ordering guarantees.
5. Implement resumable full resynchronization that rebuilds projections and advances checkpoints only after durable success.
6. Create immutable releases that pin schema/data revisions, target/audience, artifact hashes and plugin versions; enforce environment policy and project RBAC for approval and publish actions.
7. Implement rollback as a new auditable publication of a historical immutable release, without mutating or deleting prior releases/artifacts.
8. Expose API and Vue workflows for plan review, destructive approval, sync/retry/dead-letter status, release creation, approval, publish and rollback.
9. Automate retry idempotency, checkpoint recovery, failed/changed/destructive plan handling, full resync, authorization, historical release reproducibility, publish and rollback acceptance through the free local Docker gate.

## Pending Scope

M7-M8 remain pending. Existing projection tables/worker and immutable build artifacts are foundations only. M8 hardening, backup/restore, observability and final clean-checkout acceptance remain out of M7 except for contracts needed to expose M7 operational state.

## Constraints and Evidence

PostgreSQL remains canonical and schema changes require committed SQLx migrations. Preserve non-root/read-only independently health-checked images, the local free quality gate and uv-only Python. Never depend on paid remote checks. Relevant records are TASK-20260823-2A43C1, PLAN-20260823-9B6D1E, DEC-20260823-C69FFA, STD-20260823-048D0D and RPT-20260823-1A4BFC. No sensitive data was encountered.
