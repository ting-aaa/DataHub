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
    "Commit and squash-integrate the verified feature/m7-release-sync work into develop.",
    "Implement M8 audit search, rate limiting, secret redaction, observability, backup/restore and operator runbooks.",
    "Pass the complete v1 demo and clean-checkout/fresh-volume performance, concurrency, security and recovery acceptance."
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
    "RPT-20260823-9F7FDD and HANDOFF-20260823-CAFFC2."
  ],
  "status": "active",
  "summary": "M0-M7 are functionally complete and verified, with M7 pending integration; the active full-v1 task advances to the remaining M8 hardening and final acceptance work.",
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
  "updated_at": "2026-08-23T21:42:36Z",
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

M0-M7 are functionally complete. M1 covers UUIDv7, the accepted type system, TargetRule, deterministic IR and target safety. M2 covers PostgreSQL domain persistence, immutable revisions, audit/outbox, local accounts, token/CSRF handling and project RBAC. M3 completes multi-field schema design, typed row creation, inline VTable editing, bounded block loading/prefetch/cache, optimistic saves, server filtering/sorting and browser acceptance. M4 completes stable FieldId formulas, dependency/cycle diagnostics, Native/Wasmtime parity, cached-value-only XLSX round trips and atomic PostgreSQL application. M5 completes deterministic revision-pinned manifests, the Rust/C#/TypeScript and JSON/CSV/XML/BSON/Protobuf/Lua matrix, stable Protobuf tags and generated-code compilation. M6 completes versioned WIT Components, immutable plugin installation and deny-by-default Wasmtime isolation. M7 completes projection plans, retry/dead-letter/checkpoint/full-resync recovery and immutable release approval/publish/rollback. M7 still requires Git integration; M8 remains pending.

## Completion Criteria

Every planned v1 capability works through a full demo project; migrations succeed from an empty PostgreSQL volume; generated code compiles; exported data is deterministic; plugin and synchronization failure modes are tested; the full stack restarts with durable state; and the final local acceptance gate passes without paid infrastructure.
