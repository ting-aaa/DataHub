<!-- PROJECT_MEMORY
{
  "blockers": [],
  "confidence": "confirmed",
  "created_at": "2026-08-23T21:03:24Z",
  "derived_from": [
    "HANDOFF-20260823-E37206"
  ],
  "event_id": "datahub-handoff-m5-to-m6-v1",
  "id": "HANDOFF-20260823-F2BBAB",
  "kind": "handoff",
  "next_actions": [],
  "review_after": "2026-09-06",
  "schema_version": 1,
  "scope": [
    "apps/datahub-plugin-host",
    "crates",
    "deploy",
    "plugins",
    "scripts",
    "tests"
  ],
  "sensitivity": "internal",
  "sources": [
    "RPT-20260823-EE0875",
    "PLAN-20260823-9B6D1E",
    "Git evidence: clean feature/m6-plugin-sandbox at 2aaf6c8."
  ],
  "status": "superseded",
  "summary": "M5 is merged and fully verified; implement the WIT/Component plugin contract, version pinning and deny-by-default Wasmtime sandbox on the clean M6 branch.",
  "supersedes": [
    "HANDOFF-20260823-E37206"
  ],
  "tags": [
    "handoff",
    "m6",
    "plugin",
    "sandbox",
    "wasmtime",
    "wit"
  ],
  "task_id": "TASK-20260823-2A43C1",
  "tier": "short",
  "title": "M5 completion to M6 Wasmtime plugin platform handoff",
  "type_version": 1,
  "updated_at": "2026-08-23T21:23:21Z",
  "valid_as_of": "2026-08-24"
}
-->

# M5 completion to M6 Wasmtime plugin platform handoff

## Completed Baseline

M0-M5 are complete on develop at squash commit 2aaf6c8. M5 provides deterministic revision-pinned build manifests and the complete built-in code/data output matrix. The free local gate passes 29 Rust tests, 10 Web tests, migrations 0001-0006, five healthy application images, five builds with 45 artifacts, generated-code compilation, projection convergence and restart persistence.

## Repository State

feature/m5-deterministic-export and its remote branch are deleted after PR #5. develop equals origin/develop at 2aaf6c8. A clean feature/m6-plugin-sandbox branch already exists from that exact commit. apps/datahub-plugin-host currently provides only the process/container boundary; it is not yet the M6 plugin platform.

## M6 Objective

Implement a Wasmtime Component Model plugin platform with explicit WIT contracts, validated manifests, installation and immutable version pinning. Plugins receive only declared read-only inputs and may write only to declared output directories. Credentials and network access are unavailable by default.

## Required Work

1. Define versioned WIT/component interfaces and a plugin manifest that declares identity, version, compatible host ABI, capabilities, inputs, outputs and resource limits.
2. Implement plugin package validation, installation, immutable version pinning and deterministic selection by build/release inputs.
3. Run components through Wasmtime with no inherited credentials, environment, filesystem or network; preopen only declared read-only input and writable output directories.
4. Enforce canonical path containment against absolute paths, traversal, symlink escape and undeclared outputs.
5. Enforce time, memory, fuel and output-size/file-count quotas, with deterministic structured diagnostics.
6. Reject malformed packages, invalid manifests, incompatible WIT/ABI versions and capability escalation.
7. Provide a compiling example component and exercise it through the real plugin host and Docker boundary.
8. Add unit/integration and local-gate tests for success, version pinning, traversal, timeout, memory, fuel, output quota, malformed packages, forbidden network/credentials and deterministic outputs.

## Pending Scope

M6-M8 remain pending. The existing plugin-host application is a foundation only and must not be treated as proof of sandboxing, component compatibility or resource enforcement. M7 synchronization/release/rollback and M8 hardening/acceptance are out of M6 scope except for interfaces needed by later milestones.

## Constraints and Evidence

Use the free local Docker gate; do not introduce paid infrastructure. Preserve PostgreSQL migrations, independent image health checks, non-root/read-only runtime boundaries and uv-only Python. Relevant records are TASK-20260823-2A43C1, PLAN-20260823-9B6D1E, DEC-20260823-C69FFA, STD-20260823-048D0D and RPT-20260823-EE0875. No sensitive data was encountered.
