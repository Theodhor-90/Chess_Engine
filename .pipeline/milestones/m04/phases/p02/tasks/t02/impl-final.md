All verification commands passed. Implementation complete.

- **`crates/engine/src/main.rs`** — Replaced `Duration::from_secs(5)` with `let side = state.position.side_to_move(); chess_uci::time::allocate_time(&params, side)` in the `Go` handler's `wtime`/`btime` branch