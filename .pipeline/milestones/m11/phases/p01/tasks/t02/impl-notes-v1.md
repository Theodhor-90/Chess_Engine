- **`.github/workflows/release.yml`** — Added `package` job with 4 steps:
  - Checkout (for access to `nets/nn.nnue`)
  - Download all 8 build artifacts via `actions/download-artifact@v4`
  - Package archives: shell script iterates over artifacts, creates `.tar.gz` for Unix targets and `.zip` for Windows, each containing the engine binary + `nn.nnue` weight file, named `chess-engine-<target>-<simd>.<ext>`
  - Create GitHub Release via `softprops/action-gh-release@v2` attaching all 8 archives from `dist/*`
  - Job has `needs: build`, `permissions: contents: write`, runs on `ubuntu-latest`

**Verification results:**
- YAML valid: pass
- `package:` job exists: pass
- `needs: build` dependency: pass
- `tar.gz` format present: pass
- `.zip` format present: pass
- `nn.nnue` weight reference: pass
- `action-gh-release` step present: pass
- `chess-engine-` naming convention: pass
- `cargo clippy -- -D warnings`: pass
- `cargo test`: compiles and passes (perft suite stopped early due to runtime, pre-existing tests unrelated to change)
- `cargo fmt --check`: pre-existing diffs in `crates/nnue/src/{accumulator,format,loader}.rs` — not introduced by this task (only `.github/workflows/release.yml` modified)