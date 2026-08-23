<!-- PROJECT_MEMORY
{
  "blockers": [],
  "confidence": "confirmed",
  "created_at": "2026-08-23T20:42:52Z",
  "derived_from": [
    "HANDOFF-20260823-88CF74"
  ],
  "event_id": "datahub-handoff-m4-to-m5-v1",
  "id": "HANDOFF-20260823-E37206",
  "kind": "handoff",
  "next_actions": [
    "Integrate M4, create the M5 feature branch and complete deterministic build/export acceptance."
  ],
  "review_after": "2026-09-06",
  "schema_version": 1,
  "scope": [
    "apps",
    "crates",
    "migrations",
    "scripts",
    "tests",
    "web"
  ],
  "sensitivity": "internal",
  "sources": [
    "RPT-20260823-38DC17",
    "PLAN-20260823-9B6D1E"
  ],
  "status": "active",
  "summary": "M4 is fully verified; integrate its branch, then complete deterministic revision-pinned builds and the full built-in code/data export matrix in M5.",
  "supersedes": [
    "HANDOFF-20260823-88CF74"
  ],
  "tags": [
    "build",
    "deterministic",
    "export",
    "handoff",
    "m5"
  ],
  "task_id": "TASK-20260823-2A43C1",
  "tier": "short",
  "title": "M4 completion to M5 deterministic build and export handoff",
  "type_version": 1,
  "updated_at": "2026-08-23T20:42:52Z",
  "valid_as_of": "2026-08-24"
}
-->

# M4 completion to M5 deterministic build and export handoff

## Completed

M4 is complete on feature/m4-formula-xlsx. FieldId formulas, dependency and cycle handling, Native/Wasmtime parity, stable-identity XLSX export/import, cached-formula validation, immutable formula revisions, atomic row saves, API routes and Vue workflows are implemented. The free local gate passed 26 Rust tests, 10 Web tests, five migrations and Docker/PostgreSQL end-to-end acceptance.

## Repository State

The current branch is feature/m4-formula-xlsx, based on develop after M3 squash commit 1831b54. M4 product and project-memory changes are present in the working tree and are not yet integrated into develop. No paid service or sensitive data is involved.

## Exact Next Actions

1. Commit feature/m4-formula-xlsx, rerun the local gate if the diff changes and squash-integrate M4 into develop under the free local policy.
2. Create feature/m5-build-export from updated develop.
3. Pin builds to immutable schema revision, data revision, target configuration and plugin version inputs; produce a deterministic manifest with artifact hashes.
4. Complete the built-in output matrix: retain Rust/C#/TypeScript code generation and JSON/CSV data, then add XML, BSON, Protobuf and Lua outputs.
5. Define stable Protobuf wire IDs and reject incompatible reuse or accidental renumbering.
6. Add golden tests, parse/round-trip validation, deterministic rebuild comparisons and compilation checks for generated Rust, C# and TypeScript artifacts.
7. Extend the free quality gate and Docker acceptance for every M5 artifact without introducing paid infrastructure.

## Pending Scope

M5-M8 remain pending. Existing Rust/C#/TypeScript and JSON/CSV artifact support is only an M5 foundation; it does not complete the output matrix or acceptance criteria.

## Verification Caveat

The first complete M4 gate attempt exposed an outdated TableView total assertion after a second rollback-test row was added. The assertion was corrected from one to two and the full gate then passed. The curator independently confirmed 26 Rust tests with the exact quality-gate command.
