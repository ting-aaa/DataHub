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
    "Integrate completed M3 and begin M4 formula/XLSX implementation."
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
    "RPT-20260823-49833C",
    "DEC-20260823-C69FFA",
    "TASK-20260823-2A43C1",
    "RPT-20260823-FE85CD",
    "RPT-20260823-118D95",
    "RPT-20260823-CA61E0",
    "RPT-20260823-E99D5D"
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
  "updated_at": "2026-08-23T20:11:04Z",
  "valid_as_of": "2026-08-24"
}
-->

# DataHub project overview

## Purpose

DataHub is a Docker-oriented Rust game configuration management and compilation platform with a Vue 3 web console and PostgreSQL as its canonical database.

## Current State

The public repository exists at https://github.com/ting-aaa/DataHub. M0-M3 are complete and the free local/Docker quality gate includes Rust, ESLint, Vue, four migrations and end-to-end persistence checks. M3 multi-field design, typed rows, inline VTable editing, block prefetch/cache, optimistic saves and server filtering/sorting passed browser acceptance. M4-M8 remain pending.

## Product Boundaries

The confirmed v1 direction includes local-account project RBAC, Rust/C#/TypeScript code generation, JSON/CSV/XML/BSON/Protobuf/Lua data output, and cached-value-only handling for imported Excel formulas.
