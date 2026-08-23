<!-- PROJECT_MEMORY
{
  "blockers": [],
  "confidence": "confirmed",
  "created_at": "2026-08-23T19:43:00Z",
  "derived_from": [
    "HANDOFF-20260823-DE8966"
  ],
  "event_id": "datahub-handoff-m2-to-m3-editing-v1",
  "id": "HANDOFF-20260823-5A07F4",
  "kind": "handoff",
  "next_actions": [],
  "review_after": "2026-09-06",
  "schema_version": 1,
  "scope": [
    "apps/datahub-api",
    "crates/datahub-persistence-pg",
    "migrations",
    "web"
  ],
  "sensitivity": "internal",
  "sources": [
    "RPT-20260823-118D95",
    "git status and verification reruns on 2026-08-24."
  ],
  "status": "superseded",
  "summary": "M0-M2 are complete; M3 API/console flows work, but VTable block editing and richer schema design remain the active scope before M4.",
  "supersedes": [
    "HANDOFF-20260823-DE8966"
  ],
  "tags": [
    "api",
    "handoff",
    "m3",
    "vtable",
    "web"
  ],
  "task_id": "TASK-20260823-2A43C1",
  "tier": "short",
  "title": "M2 completion to M3 editing handoff",
  "type_version": 1,
  "updated_at": "2026-08-23T19:51:14Z",
  "valid_as_of": "2026-08-24"
}
-->

# M2 completion to M3 editing handoff

## Completed

M0 is integrated. M1 is now contract-complete, including UUIDv7, the accepted type surface, language/audience target rules, target-leak protection and deterministic snapshot/permutation coverage. M2 persistence, revisions, audit/outbox, local auth and RBAC is implemented and verified.

## In Progress

M3 has working bootstrap/login/project/schema/row/build/sync API and console flows, including VTable display and reference existence checks. The working tree on feature/m1-domain-kernel contains the current M1-M3 implementation and project-memory updates; it has not yet been integrated into develop.

## Exact Next Actions

1. Implement the 512-row block/query API with server-side filtering and sorting.
2. Add VTable direct cell editing, sparse block cache, optimistic version submission, HTTP 409 conflict presentation, batch paste and undo/redo.
3. Expand the schema/type designer beyond the current field kinds and support practical multi-field editing.
4. Add API, persistence and browser tests for block reads, edits, conflicts, references and RBAC.
5. Run the full local quality gate, record database/HTTP/browser evidence, and integrate the feature branch under the free local policy.
6. Begin M4 formula and XLSX only after M3 acceptance is complete.

## Pending Milestones

M4 formula/XLSX, complete M5 output/build coverage, M6 Wasmtime plugins, M7 release/rollback and full sync policy, and M8 hardening/acceptance remain pending. Existing JSON/CSV/codegen, artifacts and projection sync are partial later-milestone foundations only.

## Verification Snapshot

Current quick verification passes Rust format, Clippy and 17 tests; Vue typecheck, 7 tests and production build. The latest complete quality gate and browser smoke evidence is summarized in RPT-20260823-118D95. No secrets were encountered.
