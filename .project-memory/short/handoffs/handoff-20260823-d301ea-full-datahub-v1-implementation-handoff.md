<!-- PROJECT_MEMORY
{
  "blockers": [],
  "confidence": "confirmed",
  "created_at": "2026-08-23T18:53:25Z",
  "derived_from": [
    "HANDOFF-20260823-545AD6",
    "TASK-20260823-9C0927"
  ],
  "event_id": "datahub-handoff-full-v1-local-gates-v1",
  "id": "HANDOFF-20260823-D301EA",
  "kind": "handoff",
  "next_actions": [
    "Remove paid GitHub gate dependencies and run the local M0 quality gate.",
    "Integrate M0 into develop and begin M1 domain-kernel implementation."
  ],
  "review_after": "2026-09-06",
  "schema_version": 1,
  "scope": [
    "."
  ],
  "sensitivity": "internal",
  "sources": [
    "Explicit user scope change on 2026-08-24.",
    "DEC-20260823-C69FFA, TASK-20260823-2A43C1, and PLAN-20260823-9B6D1E."
  ],
  "status": "active",
  "summary": "The active objective is full DataHub v1 delivery through free local/Docker automated gates, beginning with removal of paid GitHub CI dependencies and M1 kernel work.",
  "supersedes": [
    "HANDOFF-20260823-545AD6"
  ],
  "tags": [
    "docker",
    "handoff",
    "local-ci",
    "v1"
  ],
  "task_id": "TASK-20260823-2A43C1",
  "tier": "short",
  "title": "Full DataHub v1 implementation handoff",
  "type_version": 1,
  "updated_at": "2026-08-23T18:53:25Z",
  "valid_as_of": "2026-08-24"
}
-->

# Full DataHub v1 implementation handoff

## Scope Change

The user canceled all workflows that require payment and expanded the active objective from M0 GitHub integration to full DataHub v1 implementation with automated tests. The prior billing-blocked GitHub Actions path is no longer a prerequisite.

## Completed Baseline

The M0 Docker-first source, local verification, public repository, and feature branch history remain valid. The old M0 task is closed by scope decision: its local deliverables are complete and its remaining paid-CI merge workflow is canceled.

## Active Work

TASK-20260823-2A43C1 and PLAN-20260823-9B6D1E govern M0 transition plus M1-M8. Quality gates must be free, local, Docker-first and automated. GitHub Actions may be optional but cannot block progress or require billing.

## Exact Next Actions

1. Remove required GitHub status checks and cancel the PR #1 auto-merge dependency.
2. Add and run the documented local quality-gate entrypoint against M0.
3. Integrate the verified M0 baseline into develop manually using the approved squash strategy and delete the feature branch.
4. Create feature/m1-domain-kernel from updated develop and implement M1 with automated unit/property/determinism tests.
5. Return milestone evidence and any durable decisions or failures to the curator.
