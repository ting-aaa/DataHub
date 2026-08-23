<!-- PROJECT_MEMORY
{
  "blockers": [],
  "confidence": "confirmed",
  "created_at": "2026-08-23T19:51:08Z",
  "derived_from": [
    "HANDOFF-20260823-5A07F4"
  ],
  "event_id": "datahub-handoff-m3-tableview-to-editing-v1",
  "id": "HANDOFF-20260823-CB0534",
  "kind": "handoff",
  "next_actions": [
    "Complete inline editing, conflicts, lazy prefetch/cache and richer schema design with automated acceptance."
  ],
  "review_after": "2026-09-06",
  "schema_version": 1,
  "scope": [
    "apps/datahub-api",
    "crates/datahub-persistence-pg",
    "migrations/0004_table_views.sql",
    "web"
  ],
  "sensitivity": "internal",
  "sources": [
    "RPT-20260823-CA61E0",
    "PLAN-20260823-9B6D1E"
  ],
  "status": "active",
  "summary": "M3 TableView reads and quality hardening are verified; inline cell editing, lazy prefetch/cache and richer multi-field schema design remain before M4.",
  "supersedes": [
    "HANDOFF-20260823-5A07F4"
  ],
  "tags": [
    "editing",
    "handoff",
    "m3",
    "tableview",
    "vtable"
  ],
  "task_id": "TASK-20260823-2A43C1",
  "tier": "short",
  "title": "M3 TableView read path to inline editing handoff",
  "type_version": 1,
  "updated_at": "2026-08-23T19:51:08Z",
  "valid_as_of": "2026-08-24"
}
-->

# M3 TableView read path to inline editing handoff

## Completed Since Prior Handoff

TableView persistence and APIs now support bounded 256-1024-row blocks, safe server filters/sorts, expiry and data-revision snapshots. The Vue console consumes TableView data, separates C/S/E audience from output language, builds the currently supported field kinds and validates hard references before save. ESLint is mandatory in the local gate and the Rust Docker build compiles workspace binaries once in release mode.

## Current State

M0-M2 remain complete. M3 has functional TableView read/display and optimistic API update behavior, but is not complete. The working tree remains on feature/m1-domain-kernel with accumulated M1-M3 changes and project-memory checkpoints not yet integrated into develop.

## Exact Next Actions

1. Add direct inline VTable cell editors for supported scalar/reference types.
2. Submit edits with expected row versions and present HTTP 409 base/user/server conflict details.
3. Implement lazy adjacent-block prefetch, bounded sparse cache, invalidation by data revision and expired-view recreation.
4. Add multi-field schema editing and broaden the designer while preserving C/S/E and reference contracts.
5. Automate cell edit, conflict, prefetch/cache, expiry and RBAC scenarios; rerun the complete free local gate and browser acceptance.
6. Integrate M3 only when those criteria pass, then start M4 formula/XLSX.

## Pending Scope

M4-M8 remain pending. Existing code generation, JSON/CSV artifacts and projection sync are foundations only and do not change later milestone status.

## Verification Snapshot

The latest full gate passed Rust 17 tests, Web 7 tests plus lint/typecheck/build, four migrations and the complete database/API/restart suite. Browser state recovery, one-row TableView display and optimistic update succeeded. No credentials or other sensitive data were recorded.
