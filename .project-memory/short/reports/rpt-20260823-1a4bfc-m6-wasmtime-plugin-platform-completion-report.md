<!-- PROJECT_MEMORY
{
  "blockers": [],
  "confidence": "confirmed",
  "created_at": "2026-08-23T21:22:59Z",
  "derived_from": [
    "RPT-20260823-EE0875"
  ],
  "event_id": "datahub-report-m6-plugin-sandbox-complete-v1",
  "id": "RPT-20260823-1A4BFC",
  "kind": "report",
  "next_actions": [
    "Continue M7 projection, release, approval and rollback implementation on feature/m7-release-sync."
  ],
  "review_after": "2026-09-06",
  "schema_version": 1,
  "scope": [
    "apps/datahub-plugin-host",
    "deploy/docker",
    "docs/development/plugins.md",
    "examples/datahub-echo-plugin",
    "scripts/quality-gate.ps1",
    "wit/datahub-plugin.wit"
  ],
  "sensitivity": "internal",
  "sources": [
    "Main-agent M6 completion delta and full local quality-gate result on 2026-08-24.",
    "Git evidence: PR #6, feature commit 3a303a5 and squash commit 636a131; develop equals origin/develop.",
    "Curator repository audit and cargo test --workspace --all-features -- --test-threads=2: 33 tests and doctests passed."
  ],
  "status": "completed",
  "summary": "M6 completes the versioned WIT Component contract, immutable package registry and deny-by-default Wasmtime sandbox, fully verified and merged to develop.",
  "supersedes": [],
  "tags": [
    "m6",
    "plugin",
    "sandbox",
    "wasmtime",
    "wit"
  ],
  "task_id": "TASK-20260823-2A43C1",
  "tier": "short",
  "title": "M6 Wasmtime plugin platform completion report",
  "type_version": 1,
  "updated_at": "2026-08-23T21:22:59Z",
  "valid_as_of": "2026-08-24"
}
-->

# M6 Wasmtime plugin platform completion report

## Integration

Feature branch feature/m6-plugin-sandbox was committed at 3a303a5 and merged through PR #6 as squash commit 636a131. The feature branch was deleted. develop and origin/develop both resolve to 636a131.

## Component Contract and Package Registry

wit/datahub-plugin.wit defines world datahub-plugin with run(list<u8>) -> result<list<u8>, string>. The host links no WASI imports. The plugin-host library enforces a strict TOML manifest, component hash verification, compatible API semantic versions, safe lowercase identifiers, relative path validation, duplicate capability rejection and symlink rejection. Installation is immutable, exact-version, hash-pinned and idempotent; conflicting content for the same ID/version is rejected.

## Deny-by-default Execution

Plugins receive only declared read-only virtual inputs and return one declared output file below one declared output directory. Wasmtime Component execution uses fuel, epoch-based wall-clock interruption and StoreLimits memory bounds plus aggregate input/output quotas. The guest receives no ambient filesystem, environment, credentials, clocks, randomness, sockets or network capability.

## Example and Documentation

examples/datahub-echo-plugin compiles for wasm32-unknown-unknown, embeds WIT metadata and is componentized with wit-component. The plugin host includes run-package and componentize tooling. Plugin development documentation, README, architecture and v1 plan are updated, and the Docker builder copies the WIT contract.

## Automated Acceptance

The final free local quality gate passed 33 Rust tests and doctests, 10 Web tests, compiling and componentizing the example guest, a normal plugin run, rejection of a 2 MiB output, stopping a 128 MiB allocation under a 64 MiB limit, and independently stopping an infinite guest by fuel and a 10 ms epoch timeout. Five images built and were healthy; migrations 0001-0006, M5 five-build/45-artifact acceptance, generated Rust/C#/TypeScript compilation, projection convergence and volume restart persistence also passed.

The curator independently reran the Rust command and confirmed 33 tests: API 1, auth 3, export 6, formula 4, kernel 10, plugin host 4 and XLSX 5.

## Resolved Gate Failure

An earlier full gate failed because the plugin-host Docker builder did not copy the wit directory required by component bindgen. Adding the WIT copy fixed the image build, and the complete gate passed on rerun.

## Milestone Status

M0-M6 are complete. M7-M8 remain pending, so TASK-20260823-2A43C1 stays active. The local Docker gate remains canonical; no paid dependency or service was introduced. No sensitive data was encountered.
