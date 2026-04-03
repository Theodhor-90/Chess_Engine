# Task T02 — Binary Packaging

## Objective

Extend the release workflow created in T01 to package each built binary with the NNUE weight file and publish them as GitHub Release assets, producing clearly-named compressed archives.

## Deliverables

- Archive creation step in the release workflow producing named archives per target/SIMD combination
- NNUE weight file included in each archive as a companion file (not embedded in the binary)
- GitHub Release created with all archives attached as assets

## Archive Naming and Format

- Linux and macOS archives: `.tar.gz` format
- Windows archives: `.zip` format
- Archive names must include the target triple and SIMD level for clear identification (e.g., `chess-engine-x86_64-linux-avx2.tar.gz`)
- Seven archives total (matching the seven binaries from T01)

## Key Implementation Details

- Each archive contains: the engine binary + the NNUE weight file (`.nnue`) as a companion file
- NNUE weights are included alongside the binary (not compiled into it), matching the M10 design where weights are loaded from a `.nnue` file at runtime — this keeps binary size manageable and allows weight file updates without recompilation
- The packaging step consumes the workflow artifacts uploaded by T01's build matrix
- Use `actions/upload-artifact` or direct GitHub Release asset upload via `gh` CLI or a release action
- The GitHub Release should be created as part of this workflow (will be further configured in T03 with changelog)

## Dependencies

- **T01 (Cross-compilation)** must be complete — this task consumes the build artifacts produced by T01's workflow matrix

## Verification Criteria

1. The release workflow includes a packaging job/step that runs after the build matrix completes
2. Each of the 7 target/SIMD combinations produces a correctly-formatted archive (`.tar.gz` or `.zip`)
3. Each archive contains both the engine binary and the NNUE weight file
4. Archives are named with target triple and SIMD level for clear identification
5. All 7 archives are attached as assets to a GitHub Release