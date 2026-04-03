Now I have all the context. Let me produce the definitive plan document.

# Task T02 — Binary Packaging: Implementation Plan

## 1. Deliverables

- **`.github/workflows/release.yml`** (modification) — Extend the existing workflow (created by T01) with a new `package` job that downloads all 8 build artifacts, bundles each with the NNUE weight file into a compressed archive, creates a GitHub Release, and attaches all 8 archives as release assets.

## 2. Dependencies

- **T01 (Cross-compilation)** — The existing `release.yml` workflow with its 8-entry build matrix must be in place. The `package` job downloads artifacts uploaded by the `build` job.
- **`actions/download-artifact@v4`** — Standard GitHub Action for downloading artifacts between jobs within a workflow.
- **`softprops/action-gh-release@v2`** — GitHub Action for creating releases and uploading assets. Chosen over raw `gh release create` because it handles idempotent release creation, multiple asset uploads, and authentication natively within Actions.
- **NNUE weight file at `nets/nn.nnue`** — Must exist in the repository. This is the trained network produced by M10/P02/T02, loaded at runtime via the `EvalFile` UCI option.
- No new Cargo dependencies required.

## 3. Implementation Details

### New `package` Job in `.github/workflows/release.yml`

A new job `package` is added after the existing `build` job. It runs on `ubuntu-latest` and depends on the `build` job completing successfully (`needs: build`).

**Job structure**:

```yaml
package:
  needs: build
  runs-on: ubuntu-latest
  permissions:
    contents: write
  steps:
    - name: Checkout
    - name: Download all artifacts
    - name: Package archives
    - name: Create GitHub Release
```

### Step 1 — Checkout

```yaml
- name: Checkout
  uses: actions/checkout@v4
```

Required to access the NNUE weight file (`nets/nn.nnue`) from the repository.

### Step 2 — Download All Artifacts

```yaml
- name: Download all artifacts
  uses: actions/download-artifact@v4
  with:
    path: artifacts/
```

This downloads all 8 artifacts uploaded by the `build` job into `artifacts/`. Each artifact is placed in a subdirectory named by its artifact name (e.g., `artifacts/engine-x86_64-unknown-linux-gnu-simd-avx2/engine`).

### Step 3 — Package Archives

A shell script iterates over the 8 artifact directories and creates a compressed archive for each, including the NNUE weight file.

**Archive naming convention**: `chess-engine-<target>-<simd>.<ext>` where:
- `<target>` is the Rust target triple (e.g., `x86_64-unknown-linux-gnu`)
- `<simd>` is the SIMD feature (e.g., `avx2`, `sse2`, `neon`) — the `simd-` prefix is stripped for brevity
- `<ext>` is `.tar.gz` for Linux/macOS targets, `.zip` for Windows targets

This produces 8 archives:

| # | Archive Name | Format |
|---|---|---|
| 1 | `chess-engine-x86_64-unknown-linux-gnu-sse2.tar.gz` | tar.gz |
| 2 | `chess-engine-x86_64-unknown-linux-gnu-avx2.tar.gz` | tar.gz |
| 3 | `chess-engine-x86_64-apple-darwin-sse2.tar.gz` | tar.gz |
| 4 | `chess-engine-x86_64-apple-darwin-avx2.tar.gz` | tar.gz |
| 5 | `chess-engine-x86_64-pc-windows-msvc-sse2.zip` | zip |
| 6 | `chess-engine-x86_64-pc-windows-msvc-avx2.zip` | zip |
| 7 | `chess-engine-aarch64-unknown-linux-gnu-neon.tar.gz` | tar.gz |
| 8 | `chess-engine-aarch64-apple-darwin-neon.tar.gz` | tar.gz |

**Packaging script**:

