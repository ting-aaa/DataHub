<!-- PROJECT_MEMORY
{
  "blockers": [
    "GitHub CLI is unavailable in the current shell; use an authenticated GitHub path for PR and merge operations."
  ],
  "confidence": "high",
  "created_at": "2026-08-23T18:24:30Z",
  "derived_from": [],
  "event_id": "datahub-plan-m0-docker-foundation-v1",
  "id": "PLAN-20260823-686A1E",
  "kind": "plan",
  "next_actions": [
    "Commit and push feature/m0-foundation, open its PR to develop, verify CI, and squash-merge."
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
    "RPT-20260823-44414F"
  ],
  "status": "active",
  "summary": "M0 implementation and local verification are complete; only GitHub feature integration remains active.",
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
  "updated_at": "2026-08-23T18:44:19Z",
  "valid_as_of": "2026-08-24"
}
-->

# M0 Docker-first implementation plan

## Completed

- Repository policy files, Cargo/pnpm workspaces, Rust services/crates, Vue console, migration, and safe environment examples.
- Multi-stage non-root Dockerfiles and Compose topology for PostgreSQL, migrator, API, worker, plugin host, and web/Nginx.
- Local Rust and frontend checks, image/Compose startup, health endpoints, SQLx migration, and named-volume persistence verification.
- Public origin with main and develop bootstrap branches.

## Active

- Review and commit the feature work.
- Push feature/m0-foundation and open a PR to develop.
- Require GitHub Actions to pass, then squash-merge and confirm the remote develop state.

## Deferred Product Work

Schema, configuration editing, formula, Excel, generators, plugins, synchronization, and release capabilities remain later milestones and are not part of this M0 closure.
