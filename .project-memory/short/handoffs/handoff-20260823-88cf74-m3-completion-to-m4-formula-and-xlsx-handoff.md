<!-- PROJECT_MEMORY
{
  "blockers": [],
  "confidence": "confirmed",
  "created_at": "2026-08-23T20:10:58Z",
  "derived_from": [
    "HANDOFF-20260823-CB0534"
  ],
  "event_id": "datahub-handoff-m3-to-m4-v1",
  "id": "HANDOFF-20260823-88CF74",
  "kind": "handoff",
  "next_actions": [
    "Integrate M3, create the M4 branch and implement formula/XLSX acceptance with automated tests."
  ],
  "review_after": "2026-09-06",
  "schema_version": 1,
  "scope": [
    "apps",
    "crates",
    "migrations",
    "tests",
    "web"
  ],
  "sensitivity": "internal",
  "sources": [
    "RPT-20260823-E99D5D",
    "PLAN-20260823-9B6D1E"
  ],
  "status": "active",
  "summary": "M3 is fully verified; integrate its branch, then implement M4 FieldId formulas, Native/WASM parity and cached-value-only XLSX round trips.",
  "supersedes": [
    "HANDOFF-20260823-CB0534"
  ],
  "tags": [
    "formula",
    "handoff",
    "m4",
    "xlsx"
  ],
  "task_id": "TASK-20260823-2A43C1",
  "tier": "short",
  "title": "M3 completion to M4 formula and XLSX handoff",
  "type_version": 1,
  "updated_at": "2026-08-23T20:10:58Z",
  "valid_as_of": "2026-08-24"
}
-->

# M3 completion to M4 formula and XLSX handoff

## Completed

M3 is complete on feature/m3-grid-editing. Multi-field schema design, typed row creation, inline VTable editing, 256-row TableView blocks, prefetch/cache deduplication, optimistic version saves, server exact filtering/sorting and browser acceptance all passed. The free local gate passes Rust 17 and Web 10 tests plus Docker/PostgreSQL end-to-end checks.

## Repository State

The M3 working tree is not yet integrated into develop. M0-M2 are already on develop at 8398d3f. Project-memory updates remain part of the active working tree. No sensitive data was encountered.

## Exact Next Actions

1. Commit feature/m3-grid-editing, run the final local gate if the diff changes and squash-integrate M3 into develop under the free local policy.
2. Create the M4 feature branch from updated develop.
3. Implement a stable FieldId-based formula AST, parser, dependency graph and cycle diagnostics.
4. Provide equivalent Native and WASM evaluation plus computed fields and auditable bulk formula commands.
5. Implement XLSX template/export and import preview/diff/atomic commit with hidden schema/row identity metadata.
6. Treat Excel formulas as cached values only; reject imports whose formula cells lack cached results.
7. Automate formula semantics, cycles, Native/WASM parity, XLSX round trips, renames, stale templates, missing caches and transaction rollback.

## Pending Scope

M4-M8 remain pending. Completing M3 does not complete the active full-v1 task.
