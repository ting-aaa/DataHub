<!-- PROJECT_MEMORY
{
  "blockers": [],
  "confidence": "confirmed",
  "created_at": "2026-08-25T15:55:25Z",
  "derived_from": [
    "HANDOFF-20260824-4978F1"
  ],
  "event_id": "datahub-task-v0-1-1-ui-label-fixes-v1",
  "id": "TASK-20260825-69F856",
  "kind": "task",
  "next_actions": [
    "Commit and push feature/ui-label-polish, open the develop PR and squash-merge it under the free local gate policy.",
    "Verify develop/origin-develop equality and local/remote feature-branch cleanup, then close the task and plan."
  ],
  "review_after": "2026-09-08",
  "schema_version": 1,
  "scope": [
    "web/src",
    "web/src/services"
  ],
  "sensitivity": "internal",
  "sources": [
    "User-directed maintenance objective and browser observations on 2026-08-25.",
    "Curator audit: develop/origin-develop at 3d6fea5 with clean worktree; App.vue label expressions and api.ts nullable input_hash model.",
    "RPT-20260825-84B884 records implemented labels, browser acceptance and the complete local quality gate."
  ],
  "status": "active",
  "summary": "The two Vue labels, regressions, browser acceptance and full local gate are complete; GitFlow squash integration remains.",
  "supersedes": [],
  "tags": [
    "maintenance",
    "regression",
    "ui",
    "v0.1.1"
  ],
  "task_id": "",
  "tier": "short",
  "title": "Fix formula and legacy build labels for v0.1.1 maintenance",
  "type_version": 1,
  "updated_at": "2026-08-25T16:26:33Z",
  "valid_as_of": "2026-08-25"
}
-->

# Fix formula and legacy build labels for v0.1.1 maintenance

## Objective

Correct two browser-visible labels without changing backend contracts: an unsaved formula must not render as `formula vnew`, and a historical build with a null input_hash must not render as `rust · undefined`.

## Confirmed Symptoms

web/src/App.vue currently concatenates the formula prefix with `formulaVersion ?? 'new'`, producing the fused text `formula vnew`. The build selector interpolates optional slicing of a nullable input_hash, which stringifies the missing value as `undefined`. web/src/services/api.ts correctly models the historical build field as `string | null`.

## Scope

Limit product changes to the Vue presentation/helpers and focused frontend regression tests under web/src. Preserve API, PostgreSQL, Docker and release behavior. Do not backfill or fabricate hashes for legacy rows.

## Progress

Implementation and acceptance are complete on feature/ui-label-polish. Shared label helpers and two focused tests are present; Web checks, real-browser acceptance and the complete Docker-first quality gate passed. The task remains active only for commit, PR, squash merge and branch-cleanup verification described by RPT-20260825-84B884.

## Acceptance Criteria

- Unsaved formulas display a clear non-version label and saved formulas retain their numeric version presentation.
- Builds with a present input_hash retain the target plus abbreviated hash; legacy null hashes show an explicit stable fallback and never `undefined`.
- Regression tests cover both null/new and normal version/hash cases.
- Frontend lint, typecheck, tests and build pass; the canonical scripts/quality-gate.ps1 completes successfully.
- Work is developed on a feature/* branch from synced develop, squash-merged into develop, and the feature branch is cleaned up under the free local gate policy.

## Starting State

develop and origin/develop both resolve to 3d6fea54ddefb34b64b73dbc103a1aee053f989f. The worktree is clean, Docker services are running, and the browser has no console errors. No paid external gate is required.
