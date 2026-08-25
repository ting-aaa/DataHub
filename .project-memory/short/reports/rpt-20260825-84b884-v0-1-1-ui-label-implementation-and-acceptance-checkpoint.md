<!-- PROJECT_MEMORY
{
  "blockers": [],
  "confidence": "confirmed",
  "created_at": "2026-08-25T16:26:28Z",
  "derived_from": [
    "TASK-20260825-69F856",
    "PLAN-20260825-10A55E"
  ],
  "event_id": "datahub-report-v0-1-1-ui-label-acceptance-v1",
  "id": "RPT-20260825-84B884",
  "kind": "report",
  "next_actions": [],
  "review_after": "2026-09-08",
  "schema_version": 1,
  "scope": [
    "web/src/App.vue",
    "web/src/services/display-labels.spec.ts",
    "web/src/services/display-labels.ts"
  ],
  "sensitivity": "internal",
  "sources": [
    "Main-agent implementation, browser and full quality-gate memory_delta on 2026-08-26.",
    "Curator repository audit of feature/ui-label-polish based on develop 3d6fea5.",
    "Curator pnpm test rerun: 5 files and 12 tests passed."
  ],
  "status": "superseded",
  "summary": "Formula and legacy-build labels are fixed and fully accepted locally; only feature-to-develop squash integration remains.",
  "supersedes": [],
  "tags": [
    "acceptance",
    "maintenance",
    "ui",
    "v0.1.1"
  ],
  "task_id": "TASK-20260825-69F856",
  "tier": "short",
  "title": "v0.1.1 UI label implementation and acceptance checkpoint",
  "type_version": 1,
  "updated_at": "2026-08-25T16:29:22Z",
  "valid_as_of": "2026-08-26"
}
-->

# v0.1.1 UI label implementation and acceptance checkpoint

## Implementation

feature/ui-label-polish was created from clean develop commit 3d6fea54ddefb34b64b73dbc103a1aee053f989f. web/src/services/display-labels.ts centralizes the two presentation contracts and App.vue now uses those helpers.

A null formulaVersion renders `FieldId AST · 未保存`; numeric versions retain `FieldId AST · formula vN`. A BuildRecord with null input_hash renders `target · 历史构建`; a present hash retains the target plus its first eight characters. No backend contract, persisted historical row or release behavior changed.

web/src/services/display-labels.spec.ts covers unsaved and numbered formula states plus null and present build hashes. The curator inspected the implementation and independently reran Vitest: five files and 12 tests passed.

## Focused and Browser Acceptance

Local Web lint, typecheck, 12 tests and production build exited 0. Docker Compose rebuilt Web, API and migrator while preserving the PostgreSQL volume. After refreshing the existing authenticated browser session and waiting for asynchronous loading, the page reported formulaUnsaved=true, legacyBuild=true, hasVnew=false and hasUndefined=false. Browser console warnings and errors were empty.

## Canonical Quality Gate

scripts/quality-gate.ps1 exited 0. It passed Rust formatting, Clippy with warnings denied, 38 Rust tests/doctests, 12 Web tests, a 149-file tracked secret scan, Wasmtime adversarial checks, five healthy images, SQLx migrations 0001-0008, auth/RBAC/rate/formula/XLSX/build acceptance, generated Rust/C#/TypeScript compilation, the 1,024-row and 1,027-projection scenarios, fresh-volume backup/restore/write/restart and cleanup of isolated quality resources.

## Build Observations

The first Docker rebuild pulled newer layers because node:24-bookworm-slim and debian:bookworm-slim are floating tags, making it slower despite all Rust build layers hitting cache. The official npm registry returned ECONNRESET once and the package manager retry succeeded. rsproxy had no error. Pinning base-image digests may improve future cache predictability, but it is a non-blocking optimization outside this task.

## Integration State

Implementation and acceptance were complete at this checkpoint. RPT-20260825-E793AF supersedes it with PR #20 integration, branch cleanup and task closure evidence. No sensitive data was encountered.
