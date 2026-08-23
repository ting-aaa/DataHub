# GitFlow policy

- `main` contains releasable versions.
- `develop` is the integration branch.
- `feature/*` starts from `develop` and squash-merges back after CI succeeds.
- `release/*` starts from `develop`, merge-commits to `main`, receives a version
  tag, and is merged back to `develop`.
- `hotfix/*` starts from `main`, merge-commits to `main`, and is merged back to
  `develop`.

Direct pushes, force pushes, and deletion of `main` or `develop` are prohibited.
Pull requests require all configured checks and a conflict-free merge.
