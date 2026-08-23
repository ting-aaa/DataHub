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
    "Replace paid GitHub status/auto-merge requirements and integrate the verified M0 baseline under the local gate.",
    "Start M1 domain-kernel implementation with automated unit and property tests."
  ],
  "review_after": "2026-09-06",
  "schema_version": 1,
  "scope": [
    "."
  ],
  "sensitivity": "internal",
  "sources": [
    "Explicit user instruction on 2026-08-24 to cancel paid workflows, change the plan, fully implement DataHub, and automate tests.",
    "PROJ-20260823-0128D7, ARCH-20260823-F3A201, DEC-20260823-A72203, and DEC-20260823-C69FFA."
  ],
  "status": "active",
  "summary": "Implement and verify the full DataHub v1 product through Docker-first milestone delivery and free local automated quality gates.",
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
  "updated_at": "2026-08-23T18:52:55Z",
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

## Completion Criteria

Every planned v1 capability works through a full demo project; migrations succeed from an empty PostgreSQL volume; generated code compiles; exported data is deterministic; plugin and synchronization failure modes are tested; the full stack restarts with durable state; and the final local acceptance gate passes without paid infrastructure.
