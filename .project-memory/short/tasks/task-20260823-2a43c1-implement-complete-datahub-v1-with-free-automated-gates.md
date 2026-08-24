<!-- PROJECT_MEMORY
{
  "blockers": [],
  "confidence": "confirmed",
  "created_at": "2026-08-23T18:52:55Z",
  "derived_from": [
    "TASK-20260823-9C0927"
  ],
  "event_id": "datahub-task-full-v1-local-gates-v1",
  "id": "TASK-20260823-2A43C1",
  "kind": "task",
  "next_actions": [
    "Commit and squash-integrate feature/m8-hardening-acceptance into develop under the free local gate policy.",
    "Complete the GitFlow v1 release PR to main with a merge commit and verify the release tag/history.",
    "Record final integrated Compose, migration, health, restart and backup/restore evidence before closing the task."
  ],
  "review_after": "2026-09-06",
  "schema_version": 1,
  "scope": [
    "."
  ],
  "sensitivity": "internal",
  "sources": [
    "Explicit user instruction on 2026-08-24 to cancel paid workflows, change the plan, fully implement DataHub, and automate tests.",
    "PROJ-20260823-0128D7, ARCH-20260823-F3A201, DEC-20260823-A72203, and DEC-20260823-C69FFA.",
    "RPT-20260823-FE85CD and HANDOFF-20260823-DE8966.",
    "RPT-20260823-118D95 and HANDOFF-20260823-5A07F4.",
    "RPT-20260823-CA61E0 and HANDOFF-20260823-CB0534.",
    "RPT-20260823-E99D5D and HANDOFF-20260823-88CF74.",
    "RPT-20260823-38DC17 and HANDOFF-20260823-E37206.",
    "RPT-20260823-EE0875 and HANDOFF-20260823-F2BBAB.",
    "RPT-20260823-1A4BFC and HANDOFF-20260823-93F5C0.",
    "RPT-20260823-9F7FDD and HANDOFF-20260823-CAFFC2.",
    "RPT-20260824-242227 and HANDOFF-20260824-89F499.",
    "RPT-20260824-61086A."
  ],
  "status": "active",
  "summary": "M0-M8 are functionally complete and the full local gate passes; the active full-v1 task now awaits M8 GitHub integration, v1 release integration and final integrated evidence.",
  "supersedes": [],
  "tags": [
    "active",
    "automated-testing",
    "docker",
    "local-ci",
    "v1"
  ],
  "task_id": "",
  "tier": "short",
  "title": "Implement complete DataHub v1 with free automated gates",
  "type_version": 1,
  "updated_at": "2026-08-24T15:38:24Z",
  "valid_as_of": "2026-08-24"
}
-->

# Implement complete DataHub v1 with free automated gates

## Objective

Fully implement DataHub v1 as the Docker-first Rust/Vue/PostgreSQL game configuration management and compiler platform defined by the accepted product and architecture decisions. Automate verification with free local and Docker-based quality gates; do not require paid services.

## Included Capabilities

Complete the typed schema/configuration domain, revisions and auditing, local accounts and project RBAC, high-performance VTable editing, formula engine, XLSX round trips, deterministic builds, Rust/C#/TypeScript generators, JSON/CSV/XML/BSON/Protobuf/Lua outputs, Wasmtime plugin isolation, PostgreSQL synchronization, releases, approvals, rollback, observability, and operational documentation.

## Delivery Method

Transition the existing M0 branch away from required GitHub checks and billing-dependent auto-merge, integrate it under the verified local gate, then deliver M1-M8 in bounded feature branches. Each milestone must include implementation, migrations/contracts where applicable, automated tests, Docker smoke or integration verification, documentation, and evidence before local GitFlow integration.

## Progress

M0-M8 are functionally complete. M1 covers UUIDv7, the accepted type system, TargetRule, deterministic IR and target safety. M2 covers PostgreSQL domain persistence, immutable revisions, audit/outbox, local accounts, token/CSRF handling and project RBAC. M3 completes multi-field schema design, typed row creation, inline VTable editing, bounded block loading/prefetch/cache, optimistic saves, server filtering/sorting and browser acceptance. M4 completes stable FieldId formulas, dependency/cycle diagnostics, Native/Wasmtime parity, cached-value-only XLSX round trips and atomic PostgreSQL application. M5 completes deterministic revision-pinned manifests and the complete built-in output matrix. M6 completes versioned WIT Components and deny-by-default Wasmtime isolation. M7 completes projection recovery and immutable release rollback and is integrated at a986c5d. M8 completes audit/rate-limit/secret/metrics hardening, backup/restore, operational docs and full local acceptance. M8 and the final v1 release still require Git integration.

## Completion Criteria

Every planned v1 capability works through a full demo project; migrations succeed from an empty PostgreSQL volume; generated code compiles; exported data is deterministic; plugin and synchronization failure modes are tested; the full stack restarts with durable state; and the final local acceptance gate passes without paid infrastructure.
