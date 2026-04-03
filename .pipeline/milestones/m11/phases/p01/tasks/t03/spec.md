# Task T03 — Versioning

## Objective

Adopt SemVer for the project, automate changelog generation from git history, and configure the release workflow to trigger on version tags — so that `git tag v1.0.0 && git push --tags` executes the full build-package-publish pipeline end to end.

## Deliverables

- Changelog generation step producing a changelog from git history between tags (commits since the previous tag)
- Release workflow trigger configured on `v*` tag pushes
- GitHub Release created with the generated changelog as the release body
- `Cargo.toml` workspace version aligned with the tagged SemVer version

## Key Implementation Details

- The release workflow (`.github/workflows/release.yml` from T01/T02) must be triggered by pushing a tag matching `v*` (e.g., `v1.0.0`)
- This separates development CI (run on PRs via `.github/workflows/regression.yml`) from release CI (run on tags), keeping PR feedback fast
- Changelog is auto-generated from git commit messages between the current tag and the previous tag
- The generated changelog is used as the GitHub Release body
- The workspace-level `Cargo.toml` version field should follow SemVer and be aligned with the tagged version
- The tag-triggered pipeline should execute the full sequence: build (T01) → package (T02) → create GitHub Release with changelog and attached archives

## Dependencies

- **T01 (Cross-compilation)** and **T02 (Binary Packaging)** must be complete — this task configures the trigger and changelog for the workflow they created

## Verification Criteria

1. The release workflow triggers on `v*` tag pushes (the `on:` section includes `push: tags: ['v*']`)
2. A changelog generation step produces a changelog from git commit history between the current and previous tags
3. The GitHub Release is created with the generated changelog as its body
4. The `Cargo.toml` workspace version follows SemVer format
5. The full pipeline (build → package → release with changelog) executes end-to-end when a version tag is pushed