```yaml
- name: Package archives
  run: |
    mkdir -p dist/
    for dir in artifacts/engine-*; do
      artifact_name=$(basename "$dir")
      # Extract target and simd from artifact name: engine-<target>-simd-<level>
      # e.g., engine-x86_64-unknown-linux-gnu-simd-avx2
      target=$(echo "$artifact_name" | sed 's/^engine-//' | sed 's/-simd-[a-z0-9]*$//')
      simd_level=$(echo "$artifact_name" | sed 's/.*simd-//')
      archive_base="chess-engine-${target}-${simd_level}"

      if echo "$target" | grep -q "windows"; then
        # Windows: create .zip with engine.exe and nn.nnue
        cp "$dir/engine.exe" .
        cp nets/nn.nnue .
        zip "dist/${archive_base}.zip" engine.exe nn.nnue
        rm engine.exe nn.nnue
      else
        # Unix: create .tar.gz with engine and nn.nnue
        cp "$dir/engine" .
        chmod +x engine
        cp nets/nn.nnue .
        tar czf "dist/${archive_base}.tar.gz" engine nn.nnue
        rm engine nn.nnue
      fi
    done
    echo "Archives created:"
    ls -la dist/
```

**Key decisions**:
- The NNUE weight file is located at `nets/nn.nnue` in the repository, matching the M10/P02/T02 training pipeline output location.
- Archives are flat (no subdirectory inside) — the binary and weight file sit at the archive root for ease of extraction.
- The `chmod +x` ensures Unix binaries are executable after extraction.
- The script uses `sed` consistently to parse the artifact name, which follows the convention `engine-<target>-simd-<level>` established by T01's `actions/upload-artifact` step.

### Step 4 — Create GitHub Release

```yaml
- name: Create GitHub Release
  uses: softprops/action-gh-release@v2
  with:
    tag_name: ${{ github.ref_name }}
    name: ${{ github.ref_name }}
    draft: false
    prerelease: false
    files: dist/*
  env:
    GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```

**Notes**:
- `tag_name` uses `${{ github.ref_name }}`, which resolves to the branch name for `workflow_dispatch` (the current trigger) or the tag name when T03 adds the `push: tags: ['v*']` trigger.
- `softprops/action-gh-release@v2` handles creating the release and uploading all files matching `dist/*` as release assets.
- `permissions: contents: write` is set at the job level to allow release creation via `GITHUB_TOKEN`.
- T03 will add the tag trigger and changelog body. For now, the release is created with an auto-generated name matching the ref. This keeps T02 self-contained while leaving T03's scope intact.

## 4. API Contracts

N/A — this task modifies a CI workflow file, not a programmatic API.

## 5. Test Plan

| # | Test | What it verifies | Maps to verification criterion |
|---|------|-----------------|-------------------------------|
| 1 | YAML validity | Workflow YAML parses without syntax errors | VC 1 (packaging job exists) |
| 2 | Package job exists | `grep -q 'package:' .github/workflows/release.yml` | VC 1 |
| 3 | Package depends on build | `grep -q 'needs: build' .github/workflows/release.yml` | VC 1 |
| 4 | tar.gz format present | `grep -q 'tar.gz' .github/workflows/release.yml` | VC 2 (correct format) |
| 5 | zip format present | `grep -q '\.zip' .github/workflows/release.yml` | VC 2 (correct format) |
| 6 | NNUE weight reference | `grep -q 'nn\.nnue\|nets/nn\.nnue' .github/workflows/release.yml` | VC 3 (archive contains NNUE) |
| 7 | Archive naming includes target and SIMD | Inspect packaging script for `chess-engine-${target}-${simd_level}` pattern | VC 4 (named with target/SIMD) |
| 8 | Release creation step | `grep -q 'action-gh-release' .github/workflows/release.yml` | VC 5 (assets attached to release) |
| 9 | All 8 archives produced | Packaging script iterates over all `artifacts/engine-*` directories (8 from build matrix) | VC 2, VC 5 |
| 10 | Preflight checks pass | `cargo fmt --check && cargo clippy -- -D warnings && cargo test` | Existing code unchanged |

