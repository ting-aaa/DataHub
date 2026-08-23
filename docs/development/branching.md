# GitFlow policy

- `main` contains releasable versions.
- `develop` is the integration branch.
- `feature/*` starts from `develop` and squash-merges back after the local quality
  gate succeeds.
- `release/*` starts from `develop`, merge-commits to `main`, receives a version
  tag, and is merged back to `develop`.
- `hotfix/*` starts from `main`, merge-commits to `main`, and is merged back to
  `develop`.

Direct pushes, force pushes, and deletion of `main` or `develop` are prohibited.
Pull requests require a conflict-free merge. Before merging, run
`pwsh -NoProfile -File scripts/quality-gate.ps1` and record the result in the PR.
The repository does not depend on paid cloud CI runners.
