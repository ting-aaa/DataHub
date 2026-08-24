<!-- PROJECT_MEMORY
{
  "blockers": [],
  "confidence": "confirmed",
  "created_at": "2026-08-24T15:13:53Z",
  "derived_from": [
    "HANDOFF-20260823-CAFFC2"
  ],
  "event_id": "datahub-handoff-m8-to-final-release-v1",
  "id": "HANDOFF-20260824-89F499",
  "kind": "handoff",
  "next_actions": [],
  "review_after": "2026-09-07",
  "schema_version": 1,
  "scope": [
    "."
  ],
  "sensitivity": "internal",
  "sources": [
    "RPT-20260824-61086A",
    "PLAN-20260823-9B6D1E",
    "Curator Git audit: M8 complete in uncommitted feature/m8-hardening-acceptance working tree based on integrated M7 commit a986c5d.",
    "RPT-20260824-976431 and HANDOFF-20260824-4978F1."
  ],
  "status": "superseded",
  "summary": "The pending M8/release integration handoff is resolved and superseded by the completed v0.1.0 maintenance baseline handoff.",
  "supersedes": [
    "HANDOFF-20260823-CAFFC2"
  ],
  "tags": [
    "github",
    "handoff",
    "integration",
    "m8",
    "release",
    "v1"
  ],
  "task_id": "TASK-20260823-2A43C1",
  "tier": "short",
  "title": "M8 completion to final GitHub integration and v1 release handoff",
  "type_version": 1,
  "updated_at": "2026-08-24T15:53:24Z",
  "valid_as_of": "2026-08-24"
}
-->

# M8 completion to final GitHub integration and v1 release handoff

## Completed Baseline

M0-M8 are functionally complete. The final canonical free local quality gate exits 0 with 38 Rust tests, 10 Web tests, a 142-file tracked secret scan, plugin adversarial checks, five images, migrations 0001-0008, deterministic builds, projection/release recovery, 1,024-row/concurrency budgets, security/observability checks and fresh-volume backup/restore. JSON tracing carries request-ID spans. The gate cleaned up all isolated containers and volumes.

Repository Cargo configuration pins crates.io replacement to rsproxy-sparse. The Rust Docker builder passes rsproxy Rustup defaults and copies that Cargo configuration because host mirror variables do not enter Docker automatically. An isolated API image proof began Rustup downloads in about six seconds, used rsproxy-sparse for Cargo dependencies and completed the release workspace image in two minutes sixteen seconds.

## Repository State

Resolved. M8 PR #8 is integrated at 16d328f, release PR #9 is merged to main at c97fbef, and reconciliation PR #10 is merged to develop at 6bbe5cc. Final evidence is recorded in RPT-20260824-976431 and HANDOFF-20260824-4978F1.

## Exact Next Actions

None for the v0.1.0 implementation program. Future maintenance resumes from HANDOFF-20260824-4978F1.

## Completion Guard

Do not close the task merely because the feature implementation or local gate is complete. Completion requires the M8 feature squash on develop, release integration on main, branch/tag verification and final integrated runtime evidence. Any failure keeps the task active with the failing item recorded.

## Constraints and Evidence

Docker Compose and PostgreSQL remain canonical. Keep SQLx migrations, non-root/read-only independently health-checked images, uv-only Python and the free local gate. Never place credential values in Git or memory. Relevant records are TASK-20260823-2A43C1, PLAN-20260823-9B6D1E, DEC-20260823-C69FFA, STD-20260823-048D0D, RPT-20260824-61086A and HANDOFF-20260823-CAFFC2. No sensitive data was encountered.
