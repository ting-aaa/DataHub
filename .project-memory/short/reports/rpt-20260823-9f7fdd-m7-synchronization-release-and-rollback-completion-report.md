<!-- PROJECT_MEMORY
{
  "blockers": [],
  "confidence": "confirmed",
  "created_at": "2026-08-23T21:42:01Z",
  "derived_from": [
    "RPT-20260823-1A4BFC"
  ],
  "event_id": "datahub-report-m7-release-sync-complete-v1",
  "id": "RPT-20260823-9F7FDD",
  "kind": "report",
  "next_actions": [
    "Commit and integrate M7, then execute M8 hardening and final acceptance."
  ],
  "review_after": "2026-09-06",
  "schema_version": 1,
  "scope": [
    "apps/datahub-api",
    "apps/datahub-worker",
    "crates/datahub-persistence-pg",
    "migrations/0007_release_sync.sql",
    "scripts/quality-gate.ps1",
    "web"
  ],
  "sensitivity": "internal",
  "sources": [
    "Main-agent M7 completion delta and full local quality-gate result on 2026-08-24.",
    "Curator repository audit: feature/m7-release-sync working tree based on 636a131; migration/API/repository/UI/gate evidence inspected.",
    "Curator cargo test --workspace --all-features -- --test-threads=2: 35 tests and doctests passed."
  ],
  "status": "completed",
  "summary": "M7 deterministic projection planning/recovery and immutable release approval/publish/rollback are implemented and fully verified on the feature branch, pending integration.",
  "supersedes": [],
  "tags": [
    "m7",
    "postgresql",
    "release",
    "rollback",
    "sync"
  ],
  "task_id": "TASK-20260823-2A43C1",
  "tier": "short",
  "title": "M7 synchronization release and rollback completion report",
  "type_version": 1,
  "updated_at": "2026-08-23T21:42:01Z",
  "valid_as_of": "2026-08-24"
}
-->

# M7 synchronization release and rollback completion report

## Repository State

M7 functionality is complete and verified on feature/m7-release-sync, but it is not yet committed or merged. HEAD, develop and origin/develop remain at the M6 squash commit 636a131. Product, documentation, migration 0007 and project-memory changes are present in the working tree.

## Projection Planning and Recovery

Migration 0007 adds projection plans, retry/checkpoint/dead-letter state, environments and immutable releases. Projection plans are deterministic and stable-ID based. Compatible plans apply directly; destructive removal or retyping is blocked until explicit approval. Outbox processing isolates poison events, retries to a fixed limit, retains dead letters, exposes checkpoint state and supports full resynchronization without discarding failures.

## Release Lifecycle

Environments carry an explicit approval policy and a current-release pointer. Releases snapshot a successful deterministic build, input_hash and manifest. Production publication is blocked until approval. Publishing atomically advances the environment pointer while preserving prior immutable releases. Rollback creates and publishes a new immutable release copied from a historical snapshot rather than mutating history.

## API and Console

The API and Vue console expose projection plan creation/review/approval/application, sync/dead-letter/full-resync state, environment creation, release creation/approval/publication and rollback workflows with project RBAC and optimistic conflict responses.

## Automated Acceptance

The final free local quality gate passed Rust formatting, Clippy with warnings denied, 35 Rust tests and doctests, Web lint/typecheck/10 tests/build, WIT plugin adversarial checks, five Docker images, migrations 0001-0007, and the M5 five-build/45-artifact matrix.

M7 acceptance created two projection plans: one compatible plan applied, while one destructive plan returned 409 until approved and then applied. A poison schema.saved event without an aggregate was isolated and dead-lettered at exactly five attempts. Checkpoint/full resync restored two rows and returned ready while retaining the dead letter. Production publish returned 409 until approval; two publishes plus rollback produced three immutable releases, preserved historical input_hash values and advanced the current environment pointer. PostgreSQL volume restart persistence passed.

The curator independently reran the Rust command and confirmed 35 tests, including two projection-planning unit tests.

## Resolved Gate Failure

An earlier full gate exposed an incorrect project association in full_resync: canonical rows were not joined through their schema/project owner. Adding the canonical row-to-schema project join fixed resynchronization; the complete gate then passed.

## Milestone Status

M0-M7 are functionally complete. M8 remains pending, so TASK-20260823-2A43C1 stays active. M7 still requires commit, PR/squash integration and branch deletion before M8 begins. The local Docker gate remains canonical. No paid dependency, secret or sensitive data was encountered.
