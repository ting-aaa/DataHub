<!-- PROJECT_MEMORY
{
  "blockers": [],
  "confidence": "confirmed",
  "created_at": "2026-08-23T19:01:37Z",
  "derived_from": [],
  "event_id": "datahub-report-m0-transition-m1-kernel-v1",
  "id": "RPT-20260823-FE85CD",
  "kind": "report",
  "next_actions": [],
  "review_after": "2026-09-06",
  "schema_version": 1,
  "scope": [
    "Cargo.lock",
    "Cargo.toml",
    "crates/datahub-kernel",
    "scripts"
  ],
  "sensitivity": "internal",
  "sources": [
    "Main-agent implementation delta for TASK-20260823-2A43C1 at index generation 23.",
    "Curator audit on 2026-08-24: git branch/log/ls-remote, scripts/quality-gate.ps1, kernel sources, Cargo manifests, cargo fmt, cargo clippy, and cargo test.",
    "RPT-20260823-118D95 resolves the previously recorded M1 gaps."
  ],
  "status": "superseded",
  "summary": "Historical M0/M1 checkpoint; its M1 gap assessment was subsequently resolved and is superseded by RPT-20260823-118D95.",
  "supersedes": [],
  "tags": [
    "domain-kernel",
    "local-ci",
    "m0",
    "m1",
    "verification"
  ],
  "task_id": "TASK-20260823-2A43C1",
  "tier": "short",
  "title": "M0 transition and M1 kernel checkpoint",
  "type_version": 1,
  "updated_at": "2026-08-23T19:43:06Z",
  "valid_as_of": "2026-08-24"
}
-->

# M0 transition and M1 kernel checkpoint

## M0 Transition

PR #1 was integrated into develop as squash commit 026cb6f. Local and remote feature/m0-foundation were deleted. The GitHub Actions workflow was removed, automatic merge was disabled, and required status contexts were removed while pull-request, force-push and deletion protections were retained.

scripts/quality-gate.ps1 passed end to end: Rust format/Clippy/tests; frozen pnpm install, Vue typecheck/Vitest/build; isolated Compose build/start on alternate loopback ports; API, proxy and web probes; migration version 1; persistence across down/up; and volume/container cleanup.

## M1 Delivered Kernel Slice

feature/m1-domain-kernel starts from develop at 026cb6f. The uncommitted implementation adds typed UUID wrappers, recursive TypeAst, ConfigValue and ConfigRow, schema canonicalization, structured validation codes/issues for schema/value/constraint failures, and deterministic Rust/C#/TypeScript Target IR with naming rules. Files are Cargo.toml/Cargo.lock and crates/datahub-kernel/src/id.rs, schema.rs, validation.rs, ir.rs and lib.rs.

## Verification

Curator reruns passed cargo fmt --all -- --check, workspace/all-targets/all-features Clippy with warnings denied, and cargo test --workspace --all-features. The workspace has 7 tests: API 1, kernel 5, and the existing health test 1; all passed. The first implementation Clippy run found match_same_arms in recursive type handling; Optional and List arms were merged and the rerun passed.

## Historical Contract Gaps and Resolution

At this checkpoint the audit found UUIDv4 and missing extended types, TargetRule, target-leak checks, and snapshot/permutation coverage. Subsequent implementation resolved every listed gap: all typed IDs now use UUIDv7; the accepted TypeAst surface and hard/soft references are present; language and C/S/E audience rules are separated; deterministic IR, target-leak validation, snapshot and permutation tests pass. RPT-20260823-118D95 is the current evidence.

## Supersession

M1 and M2 are complete. M3 editing and richer schema design are now active; use RPT-20260823-118D95 and HANDOFF-20260823-5A07F4 for current state.
