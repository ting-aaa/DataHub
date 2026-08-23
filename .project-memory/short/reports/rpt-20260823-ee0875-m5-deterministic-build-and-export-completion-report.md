<!-- PROJECT_MEMORY
{
  "blockers": [],
  "confidence": "confirmed",
  "created_at": "2026-08-23T21:03:10Z",
  "derived_from": [
    "RPT-20260823-38DC17"
  ],
  "event_id": "datahub-report-m5-deterministic-export-complete-v1",
  "id": "RPT-20260823-EE0875",
  "kind": "report",
  "next_actions": [
    "Continue M6 Wasmtime Component/WIT plugin platform implementation on feature/m6-plugin-sandbox."
  ],
  "review_after": "2026-09-06",
  "schema_version": 1,
  "scope": [
    "README.md",
    "apps/datahub-api",
    "crates/datahub-export",
    "crates/datahub-persistence-pg",
    "docs/product/v1-plan.md",
    "migrations/0006_deterministic_builds.sql",
    "scripts/quality-gate.ps1"
  ],
  "sensitivity": "internal",
  "sources": [
    "Main-agent M5 completion delta and full local quality-gate result on 2026-08-24.",
    "Git evidence: PR #5, feature commit 49a7f3e and squash commit 2aaf6c8; develop equals origin/develop.",
    "Curator repository audit and cargo test --workspace --all-features -- --test-threads=2: 29 tests and doctests passed."
  ],
  "status": "completed",
  "summary": "M5 completes deterministic revision-pinned manifests and the Rust/C#/TypeScript plus JSON/CSV/XML/BSON/Protobuf/Lua artifact matrix, fully verified and merged to develop.",
  "supersedes": [],
  "tags": [
    "build",
    "deterministic",
    "export",
    "m5",
    "protobuf"
  ],
  "task_id": "TASK-20260823-2A43C1",
  "tier": "short",
  "title": "M5 deterministic build and export completion report",
  "type_version": 1,
  "updated_at": "2026-08-23T21:03:10Z",
  "valid_as_of": "2026-08-24"
}
-->

# M5 deterministic build and export completion report

## Integration

Feature branch feature/m5-deterministic-export was committed at 49a7f3e and merged through PR #5 as squash commit 2aaf6c8. The remote feature branch was deleted. develop and origin/develop both resolve to 2aaf6c8.

## Export Matrix

The datahub-export crate now emits Rust, C# and TypeScript code plus JSON, CSV, XML, BSON, Protobuf schema/binary and Lua data artifacts. Stable Protobuf field tags derive only from immutable FieldId values, skip the reserved field-number range and reject collisions instead of silently reassigning tags.

## Deterministic Build Contract

Timestamp-free manifests record target and audience, immutable schema/data/row revisions and versions, plugin versions and sorted artifact hashes. Identical inputs produce identical manifests, input hashes and artifacts. Migration 0006 adds job input_hash and manifest storage plus its lookup index. The API reads the complete build input set in one repeatable-read read-only transaction and persists immutable artifacts, the manifest and input hash.

## Automated Acceptance

The free local gate parses every codec, checks the exact artifact matrix and manifest contents, compares identical rebuilds and compiles generated Rust, C# and TypeScript. The final full scripts/quality-gate.ps1 run passed 29 Rust tests, 10 Web tests, five independently built and healthy application images, SQLx migrations 0001-0006, five builds with 45 artifacts, projection convergence and PostgreSQL volume restart persistence. The curator independently reran the exact Rust test command and confirmed 29 tests plus doctests pass.

## Resolved Harness Failures

Two earlier complete-gate failures were limited to the test harness. First, manifest.json was incorrectly counted as a data JSON artifact; the matcher was narrowed to the data path. Second, generated TypeScript compilation resolved its project root incorrectly; the tsc lookup was corrected. The final full gate passed after both fixes.

## Milestone Status

M0-M5 are complete. M6-M8 remain pending, so TASK-20260823-2A43C1 stays active. The local Docker gate remains canonical and no paid service or binding was introduced. No sensitive data was encountered.
