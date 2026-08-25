<!-- PROJECT_MEMORY
{
  "blockers": [],
  "confidence": "confirmed",
  "created_at": "2026-08-25T16:29:04Z",
  "derived_from": [
    "RPT-20260825-84B884"
  ],
  "event_id": "datahub-report-v0-1-1-ui-label-final-v1",
  "id": "RPT-20260825-E793AF",
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
    "RPT-20260825-84B884 implementation and acceptance checkpoint.",
    "GitHub PR #20 merged at 2026-08-25T16:28:04Z as 86e03a6597667b006a330b900857472fed395495.",
    "Curator Git/GitHub audit: develop and origin/develop synchronized; original local and remote feature branches absent; closeout worktree clean."
  ],
  "status": "completed",
  "summary": "The formula and legacy-build label fixes passed full acceptance and were squash-integrated through PR #20 at 86e03a6.",
  "supersedes": [
    "RPT-20260825-84B884"
  ],
  "tags": [
    "completion",
    "gitflow",
    "maintenance",
    "ui",
    "v0.1.1"
  ],
  "task_id": "TASK-20260825-69F856",
  "tier": "short",
  "title": "v0.1.1 UI label maintenance completion report",
  "type_version": 1,
  "updated_at": "2026-08-25T16:29:04Z",
  "valid_as_of": "2026-08-26"
}
-->

# v0.1.1 UI label maintenance completion report

## Delivered Fix

DataHub now renders an unsaved formula as `FieldId AST · 未保存` while preserving `FieldId AST · formula vN` for saved revisions. Historical builds with null input_hash render `target · 历史构建`; builds with hashes retain the first eight characters. App.vue consumes the shared display-label helpers, and focused tests cover null and normal cases without changing API or persisted-history contracts.

## Acceptance

Focused Web lint, typecheck, 12 tests and build passed. Real-browser acceptance on the preserved Docker/PostgreSQL stack confirmed the unsaved and legacy labels, absence of `vnew` and `undefined`, and no console warnings/errors. The full scripts/quality-gate.ps1 passed Rust fmt/Clippy/38 tests and doctests, 12 Web tests, a 149-file secret scan, Wasmtime adversarial checks, five images, eight migrations, auth/RBAC/rate/formula/XLSX/build and generated-code checks, the 1,024-row/1,027-projection scenario, fresh-volume restore/write/restart, and cleanup.

## GitFlow Integration

PR #20 was squash-merged from feature/ui-label-polish into develop at 2026-08-25T16:28:04Z as commit 86e03a6597667b006a330b900857472fed395495. Curator audit confirmed the PR state and merge commit. develop, origin/develop and the closeout branch all resolve to 86e03a6; origin/feature/ui-label-polish and the original local feature branch are absent. The worktree was clean before memory closeout.

## Non-blocking Follow-up

Floating node:24-bookworm-slim and debian:bookworm-slim tags can invalidate Docker layer caches when upstream tags move. Pinning approved digests may improve predictable rebuild time. The official npm registry may occasionally reset connections; the observed ECONNRESET recovered automatically through retry. Neither observation blocked acceptance or warrants expanding this completed maintenance task.

## Closure

TASK-20260825-69F856 and PLAN-20260825-10A55E are complete. No paid required service, blocker, credential exposure or sensitive data was encountered.
