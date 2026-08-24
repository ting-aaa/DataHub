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
  "next_actions": [],
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
    "RPT-20260824-61086A.",
    "RPT-20260824-976431 and HANDOFF-20260824-4978F1."
  ],
  "status": "completed",
  "summary": "DataHub v0.1.0 M0-M8, free automated acceptance, GitFlow release integration and retained-volume Docker/PostgreSQL verification are complete.",
  "supersedes": [],
  "tags": [
    "automated-testing",
    "completed",
    "docker",
    "local-ci",
    "v1"
  ],
  "task_id": "",
  "tier": "short",
  "title": "Implement complete DataHub v1 with free automated gates",
  "type_version": 1,
  "updated_at": "2026-08-24T15:53:24Z",
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

M0-M8 are complete. M8 PR #8 is integrated into develop at 16d328f, release PR #9 is merged to main at c97fbef, and reconciliation PR #10 is merged back to develop at 6bbe5cc. A fresh remote-develop clone passed the complete free local gate, and the retained-volume Compose deployment passed final API/Web/metrics/migration/data checks.

## Completion Criteria

Satisfied by RPT-20260824-976431: migrations succeed, generated code compiles, deterministic exports and failure modes are tested, backup/restore and restart preserve durable state, and the final clean-clone gate passes without paid infrastructure.
