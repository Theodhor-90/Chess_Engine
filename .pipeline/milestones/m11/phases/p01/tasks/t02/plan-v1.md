# Verify YAML is syntactically valid
python3 -c "import yaml; yaml.safe_load(open('.github/workflows/release.yml')); print('YAML valid')"

# Verify the package job exists and depends on build
grep -q 'package:' .github/workflows/release.yml && echo "package job exists"
grep -q 'needs: build' .github/workflows/release.yml && echo "package depends on build"

# Verify archive formats are correct (tar.gz for unix, zip for windows)
grep -q 'tar.gz' .github/workflows/release.yml && echo "tar.gz format present"
grep -q '\.zip' .github/workflows/release.yml && echo "zip format present"

# Verify NNUE weight file is referenced in the packaging step
grep -q 'nnue\|NNUE' .github/workflows/release.yml && echo "NNUE weight reference present"

# Verify GitHub Release creation step exists
grep -q 'gh-release\|gh release' .github/workflows/release.yml && echo "Release creation step present"

# Standard preflight checks still pass
cargo fmt --check
cargo clippy -- -D warnings
cargo test
```

## 8. Design Deviations

**Deviation 1 — Eight archives instead of seven**

- **What the parent spec requires:** The phase spec (spec-locked.md, Design Decision 3) states "This yields seven release artifacts total" and the task spec says "Seven archives total (matching the seven binaries from T01)."
- **Why that is problematic:** The actual T01 implementation (`.github/workflows/release.yml`) produces **8** matrix entries: 3 x86-64 targets × 2 SIMD variants = 6, plus 2 ARM64 targets × 1 NEON variant = 2, totaling 8. The T01 deliverables description in the sibling task summary confirms "producing 8 release artifacts total." The spec miscounted (3×2 + 2×1 = 8, not 7).
- **Alternative chosen:** Produce 8 archives — one per build artifact — matching the actual T01 output. This is the only correct behavior since the packaging job must consume all artifacts the build matrix produces.

**Deviation 2 — Release creation is preliminary (no tag trigger or changelog)**

- **What the task spec implies:** "GitHub Release created with all archives attached as assets."
- **Why this needs qualification:** The current workflow trigger is `workflow_dispatch` (from T01). T03 is responsible for adding the tag trigger and changelog generation. The release creation step added here will work with `workflow_dispatch` using the current ref as tag, but the full tag-triggered pipeline is T03's scope.
- **Alternative chosen:** Add the release creation step now so it works with manual dispatch, using `${{ github.ref_name }}` as the tag. T03 will refine the trigger to `push: tags: ['v*']` and add the changelog body. This keeps T02 self-contained while leaving T03's scope intact.