**Verification criteria mapping** (from task spec):
- **VC 1**: "The release workflow includes a packaging job/step that runs after the build matrix completes" → Tests 1–3
- **VC 2**: "Each of the 8 target/SIMD combinations produces a correctly-formatted archive" → Tests 4–5, 9
- **VC 3**: "Each archive contains both the engine binary and the NNUE weight file" → Test 6
- **VC 4**: "Archives are named with target triple and SIMD level for clear identification" → Test 7
- **VC 5**: "All 8 archives are attached as assets to a GitHub Release" → Tests 8–9

## 6. Implementation Order

1. **`.github/workflows/release.yml`** — Add the `package` job after the existing `build` job, containing all four steps (checkout, download artifacts, package archives, create release).

This is a single-file modification. The implementation is one logical addition to the workflow file.

## 7. Verification Commands

```bash
# Verify YAML is syntactically valid
python3 -c "import yaml; yaml.safe_load(open('.github/workflows/release.yml')); print('YAML valid')"

# Verify the package job exists and depends on build
grep -q 'package:' .github/workflows/release.yml && echo "package job exists"
grep -q 'needs: build' .github/workflows/release.yml && echo "package depends on build"

# Verify archive formats are correct (tar.gz for unix, zip for windows)
grep -q 'tar.gz' .github/workflows/release.yml && echo "tar.gz format present"
grep -q '\.zip' .github/workflows/release.yml && echo "zip format present"

# Verify NNUE weight file is referenced in the packaging step
grep -q 'nn\.nnue' .github/workflows/release.yml && echo "NNUE weight reference present"

# Verify GitHub Release creation step exists
grep -q 'action-gh-release' .github/workflows/release.yml && echo "Release creation step present"

# Verify archive naming convention includes target and SIMD
grep -q 'chess-engine-' .github/workflows/release.yml && echo "Archive naming convention present"

# Standard preflight checks still pass
cargo fmt --check
cargo clippy -- -D warnings
cargo test
```

## 8. Design Deviations

**Deviation 1 — Eight archives instead of seven**

- **What the parent spec requires:** The phase spec (spec-locked.md, Design Decision 3) states "This yields seven release artifacts total" and the Artifacts section states "7 total." The task spec says "Seven archives total (matching the seven binaries from T01)."
- **Why that is problematic:** The actual T01 implementation (`.github/workflows/release.yml`) produces **8** matrix entries: 3 x86-64 targets × 2 SIMD variants = 6, plus 2 ARM64 targets × 1 NEON variant = 2, totaling 8. The T01 deliverables description in the sibling task summary confirms "producing 8 release artifacts total." The spec miscounted (3×2 + 2×1 = 8, not 7).
- **Alternative chosen:** Produce 8 archives — one per build artifact — matching the actual T01 output. This is the only correct behavior since the packaging job must consume all artifacts the build matrix produces.

**Deviation 2 — Release creation is preliminary (no tag trigger or changelog)**

- **What the task spec implies:** "GitHub Release created with all archives attached as assets."
- **Why this needs qualification:** The current workflow trigger is `workflow_dispatch` (from T01). T03 is responsible for adding the tag trigger and changelog generation. The release creation step added here will work with `workflow_dispatch` using the current ref as the release name, but the full tag-triggered pipeline is T03's scope.
- **Alternative chosen:** Add the release creation step now so it works with manual dispatch, using `${{ github.ref_name }}` as the tag. T03 will refine the trigger to `push: tags: ['v*']` and add the changelog body. This keeps T02 self-contained while leaving T03's scope intact.

**Deviation 3 — `softprops/action-gh-release@v2` instead of `gh` CLI**

- **What the task spec suggests:** "Use `actions/upload-artifact` or direct GitHub Release asset upload via `gh` CLI or a release action."
- **Why `softprops/action-gh-release@v2` is preferred:** It handles release creation and multi-file asset upload in a single step, manages `GITHUB_TOKEN` authentication natively, is idempotent (safe to re-run), and is the most widely adopted release action in the GitHub Actions ecosystem. Using `gh release create` would require multiple commands to handle creation and asset upload separately, plus explicit authentication setup.