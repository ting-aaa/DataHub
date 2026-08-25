<!-- PROJECT_MEMORY
{
  "blockers": [],
  "confidence": "confirmed",
  "created_at": "2026-08-24T15:53:15Z",
  "derived_from": [
    "HANDOFF-20260824-89F499"
  ],
  "event_id": "datahub-handoff-v0-1-0-maintenance-baseline-v1",
  "id": "HANDOFF-20260824-4978F1",
  "kind": "handoff",
  "next_actions": [],
  "review_after": "2026-09-07",
  "schema_version": 1,
  "scope": [
    "."
  ],
  "sensitivity": "internal",
  "sources": [
    "RPT-20260824-976431",
    "PLAN-20260823-9B6D1E",
    "Curator Git and live runtime audit on 2026-08-24."
  ],
  "status": "superseded",
  "summary": "The v0.1.0 maintenance baseline is superseded by the completed v0.1.1 UI-label closeout handoff.",
  "supersedes": [
    "HANDOFF-20260824-89F499"
  ],
  "tags": [
    "closure",
    "handoff",
    "maintenance",
    "release",
    "v0.1.0"
  ],
  "task_id": "TASK-20260823-2A43C1",
  "tier": "short",
  "title": "DataHub v0.1.0 closure and maintenance baseline handoff",
  "type_version": 1,
  "updated_at": "2026-08-25T16:29:22Z",
  "valid_as_of": "2026-08-24"
}
-->

# DataHub v0.1.0 closure and maintenance baseline handoff

## Delivered State

DataHub v0.1.0 completes M0-M8 as a Docker-first Rust/Vue/PostgreSQL configuration management and compiler platform. M8 is integrated into develop at 16d328f, the release is merged to main at c97fbef, and main is reconciled back to develop at 6bbe5cc. The canonical free local quality gate passed from a fresh remote-develop clone and requires no paid external service.

## Verified Baseline

The final gate passed 38 Rust tests/doctests, 10 Web tests, formatting, Clippy, lint, typecheck, production build, five images, eight migrations, generated Rust/C#/TypeScript compilation, plugin/security/recovery suites, 1,024-row concurrency/performance acceptance, 1,027 projected rows and a 143-file tracked secret scan. Fresh-volume PostgreSQL restore, post-restore write and restart persistence passed and isolated resources were cleaned.

The retained-volume deployment exposes API readiness with version 0.1.0, Web and metrics HTTP 200, the expected services running, migration state 8|8, one project and two schema revisions. Repository and Docker builds use rsproxy-sparse/Rustup mirror configuration; the clean checkout visibly updated rsproxy-sparse.

## Recovery Starting Point

For future maintenance, begin from origin/develop at 6bbe5cc and retain the established feature/* -> develop squash and release/hotfix -> main merge-commit policy. Docker Compose, PostgreSQL SQLx migrations, scripts/quality-gate.ps1 and uv-only Python remain canonical. Release evidence is RPT-20260824-976431. No active implementation task or blocker remains from the v0.1.0 program.

## Caveat

No Git tag was present in the curator audit. The v0.1.0 identity is currently carried by the release branch/PR history, main merge commit and runtime version; no tag is claimed by project memory.
