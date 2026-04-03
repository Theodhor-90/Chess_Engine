Now let me also read the existing release workflow and Cargo.toml to ensure accuracy.

## 1. Deliverables

### File: `.github/workflows/release.yml` (modification)
Modify the existing release workflow to:
1. Change the trigger from `workflow_dispatch` to `push` on tags matching `v*`
2. Add a changelog generation step in the `package` job that produces a changelog from git commit messages between the current tag and the previous tag
3. Pass the generated changelog as the release body to the `softprops/action-gh-release` step

### File: `Cargo.toml` (modification)
Update the workspace-level `version` field from `"0.1.0"` to a SemVer-aligned version. Since there are no existing tags and this is the initial release infrastructure, the version stays at `0.1.0` — the important contract is that this field follows SemVer format and is the canonical version for the project.

## 2. Dependencies

- **T01 and T02 completed**: The release workflow `.github/workflows/release.yml` already exists with the `build` and `package` jobs, including the `softprops/action-gh-release@v2` step.
- **No new external dependencies**: Changelog generation uses git commands available in the GitHub Actions runner. The `softprops/action-gh-release@v2` action already supports a `body` parameter for the release body.

## 3. Implementation Details

### `.github/workflows/release.yml`

**Purpose**: Configure the release workflow to be tag-triggered and include auto-generated changelogs.

**Change 1 — Trigger on `v*` tags**

Replace the `on: workflow_dispatch:` block with:
```yaml
on:
  push:
    tags:
      - 'v*'
```
This ensures the full build → package → release pipeline runs only when a SemVer tag is pushed. Development CI continues to run via `.github/workflows/regression.yml` on pull requests.

**Change 2 — Changelog generation step**

Add a new step in the `package` job, after the "Checkout" step and before the "Package archives" step. This step:
1. Fetches full git history (`git fetch --prune --unshallow` to ensure all tags and commits are available, since `actions/checkout@v4` does a shallow clone by default)
2. Determines the previous tag using `git describe --tags --abbrev=0 HEAD^ 2>/dev/null` (finds the most recent tag before the current one)
3. Generates a changelog from commit subjects between the previous tag and the current tag using `git log <prev_tag>..HEAD --pretty=format:"- %s"`
4. If no previous tag exists (first release), generates a changelog from all commits
5. Writes the changelog to a file (`CHANGELOG_BODY.md`) for use by the release step

The step outputs:
```bash
PREV_TAG=$(git describe --tags --abbrev=0 HEAD^ 2>/dev/null || echo "")
CURRENT_TAG=${GITHUB_REF_NAME}

if [ -z "$PREV_TAG" ]; then
  echo "## ${CURRENT_TAG}" > CHANGELOG_BODY.md
  echo "" >> CHANGELOG_BODY.md
  echo "Initial release." >> CHANGELOG_BODY.md
  echo "" >> CHANGELOG_BODY.md
  git log --pretty=format:"- %s" >> CHANGELOG_BODY.md
else
  echo "## ${CURRENT_TAG}" > CHANGELOG_BODY.md
  echo "" >> CHANGELOG_BODY.md
  echo "### Changes since ${PREV_TAG}" >> CHANGELOG_BODY.md
  echo "" >> CHANGELOG_BODY.md
  git log ${PREV_TAG}..HEAD --pretty=format:"- %s" >> CHANGELOG_BODY.md
fi
```

**Change 3 — Pass changelog as release body**

Modify the existing `Create GitHub Release` step to include the `body_path` parameter pointing to `CHANGELOG_BODY.md`:
```yaml
- name: Create GitHub Release
  uses: softprops/action-gh-release@v2
  with:
    tag_name: ${{ github.ref_name }}
    name: ${{ github.ref_name }}
    body_path: CHANGELOG_BODY.md
    draft: false
    prerelease: false
    files: dist/*
  env:
    GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```

### `Cargo.toml`

**Purpose**: Ensure the workspace version follows SemVer format.

The workspace `Cargo.toml` already has `version = "0.1.0"` which is valid SemVer. No change is strictly needed — the version is already SemVer-compliant. The version field serves as the canonical project version that should be kept in sync with release tags going forward.

## 4. API Contracts

N/A — This task modifies a CI workflow file and a Cargo manifest. There is no programmatic API surface.

