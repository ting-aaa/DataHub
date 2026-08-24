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
  "next_actions": [
    "Commit and squash-integrate M8, then complete and verify the GitFlow v1 release before closing the task."
  ],
  "review_after": "2026-09-07",
  "schema_version": 1,
  "scope": [
    "."
  ],
  "sensitivity": "internal",
  "sources": [
    "RPT-20260824-61086A",
    "PLAN-20260823-9B6D1E",
    "Curator Git audit: M8 complete in uncommitted feature/m8-hardening-acceptance working tree based on integrated M7 commit a986c5d."
  ],
  "status": "active",
  "summary": "All v1 functionality and local acceptance pass; commit/squash M8 into develop, complete the GitFlow release to main and record final integrated evidence before closing the task.",
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
  "updated_at": "2026-08-24T15:38:24Z",
  "valid_as_of": "2026-08-24"
}
-->

# M8 completion to final GitHub integration and v1 release handoff

## Completed Baseline

M0-M8 are functionally complete. The final canonical free local quality gate exits 0 with 38 Rust tests, 10 Web tests, a 142-file tracked secret scan, plugin adversarial checks, five images, migrations 0001-0008, deterministic builds, projection/release recovery, 1,024-row/concurrency budgets, security/observability checks and fresh-volume backup/restore. JSON tracing carries request-ID spans. The gate cleaned up all isolated containers and volumes.

Repository Cargo configuration pins crates.io replacement to rsproxy-sparse. The Rust Docker builder passes rsproxy Rustup defaults and copies that Cargo configuration because host mirror variables do not enter Docker automatically. An isolated API image proof began Rustup downloads in about six seconds, used rsproxy-sparse for Cargo dependencies and completed the release workspace image in two minutes sixteen seconds.

## Repository State

M7 is integrated on develop/origin/develop at a986c5d through PR #7. M8 is complete in the uncommitted feature/m8-hardening-acceptance working tree based on that commit. The overall task and plan must remain active until M8 and the v1 release are integrated and verified.

## Exact Next Actions

1. Commit all M8 product, documentation and project-memory changes on feature/m8-hardening-acceptance.
2. If the commit changes any generated or gate-sensitive content, rerun scripts/quality-gate.ps1 and require exit 0; otherwise preserve the recorded full-gate evidence.
3. Push the feature branch, open a PR to develop and squash-merge under the free local gate policy. Do not add paid required checks or billing-dependent auto-merge.
4. Verify local develop equals origin/develop at the M8 squash commit and confirm the local/remote feature branch is deleted.
5. Confirm the intended v1 release version/tag, create the GitFlow release/* branch from integrated develop, update only necessary release/version notes and rerun the canonical gate from the release commit.
6. Open the release PR to main and merge it with a merge commit, preserving the established release/hotfix policy. Verify main and the release tag point to the accepted release history; reconcile develop if release-only metadata differs.
7. From the integrated/release state, verify Docker Compose startup, migrations 0001-0008, API/Web health, durable-state restart and the documented backup/restore command. Record exact commit, PR, tag and verification evidence.
8. Only after those checks pass, mark TASK-20260823-2A43C1 and PLAN-20260823-9B6D1E completed, supersede this handoff and create the final v1 release/maintenance handoff.

## Completion Guard

Do not close the task merely because the feature implementation or local gate is complete. Completion requires the M8 feature squash on develop, release integration on main, branch/tag verification and final integrated runtime evidence. Any failure keeps the task active with the failing item recorded.

## Constraints and Evidence

Docker Compose and PostgreSQL remain canonical. Keep SQLx migrations, non-root/read-only independently health-checked images, uv-only Python and the free local gate. Never place credential values in Git or memory. Relevant records are TASK-20260823-2A43C1, PLAN-20260823-9B6D1E, DEC-20260823-C69FFA, STD-20260823-048D0D, RPT-20260824-61086A and HANDOFF-20260823-CAFFC2. No sensitive data was encountered.
