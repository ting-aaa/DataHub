<!-- PROJECT_MEMORY
{
  "blockers": [],
  "confidence": "confirmed",
  "created_at": "2026-08-23T21:42:31Z",
  "derived_from": [
    "HANDOFF-20260823-93F5C0"
  ],
  "event_id": "datahub-handoff-m7-to-m8-v1",
  "id": "HANDOFF-20260823-CAFFC2",
  "kind": "handoff",
  "next_actions": [],
  "review_after": "2026-09-06",
  "schema_version": 1,
  "scope": [
    "apps",
    "crates",
    "deploy",
    "docs",
    "migrations",
    "scripts",
    "tests",
    "web"
  ],
  "sensitivity": "internal",
  "sources": [
    "RPT-20260823-9F7FDD",
    "PLAN-20260823-9B6D1E",
    "Curator Git audit: M7 complete in uncommitted feature/m7-release-sync working tree based on 636a131."
  ],
  "status": "superseded",
  "summary": "M7 is fully verified but pending integration; then complete audit/rate-limit/observability/backup hardening and prove the entire v1 from a clean checkout and fresh Docker volumes.",
  "supersedes": [
    "HANDOFF-20260823-93F5C0"
  ],
  "tags": [
    "acceptance",
    "backup",
    "handoff",
    "hardening",
    "m8",
    "observability",
    "security"
  ],
  "task_id": "TASK-20260823-2A43C1",
  "tier": "short",
  "title": "M7 completion to M8 hardening and final acceptance handoff",
  "type_version": 1,
  "updated_at": "2026-08-24T15:14:00Z",
  "valid_as_of": "2026-08-24"
}
-->

# M7 completion to M8 hardening and final acceptance handoff

## Completed Baseline

M0-M7 are functionally complete. The free gate passes 35 Rust tests, 10 Web tests, plugin adversarial tests, migrations 0001-0007, five images, deterministic artifact compilation, projection planning/retry/dead-letter/full-resync, release approval/publish/rollback and restart persistence.

## Repository State

M7 is complete in the feature/m7-release-sync working tree but is not committed or integrated. HEAD, develop and origin/develop remain at 636a131. Before M8 work, commit M7, rerun the gate if the diff changes, open the feature PR, squash-merge to develop, delete the branch and create feature/m8-hardening-acceptance from updated develop.

## M8 Objective

Finish operational and security hardening, prove backup/recovery and run the complete DataHub v1 workflow under documented performance/concurrency budgets from a clean checkout and fresh Docker volumes. Only after every acceptance item passes may TASK-20260823-2A43C1 and PLAN-20260823-9B6D1E be completed.

## Required Implementation

1. Add project-scoped audit search with deterministic pagination and filters for actor, action, aggregate, correlation identifier and time range; enforce RBAC and prevent cross-project disclosure.
2. Add configurable rate limiting for authentication and expensive/mutating API paths. Return deterministic 429 responses and verify limits, recovery windows and trusted-client handling without weakening local development.
3. Validate required configuration at startup, keep secrets only in external environment/secret sources, redact credentials/tokens/session material from logs and errors, and add automated repository/config secret checks that require no paid service.
4. Add structured service logs, request/correlation IDs and bounded metrics for HTTP outcomes/latency, database readiness, outbox pending/retry/dead-letter state, projection checkpoints, plugin traps/quotas and release actions. Document local inspection and alert-response steps.
5. Provide Docker-first PostgreSQL backup and restore commands/runbooks. Restore into a fresh volume and prove schemas, stable IDs, revisions, rows, formulas, artifacts, plugin pins, projection state, environments, releases, approvals, rollback history, audit and outbox integrity.
6. Complete operator runbooks for deployment, upgrade/migration, health diagnosis, dead-letter recovery, full resync, backup/restore, release approval/publish/rollback and disaster recovery.
7. Exercise one full demo through bootstrap/login, RBAC, schema design, rows/VTable, formulas Native/Wasm, XLSX preview/atomic commit, deterministic multi-format builds, component plugin execution, PostgreSQL projection, environment approval, publication and rollback.
8. Add large-table, concurrent-editor/optimistic-conflict, restart, security-boundary and failure-recovery suites. Commit explicit dataset, concurrency, duration and latency/resource budgets; make the gate fail when those budgets are exceeded.
9. Run the complete local gate from a clean checkout with freshly created Docker volumes, then rerun after restart and after backup/restore. No test may rely on prior containers, cached database state, paid infrastructure or hidden credentials.
10. Reconcile README, architecture, product plan, plugin docs and operational docs with the verified system; record exact commands, versions and known limitations.

## Precise Exit Criteria

M8 passes only when: Rust fmt/Clippy/tests/doctests and Web lint/typecheck/tests/build pass; all five images build independently and become healthy; migrations 0001-0007 apply to an empty PostgreSQL volume; the full v1 demo succeeds; audit search/RBAC, rate limiting, redaction/secret checks and observability assertions pass; plugin sandbox adversarial cases remain enforced; deterministic builds and generated-code compilation remain stable; projection retry/dead-letter/checkpoint/full-resync and release approval/publish/rollback remain reproducible; backup restored into a fresh volume preserves every enumerated durable record and permits the demo to continue; large-table/concurrency/restart/recovery tests meet committed budgets; and a second clean restart reports ready without data loss.

Any failed item keeps M8 and the full-v1 task active. Integration of M8 into develop and final release/handoff evidence are part of completion.

## Constraints and Evidence

PostgreSQL and Docker Compose remain canonical. Use committed SQLx migrations, non-root/read-only health-checked images, the free local gate and uv-only Python. Do not add paid services. Relevant memory is TASK-20260823-2A43C1, PLAN-20260823-9B6D1E, DEC-20260823-C69FFA, STD-20260823-048D0D, RPT-20260823-9F7FDD and HANDOFF-20260823-93F5C0. No sensitive data was encountered.
