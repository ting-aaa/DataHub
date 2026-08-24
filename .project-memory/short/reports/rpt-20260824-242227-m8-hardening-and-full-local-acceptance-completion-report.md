<!-- PROJECT_MEMORY
{
  "blockers": [],
  "confidence": "confirmed",
  "created_at": "2026-08-24T15:13:28Z",
  "derived_from": [
    "RPT-20260823-9F7FDD"
  ],
  "event_id": "datahub-report-m8-hardening-acceptance-complete-v1",
  "id": "RPT-20260824-242227",
  "kind": "report",
  "next_actions": [],
  "review_after": "2026-09-07",
  "schema_version": 1,
  "scope": [
    "apps/datahub-api",
    "apps/datahub-plugin-host",
    "compose.yaml",
    "crates/datahub-kernel",
    "crates/datahub-persistence-pg",
    "docs/operations",
    "migrations/0008_hardening.sql",
    "scripts",
    "web"
  ],
  "sensitivity": "internal",
  "sources": [
    "Main-agent M8 completion delta and scripts/quality-gate.ps1 exit 0 on 2026-08-24.",
    "Curator repository audit: feature/m8-hardening-acceptance working tree based on a986c5d; migration/API/kernel/persistence/UI/operations/gate evidence inspected.",
    "Curator cargo test --workspace --all-features -- --test-threads=2: 35 tests and doctests passed."
  ],
  "status": "superseded",
  "summary": "M8 security, audit, observability, backup/restore and full local acceptance are complete on the feature branch; only GitHub integration and final release evidence remain.",
  "supersedes": [],
  "tags": [
    "acceptance",
    "backup",
    "hardening",
    "m8",
    "observability",
    "security"
  ],
  "task_id": "TASK-20260823-2A43C1",
  "tier": "short",
  "title": "M8 hardening and full local acceptance completion report",
  "type_version": 1,
  "updated_at": "2026-08-24T15:38:24Z",
  "valid_as_of": "2026-08-24"
}
-->

# M8 hardening and final local acceptance completion report

## Repository State

M7 was integrated through PR #7 as squash commit a986c5d; develop and origin/develop both resolve to that commit. M8 implementation and memory changes are complete in the uncommitted feature/m8-hardening-acceptance working tree based on a986c5d. M8 still requires commit, PR/squash integration and final release verification.

## Security and Audit

Migration 0008 adds hardening persistence. Project-scoped audit search supports deterministic reverse-chronological pagination and filters for actor, action, aggregate/entity, correlation identifier and time range, with RBAC preventing cross-project disclosure. Fixed-window authentication and mutation limits return deterministic 429/retry information, recover after the window and ignore spoofed client identity inputs. Required secrets can come from bounded external files or environment values; configuration failures redact values. A free tracked-file secret scan is part of the gate.

## Observability and Operations

Requests receive correlation/request IDs. Bounded metrics cover HTTP results and latency, database readiness, outbox/checkpoint/dead-letter state, releases and plugin runs/traps/quota rejections. The Vue console exposes audit search. Docker-first backup and restore scripts move a PostgreSQL custom-format dump into a fresh recovery volume, compare durable-state fingerprints and prove post-restore writes. Operator runbooks and explicit local acceptance budgets cover deployment, diagnosis, recovery, release operations and backup/restore.

## Full Local Acceptance

The final scripts/quality-gate.ps1 run exited 0 after Rust formatting, Clippy with warnings denied, 35 Rust tests and doctests, Web lint/typecheck/10 tests/build, WIT/plugin adversarial checks, five Docker images and migrations 0001-0008.

The gate exercised a 1,024-row fixture and observed 1,027 projected rows, concurrent mutation with one successful 200 response and one optimistic 409 conflict, dead-letter/recovery and release rollback, five deterministic builds with 45 artifacts, fresh-volume backup/restore with matching durable fingerprints and a successful post-restore write, plus restart persistence. Audit filters/pagination/correlation/RBAC, rate-limit 429/recovery/spoof handling, secret redaction/scan, request IDs and operational metrics all passed.

The curator independently reran the Rust command and confirmed 35 tests and doctests.

## Milestone and Task Status

M0-M8 are functionally complete and the full local acceptance gate passes. TASK-20260823-2A43C1 and PLAN-20260823-9B6D1E remain active until feature/m8-hardening-acceptance is committed, merged through GitHub under the free local GitFlow policy, branch cleanup is verified and the final integrated develop/release evidence is recorded. No paid dependency or service was introduced. No secret or sensitive data was encountered.
