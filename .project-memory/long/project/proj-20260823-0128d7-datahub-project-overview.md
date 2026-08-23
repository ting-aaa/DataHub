<!-- PROJECT_MEMORY
{
  "blockers": [],
  "confidence": "confirmed",
  "created_at": "2026-08-23T18:23:32Z",
  "derived_from": [],
  "event_id": "datahub-bootstrap-project-overview-v1",
  "id": "PROJ-20260823-0128D7",
  "kind": "project",
  "next_actions": [
    "After the user clears the GitHub billing lock, rerun PR #1 checks and verify automatic squash merge into develop."
  ],
  "review_after": "",
  "schema_version": 1,
  "scope": [
    "."
  ],
  "sensitivity": "internal",
  "sources": [
    "User instructions in the DataHub planning conversation on 2026-08-24.",
    "Repository audit on 2026-08-24: Get-ChildItem -Force returned only .project-memory and git status reported no Git repository.",
    "RPT-20260823-44414F",
    "RPT-20260823-49833C"
  ],
  "status": "active",
  "summary": "DataHub is a Docker-oriented Rust game configuration management and compilation platform with a Vue web console and PostgreSQL as its canonical database.",
  "supersedes": [],
  "tags": [
    "datahub",
    "product"
  ],
  "task_id": "",
  "tier": "long",
  "title": "DataHub project overview",
  "type_version": 1,
  "updated_at": "2026-08-23T18:48:21Z",
  "valid_as_of": "2026-08-24"
}
-->

# DataHub project overview

## Purpose

DataHub is a Docker-oriented Rust game configuration management and compilation platform with a Vue 3 web console and PostgreSQL as its canonical database.

## Current State

The public repository exists at https://github.com/ting-aaa/DataHub. M0 commit 66d7b8e is pushed and PR #1 is open to develop with protected automatic squash merge enabled. Local validation passed, but GitHub did not start the required jobs because the user account is billing-locked; the merge remains pending.

## Product Boundaries

The confirmed v1 direction includes local-account project RBAC, Rust/C#/TypeScript code generation, JSON/CSV/XML/BSON/Protobuf/Lua data output, and cached-value-only handling for imported Excel formulas.
