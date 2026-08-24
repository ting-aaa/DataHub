<!-- PROJECT_MEMORY
{
  "blockers": [],
  "confidence": "confirmed",
  "created_at": "2026-08-23T20:42:40Z",
  "derived_from": [
    "RPT-20260823-E99D5D"
  ],
  "event_id": "datahub-report-m4-formula-xlsx-complete-v1",
  "id": "RPT-20260823-38DC17",
  "kind": "report",
  "next_actions": [
    "Integrate M4 and begin M5 deterministic build/export acceptance."
  ],
  "review_after": "2026-09-06",
  "schema_version": 1,
  "scope": [
    "apps/datahub-api",
    "crates/datahub-formula",
    "crates/datahub-persistence-pg",
    "crates/datahub-xlsx",
    "migrations/0005_formulas_xlsx.sql",
    "scripts/quality-gate.ps1",
    "web"
  ],
  "sensitivity": "internal",
  "sources": [
    "Main-agent completion delta for feature/m4-formula-xlsx on 2026-08-24.",
    "Curator repository audit and cargo test --workspace --all-features -- --test-threads=2: 26 tests and doctests passed."
  ],
  "status": "completed",
  "summary": "M4 FieldId formulas, Native/Wasmtime parity, cached-value-only XLSX round trips and atomic PostgreSQL commits are implemented and fully verified.",
  "supersedes": [],
  "tags": [
    "formula",
    "m4",
    "postgresql",
    "wasmtime",
    "xlsx"
  ],
  "task_id": "TASK-20260823-2A43C1",
  "tier": "short",
  "title": "M4 formula and XLSX completion report",
  "type_version": 1,
  "updated_at": "2026-08-23T20:42:40Z",
  "valid_as_of": "2026-08-24"
}
-->

# M4 formula and XLSX completion report

## Formula Engine

The new datahub-formula crate implements a stable FieldId-based AST and parser, dependency extraction, topological evaluation and full cycle paths. Parsed formulas remain valid when display names change. Native evaluation and the Wasmtime WAT runtime produce identical results for the tested expression surface.

## XLSX Round Trips

The datahub-xlsx crate exports hidden schema, schema revision, field, row and version identities. Preview and commit preserve stable identities, reject foreign schemas and stale revisions, read Excel formula cells only through cached values and reject formula cells without caches. Empty existing rows retain their identity.

## Persistence, API and Console

Migration 0005 adds immutable formula sets and revisions. PostgreSQL repositories persist formula revisions and provide save_rows_atomic so any optimistic conflict rolls back the full batch while successful rows each produce audit and outbox records. The API exposes formula GET/PUT/preview/apply and XLSX export/preview/commit. The Vue console provides formula editing/runtime selection and XLSX export, preview and commit flows.

## Verification

The complete free local quality gate passed Rust formatting, Clippy with warnings denied, 26 workspace tests and doctests; Web lint, typecheck, 10 tests and production build; five independently built Docker images and healthy services; migrations 0001-0005; auth, RBAC, schema, row, TableView, deterministic build, outbox and reference checks; Native/Wasm formula parity and apply; XLSX stable identity, preview and atomic rollback; and restart persistence.

The 26 Rust tests are API 1, auth 3, export 3, formula 4, kernel 10 and XLSX 5. A curator rerun of the exact quality-gate Rust test command passed all 26 tests and zero-test doctest suites. An earlier direct rerun transiently reached 26 passing tests before an API doctest artifact lookup failed; the immediate exact-command rerun passed.

## Resolved Gate Failure

The first complete M4 gate attempt failed only because the XLSX rollback scenario added a second row while the older TableView total assertion still expected one. Updating the expected total to two aligned the assertion with the expanded fixture, after which the full gate passed. No product failure or paid dependency remains.

## Milestone Status

M0-M4 are complete. M5-M8 remain pending, so TASK-20260823-2A43C1 stays active. No sensitive data was encountered.
