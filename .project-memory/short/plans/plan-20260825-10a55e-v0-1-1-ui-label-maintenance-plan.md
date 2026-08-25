<!-- PROJECT_MEMORY
{
  "blockers": [],
  "confidence": "confirmed",
  "created_at": "2026-08-25T15:55:40Z",
  "derived_from": [
    "PLAN-20260823-9B6D1E"
  ],
  "event_id": "datahub-plan-v0-1-1-ui-label-fixes-v1",
  "id": "PLAN-20260825-10A55E",
  "kind": "plan",
  "next_actions": [],
  "review_after": "2026-09-08",
  "schema_version": 1,
  "scope": [
    "web/src",
    "web/src/services"
  ],
  "sensitivity": "internal",
  "sources": [
    "TASK-20260825-69F856",
    "DEC-20260823-C69FFA",
    "STD-20260823-048D0D",
    "RPT-20260825-84B884",
    "RPT-20260825-E793AF",
    "HANDOFF-20260825-DBC1D1"
  ],
  "status": "completed",
  "summary": "Implementation, regressions, browser/full-gate acceptance, PR #20 squash integration and feature-branch cleanup are complete.",
  "supersedes": [],
  "tags": [
    "maintenance",
    "plan",
    "regression",
    "ui",
    "v0.1.1"
  ],
  "task_id": "TASK-20260825-69F856",
  "tier": "short",
  "title": "v0.1.1 UI label maintenance plan",
  "type_version": 1,
  "updated_at": "2026-08-25T16:29:22Z",
  "valid_as_of": "2026-08-25"
}
-->

# v0.1.1 UI label maintenance plan

## Step 1 - Branch and Reproduce - Completed

Create a feature/* branch from synced develop. Preserve the two browser observations as regression fixtures: unsaved formula state and historical build data with input_hash null.

## Step 2 - Presentation Fix - Completed

Extract or use deterministic label formatting that distinguishes unsaved from saved formula revisions and present from missing build hashes. Keep normal numeric versions and abbreviated hashes unchanged. Do not alter API response types or persisted history.

## Step 3 - Regression Coverage - Completed

Add focused frontend tests for unsaved formula, saved numeric formula, legacy null hash and normal hash labels. Assert that the rendered or formatted output contains neither `vnew` nor `undefined`.

## Step 4 - Verification - Completed

Run Web lint, typecheck, Vitest and production build during development. Run scripts/quality-gate.ps1 as the canonical final gate, retaining its Docker/PostgreSQL/runtime evidence and cleaning isolated quality resources.

## Step 5 - GitFlow Integration - Completed

PR #20 squash-merged feature/ui-label-polish into develop at 86e03a6 after the free local gate passed. develop equals origin/develop, and the original local and remote feature branches are deleted. RPT-20260825-E793AF records final evidence.

## Guardrails

No paid required CI, backend schema/API change, legacy data rewrite, credential, or unrelated UI redesign is in scope. Preserve Docker-first deployment and the completed v0.1.0 baseline.
