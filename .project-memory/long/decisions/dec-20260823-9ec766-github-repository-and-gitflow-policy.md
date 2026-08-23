<!-- PROJECT_MEMORY
{
  "blockers": [],
  "confidence": "confirmed",
  "created_at": "2026-08-23T18:24:09Z",
  "derived_from": [],
  "event_id": "datahub-bootstrap-github-gitflow-policy-v1",
  "id": "DEC-20260823-9EC766",
  "kind": "decision",
  "next_actions": [],
  "review_after": "",
  "schema_version": 1,
  "scope": [
    ".",
    ".github"
  ],
  "sensitivity": "internal",
  "sources": [
    "Explicit user-approved repository and branch policy in the DataHub planning conversation on 2026-08-24.",
    "Repository audit on 2026-08-24: git status reported no Git repository; PowerShell could not resolve the gh command.",
    "git remote, branch, log, and ls-remote verification on 2026-08-24; RPT-20260823-44414F.",
    "Repository settings and protections verified by the main agent; PR #1 and RPT-20260823-49833C."
  ],
  "status": "superseded",
  "summary": "The public MIT repository ting-aaa/DataHub uses CI-gated GitFlow with squash feature merges and merge-commit releases/hotfixes.",
  "supersedes": [],
  "tags": [
    "decision",
    "gitflow",
    "github"
  ],
  "task_id": "",
  "tier": "long",
  "title": "GitHub repository and GitFlow policy",
  "type_version": 1,
  "updated_at": "2026-08-23T18:53:29Z",
  "valid_as_of": "2026-08-24"
}
-->

# GitHub repository and GitFlow policy

## Policy

The public MIT repository is ting-aaa/DataHub. Main is the release branch and develop is the integration branch. Feature branches squash-merge into develop; release and hotfix branches merge-commit into main and reconcile back into develop. Required CI checks gate automatic merges.

## Current Application

Main remains the default branch. Main and develop enforce strict Rust checks, Web checks, and Docker smoke status checks, administrator enforcement, pull-request flow, conversation resolution, and no force pushes or deletion. M0 commit 66d7b8e is pushed and PR #1 has automatic squash merge enabled; GitHub Actions is externally blocked by the user account billing lock.

## Supersession

DEC-20260823-C69FFA replaces this policy because the user canceled all payment-dependent workflows. GitFlow remains, but required GitHub status checks and billing-dependent auto-merge do not.
