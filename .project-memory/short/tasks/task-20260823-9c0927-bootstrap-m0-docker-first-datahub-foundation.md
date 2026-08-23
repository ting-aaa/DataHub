<!-- PROJECT_MEMORY
{
  "blockers": [
    "GitHub CLI is not installed in the current shell; PR and repository-policy automation require installation or another authenticated GitHub path."
  ],
  "confidence": "confirmed",
  "created_at": "2026-08-23T18:24:19Z",
  "derived_from": [],
  "event_id": "datahub-task-m0-foundation-v1",
  "id": "TASK-20260823-9C0927",
  "kind": "task",
  "next_actions": [
    "Review, commit, and push the M0 files on feature/m0-foundation.",
    "Open the pull request to develop and verify GitHub Actions.",
    "Squash-merge after CI passes and verify origin/develop."
  ],
  "review_after": "2026-09-06",
  "schema_version": 1,
  "scope": [
    "."
  ],
  "sensitivity": "internal",
  "sources": [
    "Current task assigned by the user and main agent on 2026-08-24.",
    "Environment audit commands on 2026-08-24: docker version, docker compose version, tool version checks, git status, and port 5432 inspection.",
    "RPT-20260823-44414F",
    "HANDOFF-20260823-D2DC12"
  ],
  "status": "active",
  "summary": "The Docker-first M0 foundation is implemented and locally verified; GitHub PR, CI, and squash merge into develop remain.",
  "supersedes": [],
  "tags": [
    "active",
    "docker",
    "m0"
  ],
  "task_id": "",
  "tier": "short",
  "title": "Bootstrap M0 Docker-first DataHub foundation",
  "type_version": 1,
  "updated_at": "2026-08-23T18:44:19Z",
  "valid_as_of": "2026-08-24"
}
-->

# Bootstrap M0 Docker-first DataHub foundation

## Objective

Deliver the verified Docker-first Rust/Vue/PostgreSQL M0 foundation through the approved GitFlow process.

## Completed

The public remote, main/develop branches, Rust and Vue workspaces, PostgreSQL migration, Docker images, Compose stack, health endpoints, and GitHub Actions workflow exist. Local Rust, frontend, Compose, HTTP, migration, and persistence verification passed.

## Current Phase

The implementation is still uncommitted on feature/m0-foundation. The task remains active until the feature is pushed, GitHub CI passes, and the PR is squash-merged into develop.
