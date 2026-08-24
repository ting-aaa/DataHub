<!-- PROJECT_MEMORY
{
  "blockers": [],
  "confidence": "confirmed",
  "created_at": "2026-08-24T15:53:03Z",
  "derived_from": [
    "RPT-20260824-61086A"
  ],
  "event_id": "datahub-report-v0-1-0-final-release-v1",
  "id": "RPT-20260824-976431",
  "kind": "report",
  "next_actions": [],
  "review_after": "2026-09-07",
  "schema_version": 1,
  "scope": [
    "."
  ],
  "sensitivity": "internal",
  "sources": [
    "User-supplied final integration and clean-clone quality-gate evidence on 2026-08-24.",
    "Curator Git audit: PR commit graph 16d328f -> c97fbef -> 6bbe5cc; origin/main and origin/develop match.",
    "Curator live Compose/API/Web/metrics/PostgreSQL audit on 2026-08-24."
  ],
  "status": "completed",
  "summary": "DataHub v0.1.0 is integrated through M8, release and reconciliation PRs and passes clean-clone quality and retained-volume Docker/PostgreSQL acceptance.",
  "supersedes": [
    "RPT-20260824-61086A"
  ],
  "tags": [
    "acceptance",
    "docker",
    "gitflow",
    "release",
    "v0.1.0"
  ],
  "task_id": "TASK-20260823-2A43C1",
  "tier": "short",
  "title": "DataHub v0.1.0 release and final acceptance report",
  "type_version": 1,
  "updated_at": "2026-08-24T15:53:03Z",
  "valid_as_of": "2026-08-24"
}
-->

# DataHub v0.1.0 release and final acceptance report

## GitFlow Integration

M8 PR #8 was squash-merged into develop at 16d328f72f9b8a50fb52fc81d8605fd65bedd204. Release PR #9 merged release/v0.1.0 into main with merge commit c97fbef320b7450080a9be7fd0705f6cdd68d2a7. Reconciliation PR #10 merged main back into develop at 6bbe5cc74cccc517b621f0a2f4d697714a5fb585. Curator Git audit confirmed the commits and parent relationships; origin/main and origin/develop resolve to the expected release and reconciliation commits.

## Clean-checkout Acceptance

A fresh clone from remote develop ran scripts/quality-gate.ps1 with exit 0. The gate passed Rust formatting, Clippy with warnings denied, 38 Rust tests and doctests, Web lint/typecheck/10 tests/build, five independently built images, SQLx migrations 0001-0008, generated Rust/C#/TypeScript compilation, audit/RBAC/rate-limit checks, the 1,024-row concurrency/performance fixture and 1,027 projected rows. The tracked-file secret scan covered 143 files. PostgreSQL backup, restore into a fresh volume, post-restore write and restart persistence all passed, and quality resources were removed afterward.

The clean build explicitly reported Updating rsproxy-sparse index, confirming the repository Cargo source replacement and Docker Rustup/Cargo mirror configuration operate outside the original working tree. No paid external service is required by the canonical gate or deployment.

## Integrated Runtime Evidence

On release/v0.1.0-finalize at develop reconciliation commit 6bbe5cc, the preserved PostgreSQL volume and rebuilt Compose stack were inspected. API, PostgreSQL, plugin host and Web health checks were healthy; the worker was running. GET /health/ready returned status ok and version 0.1.0, Web returned HTTP 200, and /metrics returned HTTP 200. PostgreSQL reported migration version/count 8|8, one datahub_projects row and two datahub_schema_revisions rows. This confirms the integrated image, migration and retained-data startup path.

## Release Identity

The release is identified by release/v0.1.0 history, PR #9, main merge commit c97fbef and the runtime version 0.1.0. Curator audit found no Git tag, so this report does not claim that a v0.1.0 tag exists.

## Closure

M0-M8, free local automation, GitFlow integration and final Docker/PostgreSQL runtime acceptance are complete. TASK-20260823-2A43C1 and PLAN-20260823-9B6D1E may close. No blocker, paid mandatory dependency, credential, secret value or sensitive data was encountered.
