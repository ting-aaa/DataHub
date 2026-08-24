<!-- PROJECT_MEMORY
{
  "blockers": [],
  "confidence": "confirmed",
  "created_at": "2026-08-23T18:52:45Z",
  "derived_from": [
    "RPT-20260823-49833C"
  ],
  "event_id": "datahub-decision-free-local-quality-gates-v1",
  "id": "DEC-20260823-C69FFA",
  "kind": "decision",
  "next_actions": [
    "Keep scripts/quality-gate.ps1 as the required free local acceptance gate for future changes."
  ],
  "review_after": "",
  "schema_version": 1,
  "scope": [
    ".",
    ".github",
    "deploy",
    "tools"
  ],
  "sensitivity": "internal",
  "sources": [
    "Explicit user instruction on 2026-08-24 cancelling any workflow that requires payment and requiring full implementation with automated tests.",
    "DEC-20260823-9EC766 and RPT-20260823-49833C document the superseded CI-gated GitHub Actions policy and billing lock."
  ],
  "status": "active",
  "summary": "DataHub uses automated local/Docker quality gates and manual GitFlow integration; paid GitHub Actions, required remote statuses, and billing-dependent auto-merge are not allowed.",
  "supersedes": [
    "DEC-20260823-9EC766"
  ],
  "tags": [
    "decision",
    "docker",
    "gitflow",
    "local-ci",
    "no-paid-services"
  ],
  "task_id": "",
  "tier": "long",
  "title": "Free local Docker-first quality gate and GitFlow policy",
  "type_version": 1,
  "updated_at": "2026-08-24T15:53:24Z",
  "valid_as_of": "2026-08-24"
}
-->

# Free local Docker-first quality gate and GitFlow policy

## Decision

DataHub must not depend on any paid workflow. GitHub Actions checks that cannot run without resolving a billing lock are removed as required branch statuses, and automatic merge must not be a completion dependency. GitHub may remain the public source repository and PR/history surface, but repository integration is authorized only after the free local quality gate passes.

## Free Quality Gate

The canonical gate runs on the developer workstation and Docker Desktop. It automates Rust format, Clippy and tests; Vue typecheck, Vitest and production build; Docker image and Compose validation; fresh-database migrations; API/web health and smoke checks; persistence/restart checks; and milestone-specific integration, golden, generated-code compilation, E2E, performance and security tests. Commands must return nonzero on failure and preserve inspectable logs/results.

## Integration Policy

Retain GitFlow branch meanings. Feature work is squash-merged into develop only after the local gate passes and its evidence is recorded. Release and hotfix branches use merge commits as previously agreed. Required GitHub status checks and billing-dependent auto-merge are removed; GitHub Actions may be used only as an optional non-blocking mirror when it is free and available.

## Rejected Alternatives

Paying to unlock GitHub Actions is explicitly rejected by the user. Removing automation entirely is rejected because the user requires automated tests. Merging without evidence is rejected because it weakens the established quality standard.
