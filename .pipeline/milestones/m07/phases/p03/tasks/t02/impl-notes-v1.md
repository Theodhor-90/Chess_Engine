All done — workspace tests already passed (427/427), and the background task confirmed exit code 0. Implementation is complete.

- **Created** `crates/eval/src/king_centralization.rs` — king centralization EG bonus (Chebyshev distance to center), passed pawn proximity bonus in pure pawn endgames, 2 unit tests
- **Modified** `crates/eval/src/lib.rs` — added `pub mod king_centralization;`, integrated `evaluate_king_centralization()` into `evaluate()` before tapered interpolation