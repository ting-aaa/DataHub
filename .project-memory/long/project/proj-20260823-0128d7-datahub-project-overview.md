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
    "Integrate verified M7 and complete M8 hardening, backup/recovery and full clean-environment acceptance."
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
    "RPT-20260823-E99D5D",
    "RPT-20260823-38DC17",
    "RPT-20260823-EE0875",
    "RPT-20260823-1A4BFC",
    "RPT-20260823-9F7FDD"
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
  "updated_at": "2026-08-23T21:42:36Z",
  "valid_as_of": "2026-08-24"
}
-->

# DataHub project overview

## Purpose

DataHub is a Docker-oriented Rust game configuration management and compilation platform with a Vue 3 web console and PostgreSQL as its canonical database.

## Current State

The public repository exists at https://github.com/ting-aaa/DataHub. M0-M7 are functionally complete; M7 is verified in the uncommitted feature/m7-release-sync working tree based on develop 636a131. The free local/Docker gate passes 35 Rust tests, 10 Web tests, seven migrations, five images, plugin adversarial checks, projection recovery, immutable release rollback and restart persistence. M8 hardening and final acceptance remain pending.

## Product Boundaries

The confirmed v1 direction includes local-account project RBAC, Rust/C#/TypeScript code generation, JSON/CSV/XML/BSON/Protobuf/Lua data output, and cached-value-only handling for imported Excel formulas.
