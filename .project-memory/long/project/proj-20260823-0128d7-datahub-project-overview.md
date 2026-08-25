<!-- PROJECT_MEMORY
{
  "blockers": [],
  "confidence": "confirmed",
  "created_at": "2026-08-23T18:23:32Z",
  "derived_from": [],
  "event_id": "datahub-bootstrap-project-overview-v1",
  "id": "PROJ-20260823-0128D7",
  "kind": "project",
  "next_actions": [],
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
    "RPT-20260823-E99D5D",
    "RPT-20260823-38DC17",
    "RPT-20260823-EE0875",
    "RPT-20260823-1A4BFC",
    "RPT-20260823-9F7FDD",
    "RPT-20260824-242227",
    "RPT-20260824-61086A",
    "RPT-20260824-976431",
    "HANDOFF-20260824-4978F1",
    "RPT-20260825-E793AF",
    "HANDOFF-20260825-DBC1D1"
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
  "updated_at": "2026-08-25T16:29:22Z",
  "valid_as_of": "2026-08-24"
}
-->

# DataHub project overview

## Purpose

DataHub is a Docker-oriented Rust game configuration management and compilation platform with a Vue 3 web console and PostgreSQL as its canonical database.

## Current State

The public repository exists at https://github.com/ting-aaa/DataHub. DataHub v0.1.0 completes M0-M8. The v0.1.1 UI-label maintenance fixes for unsaved formulas and legacy null-hash builds are integrated into develop through PR #20 at 86e03a6 with 12 Web tests and the full Docker-first gate passing. The established baseline includes 38 Rust tests, eight migrations, five images, generated-code compilation, security/observability and 1,024-row/concurrency checks, fresh-volume backup/restore and restart persistence.

## Product Boundaries

The confirmed v1 direction includes local-account project RBAC, Rust/C#/TypeScript code generation, JSON/CSV/XML/BSON/Protobuf/Lua data output, and cached-value-only handling for imported Excel formulas.
