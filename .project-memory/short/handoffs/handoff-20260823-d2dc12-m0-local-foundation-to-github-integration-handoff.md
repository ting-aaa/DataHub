<!-- PROJECT_MEMORY
{
  "blockers": [
    "GitHub CLI is unavailable in the current shell."
  ],
  "confidence": "confirmed",
  "created_at": "2026-08-23T18:44:32Z",
  "derived_from": [
    "HANDOFF-20260823-323477",
    "PLAN-20260823-686A1E"
  ],
  "event_id": "datahub-handoff-m0-local-to-github-v1",
  "id": "HANDOFF-20260823-D2DC12",
  "kind": "handoff",
  "next_actions": [
    "Commit and push feature/m0-foundation.",
    "Open the PR, verify CI, and squash-merge into develop."
  ],
  "review_after": "2026-09-06",
  "schema_version": 1,
  "scope": [
    "."
  ],
  "sensitivity": "internal",
  "sources": [
    "RPT-20260823-44414F",
    "PIT-20260823-C6E3B6"
  ],
  "status": "active",
  "summary": "Local M0 implementation and verification are complete; commit, push, PR, CI, and squash merge into develop remain.",
  "supersedes": [
    "HANDOFF-20260823-323477"
  ],
  "tags": [
    "github",
    "handoff",
    "m0"
  ],
  "task_id": "TASK-20260823-9C0927",
  "tier": "short",
  "title": "M0 local foundation to GitHub integration handoff",
  "type_version": 1,
  "updated_at": "2026-08-23T18:44:32Z",
  "valid_as_of": "2026-08-24"
}
-->

# M0 local foundation to GitHub integration handoff

## Completed

The public GitHub remote exists and main/develop are reachable. The Docker-first M0 source tree is implemented on feature/m0-foundation. Local Rust, Vue, Docker Compose, HTTP, migration, and persistence verification passed. The stack remains running for follow-up checks.

## Current State

All M0 files are still uncommitted on feature/m0-foundation. Local main and develop track origin at bootstrap commit 3811dcc. Five long-running Compose services are up, the one-shot migration service exited successfully, and SQLx migration version 1 is applied.

## Exact Next Actions

1. Review the uncommitted diff, commit it on feature/m0-foundation, and push the branch.
2. Open a pull request from feature/m0-foundation to develop.
3. Observe required GitHub Actions checks and resolve any remote-only failures.
4. Squash-merge into develop after CI passes, then verify origin/develop and the local branch state.
5. Send final Git/CI/merge evidence to the memory curator so TASK-20260823-9C0927 and PLAN-20260823-686A1E can be completed.

## Caveats

GitHub CLI is not installed in this shell, so PR and branch-policy work needs CLI installation or another authenticated GitHub route. The Vite production build currently emits a large-chunk warning. Keep the running Docker stack in mind before changing ports or volumes. No credentials were stored in project memory.