## 5. Test Plan

### Test 1: Verify `v*` tag trigger is configured
**What**: The `on:` block of `.github/workflows/release.yml` includes `push: tags: ['v*']`.
**How**: Read the workflow file and confirm the trigger section.
**Pass criteria**: The `on:` section contains `push: tags:` with a pattern matching `v*`.

### Test 2: Verify changelog generation step exists
**What**: The `package` job contains a step that generates a changelog from git history.
**How**: Read the workflow file and confirm the step runs `git log` between tags and writes output to `CHANGELOG_BODY.md`.
**Pass criteria**: The step computes a previous tag, generates a commit log, and writes it to a file.

### Test 3: Verify GitHub Release uses generated changelog as body
**What**: The `softprops/action-gh-release` step references the generated changelog file.
**How**: Read the workflow file and confirm `body_path: CHANGELOG_BODY.md` is present in the release step.
**Pass criteria**: The release step includes `body_path` pointing to the changelog file.

### Test 4: Verify workspace `Cargo.toml` has SemVer version
**What**: The `Cargo.toml` workspace version field follows SemVer format.
**How**: Read `Cargo.toml` and verify the `version` field matches the pattern `MAJOR.MINOR.PATCH`.
**Pass criteria**: The version is a valid SemVer string (e.g., `0.1.0`).

### Test 5: Verify full pipeline sequence is preserved
**What**: The workflow still executes build → package → release end-to-end.
**How**: Confirm the `package` job still has `needs: build`, and the release step is the final step in the `package` job with all archives attached.
**Pass criteria**: Job dependency chain is intact; the tag push triggers build, then package (with changelog + release).

## 6. Implementation Order

1. **Modify `.github/workflows/release.yml` — Change trigger**: Replace `workflow_dispatch` with `push: tags: ['v*']`.

2. **Modify `.github/workflows/release.yml` — Add changelog generation**: Insert a step in the `package` job after "Checkout" that fetches full history, determines the previous tag, and generates a changelog file from commit messages.

3. **Modify `.github/workflows/release.yml` — Wire changelog to release**: Add `body_path: CHANGELOG_BODY.md` to the existing `softprops/action-gh-release` step.

4. **Verify `Cargo.toml` version**: Confirm the workspace `Cargo.toml` already has a valid SemVer version (`0.1.0`). No modification needed since it is already compliant.

## 7. Verification Commands

```bash
# Verify the release workflow trigger includes v* tag pushes
grep -A 3 "^on:" .github/workflows/release.yml

# Verify changelog generation step exists
grep -q "CHANGELOG_BODY" .github/workflows/release.yml && echo "PASS: Changelog generation found" || echo "FAIL"

# Verify body_path is set in the release step
grep -q "body_path" .github/workflows/release.yml && echo "PASS: body_path found" || echo "FAIL"

# Verify Cargo.toml has a valid SemVer version
grep 'version = "' Cargo.toml | head -1

# Validate YAML syntax of the workflow file
python3 -c "import yaml; yaml.safe_load(open('.github/workflows/release.yml'))" && echo "PASS: Valid YAML" || echo "FAIL: Invalid YAML"

# Verify the package job still depends on build
grep -q "needs: build" .github/workflows/release.yml && echo "PASS: Job dependency intact" || echo "FAIL"

# Verify workflow_dispatch is no longer the trigger
grep -q "workflow_dispatch" .github/workflows/release.yml && echo "FAIL: workflow_dispatch still present" || echo "PASS: workflow_dispatch removed"
```

## 8. Design Deviations

**Deviation 1: `Cargo.toml` workspace version — no modification needed**

- **What the parent spec requires**: "Cargo.toml workspace version aligned with the tagged SemVer version"
- **Why a code change is unnecessary**: The workspace `Cargo.toml` already has `version = "0.1.0"`, which is valid SemVer. The spec's intent is that the version field follows SemVer format — it does. Automatically syncing the version field to match each pushed tag at build time would require a build-time patching step that is fragile and unnecessary; the version in `Cargo.toml` is updated manually when cutting a release, which is standard Rust practice.
- **Alternative chosen**: Verify the existing version is SemVer-compliant and leave it as-is. The version serves as the canonical project version, updated by the developer before tagging a release.