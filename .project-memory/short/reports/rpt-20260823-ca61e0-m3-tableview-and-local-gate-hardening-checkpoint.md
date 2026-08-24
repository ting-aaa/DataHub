<!-- PROJECT_MEMORY
{
  "blockers": [],
  "confidence": "confirmed",
  "created_at": "2026-08-23T19:50:56Z",
  "derived_from": [
    "RPT-20260823-118D95"
  ],
  "event_id": "datahub-report-m3-tableview-quality-gate-v1",
  "id": "RPT-20260823-CA61E0",
  "kind": "report",
  "next_actions": [],
  "review_after": "2026-09-06",
  "schema_version": 1,
  "scope": [
    "apps/datahub-api",
    "crates/datahub-persistence-pg",
    "deploy/docker/rust.Dockerfile",
    "migrations/0004_table_views.sql",
    "scripts/quality-gate.ps1",
    "web"
  ],
  "sensitivity": "internal",
  "sources": [
    "Main-agent M3 implementation and final quality-gate delta for TASK-20260823-2A43C1.",
    "Curator audit on 2026-08-24: migration/API/UI/tooling source inspection, cargo fmt/clippy/test, web lint/typecheck/Vitest/build, git status and docker compose ps."
  ],
  "status": "superseded",
  "summary": "Historical M3 checkpoint superseded by RPT-20260823-E99D5D after inline editing, prefetch/cache and multi-field design were completed.",
  "supersedes": [
    "RPT-20260823-118D95"
  ],
  "tags": [
    "eslint",
    "local-ci",
    "m3",
    "tableview",
    "vtable"
  ],
  "task_id": "TASK-20260823-2A43C1",
  "tier": "short",
  "title": "M3 TableView and local gate hardening checkpoint",
  "type_version": 1,
  "updated_at": "2026-08-23T20:11:04Z",
  "valid_as_of": "2026-08-24"
}
-->

# M3 TableView and local gate hardening checkpoint

## TableView Backend

Migration 0004 creates persisted table views with block sizes constrained to 256-1024, stored filter/sort specifications, optional data revision, one-hour expiry and an expiry index. The API creates views, counts rows, records the latest data revision and returns blocks by index. Filters and values use SQLx QueryBuilder bindings; sorting is constructed from validated field IDs and direction rather than accepting raw SQL.

## Schema and Editing Contracts

C/S/E audience targeting remains independent from Rust/C#/TypeScript output language. The Vue schema builder exposes integer, float, string, bool, inline enum, list and hard-reference fields plus audience selection. Hard-reference values are rejected before save when the target row does not exist.

The browser can restore existing development data after a non-destructive stack upgrade, display a TableView containing one row and complete an optimistic row update successfully.

## Free Local Gate and Build Tooling

The web workspace uses ESLint 10 flat configuration. Web lint is part of scripts/quality-gate.ps1 before typecheck, Vitest and production build. The Rust Docker builder compiles all workspace binaries once in a locked release build, allowing the independently runnable service images to reuse that build instead of rebuilding per binary.

## Verification

The final free local quality gate passed Rust format, Clippy with warnings denied and 17 tests; Web ESLint, typecheck, 7 tests and production build; all four SQLx migrations; authentication, RBAC, optimistic conflict, hard-reference validation, C/S audience isolation, deterministic builds, outbox processing and restart persistence. The isolated quality stack cleaned up successfully.

The development stack upgraded without deleting its data. Browser verification restored the existing data set, returned one TableView row and completed an optimistic update. Sync remained healthy in the preceding checkpoint.

## Remaining M3 Scope

M3 remains active. Inline VTable cell editing, lazy block prefetch/cache behavior and a richer multi-field schema designer are not complete. M4 formula/XLSX and M5-M8 remain pending; the task must not be marked complete.
