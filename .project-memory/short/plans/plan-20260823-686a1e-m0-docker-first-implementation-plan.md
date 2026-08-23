<!-- PROJECT_MEMORY
{
  "blockers": [
    "External GitHub billing/account lock prevents required Actions jobs from starting."
  ],
  "confidence": "high",
  "created_at": "2026-08-23T18:24:30Z",
  "derived_from": [],
  "event_id": "datahub-plan-m0-docker-foundation-v1",
  "id": "PLAN-20260823-686A1E",
  "kind": "plan",
  "next_actions": [
    "User clears the billing lock; rerun required checks; allow auto squash merge; verify develop and feature-branch deletion."
  ],
  "review_after": "2026-09-06",
  "schema_version": 1,
  "scope": [
    "."
  ],
  "sensitivity": "internal",
  "sources": [
    "Accepted DataHub development plans and Docker deployment instruction from the user on 2026-08-24.",
    "TASK-20260823-9C0927",
    "RPT-20260823-44414F",
    "RPT-20260823-49833C"
  ],
  "status": "active",
  "summary": "M0 is pushed and PR #1 is configured for protected auto squash merge; completion waits on the external GitHub billing lock and CI rerun.",
  "supersedes": [],
  "tags": [
    "docker",
    "m0",
    "plan"
  ],
  "task_id": "TASK-20260823-9C0927",
  "tier": "short",
  "title": "M0 Docker-first implementation plan",
  "type_version": 1,
  "updated_at": "2026-08-23T18:48:21Z",
  "valid_as_of": "2026-08-24"
}
-->

# M0 Docker-first implementation plan

## Completed

- Repository policy files, Cargo/pnpm workspaces, Rust services/crates, Vue console, migration, and safe environment examples.
- Multi-stage non-root Dockerfiles and Compose topology for PostgreSQL, migrator, API, worker, plugin host, and web/Nginx.
- Local Rust and frontend checks, image/Compose startup, health endpoints, SQLx migration, and named-volume persistence verification.
- Public origin with main and develop bootstrap branches.
- Commit 66d7b8e pushed to feature/m0-foundation; PR #1 opened to develop with auto squash merge enabled.
- Repository merge settings and main/develop protection rules configured for strict required checks.

## Active

- User resolves the GitHub billing/account lock.
- Rerun Rust checks, Web checks, and Docker smoke for PR #1.
- Let automatic squash merge complete; confirm origin/develop and deletion of origin/feature/m0-foundation.

## Deferred Product Work

Schema, configuration editing, formula, Excel, generators, plugins, synchronization, and release capabilities remain later milestones and are not part of this M0 closure.
