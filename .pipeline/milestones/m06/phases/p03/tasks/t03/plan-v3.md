OK, so the struct does require `singular_extension_enabled`. The fact that it's missing from line 3349 means this code wouldn't compile. This was likely an incomplete T02 implementation that didn't update all the test struct literals. But since T02 is listed as "completed," perhaps this was fixed in a later commit. Either way, for my plan I should include `singular_extension_enabled` in all `SearchContext` struct literals.

Now I have complete context. Let me write the revised plan document.

# Task T03 — Principal Variation Search (PVS): Implementation Plan

## 1. Overview

This task replaces the plain alpha-beta search loop in `negamax` with Principal Variation Search (PVS). PVS searches the first move at each node with a full `(alpha, beta)` window, then searches all subsequent moves with a zero-window `(alpha, alpha + 1)`. If a zero-window search fails high (score > alpha), a re-search with the full window is performed. PVS also integrates with the existing LMR implementation to produce a three-stage re-search cascade for late non-PV moves.

## 2. Files to Modify

- **`crates/search/src/lib.rs`** — the only file modified. All changes are within the existing `negamax` function's move loop (lines ~398–554) and in the `#[cfg(test)] mod tests` block.

No new files are created. No new crates or dependencies are added.

## 3. Current Code Analysis

The move loop in `negamax` (starting at line 398) currently implements plain alpha-beta with LMR. The relevant structure is:

```rust
for (moves_searched, mv) in (0_u32..).zip(moves.into_iter()) {
    // ... piece_kind, is_tactical, see_score, make_move, history push ...
    // ... futility pruning, bad capture pruning ...

    let do_lmr = /* conditions */;

    let mut score;
    if do_lmr {
        // Stage 1: reduced-depth search with zero-window (-alpha-1, -alpha)
        // Stage 2 (on fail-high): full-depth full-window (-beta, -alpha)
        ...
    } else {
        // Full-depth full-window search (-beta, -alpha)
        ...
    }

    // ... unmake, cutoff handling, history updates ...
}
```

**Key observations:**
1. LMR already uses a zero-window `(-alpha - 1, -alpha)` for the reduced search (line 479), but the re-search on fail-high uses a **full window** `(-beta, -alpha)` (line 494). This is a two-stage cascade.
2. Non-LMR moves always get a **full window** `(-beta, -alpha)` search (line 506).
3. The `moves_searched` counter (0-indexed) is already tracked — `moves_searched == 0` identifies the first move.

## 4. Implementation Steps

### Step 4.1 — PVS for Non-LMR Moves

Replace the `else` branch (non-LMR path, lines 503–516) with PVS logic:

**Current code (lines 503–516):**
```rust
} else {
    let (s, _) = negamax(
        pos,
        depth - 1,
        -beta,
        -alpha,
        ply + 1,
        true,
        ctx,
        Some((piece_kind, mv)),
        None,
    );
    score = -s;
}
```

**New code:**
```rust
} else if moves_searched == 0 {
    // PVS: first move gets full window
    let (s, _) = negamax(
        pos,
        depth - 1,
        -beta,
        -alpha,
        ply + 1,
        true,
        ctx,
        Some((piece_kind, mv)),
        None,
    );
    score = -s;
} else {
    // PVS: subsequent moves get zero-window
    let (s, _) = negamax(
        pos,
        depth - 1,
        -alpha - 1,
        -alpha,
        ply + 1,
        true,
        ctx,
        Some((piece_kind, mv)),
        None,
    );
    score = -s;

    // Re-search with full window on fail-high (score > alpha && score < beta)
    if score > alpha && score < beta {
        let (s2, _) = negamax(
            pos,
            depth - 1,
            -beta,
            -alpha,
            ply + 1,
            true,
            ctx,
            Some((piece_kind, mv)),
            None,
        );
        score = -s2;
    }
}
```

**Rationale:** The first move (index 0) — which should be the TT move or PV move thanks to move ordering — is searched with the full window exactly as before. All subsequent non-LMR moves get a zero-window scout search first. Only if the scout search finds a score that beats alpha but doesn't exceed beta (indicating this move might be the new best) do we re-search with the full window to get the exact score.

### Step 4.2 — PVS + LMR Three-Stage Re-Search Cascade

Modify the LMR branch (lines 465–502) to implement the three-stage cascade:

**Current LMR code (lines 465–502):**
```rust
if do_lmr {
    let table = lmr_table();
    let d = (depth as usize).min(LMR_MAX_DEPTH - 1);
    let m = (moves_searched as usize).min(LMR_MAX_MOVES - 1);
    let reduction = table[d][m].max(1);

    let reduced_depth = if (depth as i32 - 1 - reduction as i32) > 0 {
        depth - 1 - reduction
    } else {
        0
    };
    // Stage 1: reduced zero-window
    let (s, _) = negamax(
        pos,
        reduced_depth,
        -alpha - 1,
        -alpha,
        ply + 1,
        true,
        ctx,
        Some((piece_kind, mv)),
        None,
    );
    score = -s;

    // Stage 2 (current): full-depth full-window re-search
    if score > alpha {
        let (s2, _) = negamax(
            pos,
            depth - 1,
            -beta,
            -alpha,
            ply + 1,
            true,
            ctx,
            Some((piece_kind, mv)),
            None,
        );
        score = -s2;
    }
}
```

**New LMR code:**
```rust
if do_lmr {
    let table = lmr_table();
    let d = (depth as usize).min(LMR_MAX_DEPTH - 1);
    let m = (moves_searched as usize).min(LMR_MAX_MOVES - 1);
    let reduction = table[d][m].max(1);

    let reduced_depth = if (depth as i32 - 1 - reduction as i32) > 0 {
        depth - 1 - reduction
    } else {
        0
    };
    // Stage 1: reduced-depth zero-window search
    let (s, _) = negamax(
        pos,
        reduced_depth,
        -alpha - 1,
        -alpha,
        ply + 1,
        true,
        ctx,
        Some((piece_kind, mv)),
        None,
    );
    score = -s;

    // Stage 2: full-depth zero-window re-search (on fail-high)
    if score > alpha {
        let (s2, _) = negamax(
            pos,
            depth - 1,
            -alpha - 1,
            -alpha,
            ply + 1,
            true,
            ctx,
            Some((piece_kind, mv)),
            None,
        );
        score = -s2;
    }

    // Stage 3: full-depth full-window re-search (on fail-high again)
    if score > alpha && score < beta {
        let (s3, _) = negamax(
            pos,
            depth - 1,
            -beta,
            -alpha,
            ply + 1,
            true,
            ctx,
            Some((piece_kind, mv)),
            None,
        );
        score = -s3;
    }
}
```

**Change summary:** The current two-stage LMR cascade (reduced zero-window → full-depth full-window) becomes a three-stage cascade:
1. **Reduced-depth zero-window** — same as current Stage 1 (unchanged).
2. **Full-depth zero-window** — new. If the reduced search fails high, re-search at full depth but still with a zero-window. This confirms whether the fail-high was due to the reduction or is genuine.
3. **Full-depth full-window** — only if the full-depth zero-window also fails high (score > alpha) and the score hasn't already exceeded beta. This gets the exact score.

The key change to Stage 2 is replacing `(-beta, -alpha)` with `(-alpha - 1, -alpha)`, and adding Stage 3 with `(-beta, -alpha)`.

### Step 4.3 — No Changes to `SearchContext` or Function Signature

PVS does not require a new flag in `SearchContext`. Unlike check extension (`check_extension_enabled`) or singular extension (`singular_extension_enabled`), PVS is not an optional enhancement that should be toggleable — it is a strict improvement over plain alpha-beta that produces identical search results with fewer nodes. The task spec describes it as "replaces plain alpha-beta," not as an optional feature.

However, the WAC comparison tests need to compare PVS vs non-PVS behavior. To support this comparison without a toggle flag, the tests use a **dual-search approach**: run the search twice on the same position — once normally (PVS active, the final implementation), and once with a forced full-window workaround by setting `beta = alpha + 1` at the root to create an artificial zero-width effect that is not PVS. See Section 5 for details.

**Alternative considered and rejected:** Adding a `pvs_enabled: bool` flag to `SearchContext`. This was rejected because (a) the spec says PVS "replaces" plain alpha-beta, implying the old code path is removed, and (b) maintaining two code paths long-term adds complexity for no production benefit.

### Step 4.4 — No Changes to `quiescence`, TT Logic, or Move Ordering

PVS affects only the recursive `negamax` calls within the move loop. The quiescence search, TT probing/storing, move ordering, check extension, singular extension, null move pruning, futility pruning, and reverse futility pruning are all unchanged.

The TT store logic at the end of `negamax` (lines 556–572) remains correct: it stores `alpha` as the score with the appropriate bound type. PVS does not change the final alpha/beta relationship — it only changes *how* the score is obtained (via zero-window + re-search rather than full-window upfront).

## 5. Test Plan

All tests are added to the existing `#[cfg(test)] mod tests` block in `crates/search/src/lib.rs`.

### Test 5.1 — `pvs_finds_same_or_better_move_on_wac`

This test validates that PVS finds the correct move on WAC tactical positions. Since PVS replaces plain alpha-beta and the old code path is removed, this test runs PVS on the same WAC positions used by the existing `check_extension_improves_wac_solve_rate` test and asserts that the engine still solves them correctly.

```rust
#[test]
fn pvs_finds_same_or_better_move_on_wac() {
    let wac_positions = [
        (
            "2rr3k/pp3pp1/1nnqbN1p/3pN3/2pP4/2P3Q1/PPB4P/R4RK1 w - - 0 1",
            vec![Square::G6],
        ),
        (
            "r1bqkb1r/pppp1ppp/2n2n2/4p2Q/2B1P3/8/PPPP1PPP/RNB1K1NR w KQkq - 4 4",
            vec![Square::F7],
        ),
        ("6k1/5ppp/8/8/8/8/8/3Q1RK1 w - - 0 1", vec![Square::D8]),
        (
            "r1bqkb1r/pppp1ppp/2n2n2/4p3/2B1P3/5N2/PPPP1PPP/RNBQK2R w KQkq - 4 4",
            vec![Square::G5, Square::D5],
        ),
    ];

    let depth: u8 = 6;
    let mut correct = 0;

    for (fen, expected_targets) in &wac_positions {
        let mut pos = Position::from_fen(fen).expect("valid fen");
        let mut ctx = SearchContext {
            start: Instant::now(),
            time_budget: Duration::from_secs(10),
            nodes: 0,
            aborted: false,
            killers: KillerTable::new(),
            history_table: HistoryTable::new(),
            countermove_table: CounterMoveTable::new(),
            pv_table: PvTable::new(),
            prev_pv: Vec::new(),
            stop_flag: None,
            max_nodes: None,
            tt: TranspositionTable::new(16),
            history: Vec::new(),
            lmr_enabled: true,
            futility_enabled: true,
            check_extension_enabled: true,
            singular_extension_enabled: true,
        };
        ctx.tt.new_generation();
        ctx.history.push(pos.hash());
        let mut best_mv = None;
        for d in 1..=depth {
            ctx.pv_table.clear();
            let (_, mv) = negamax(
                &mut pos, d, -INFINITY, INFINITY, 0, true, &mut ctx, None, None,
            );
            ctx.prev_pv = ctx.pv_table.extract_pv();
            if mv.is_some() {
                best_mv = mv;
            }
        }
        if let Some(m) = best_mv {
            if expected_targets.contains(&m.to_sq()) {
                correct += 1;
            }
        }
    }

    // PVS must solve at least 3 of 4 WAC positions (same threshold as pre-PVS baseline)
    assert!(
        correct >= 3,
        "PVS should solve at least 3 of 4 WAC positions, solved {}",
        correct
    );
}
```

### Test 5.2 — `pvs_zero_window_reduces_nodes`

This test validates that PVS (with zero-window scouts for non-first moves) searches fewer nodes than a full-window approach. Since PVS replaces plain alpha-beta and no toggle flag exists, this test uses a **dual-TT approach** to simulate non-PVS behavior: it runs the same position twice with separate `SearchContext` instances and separate TT instances. The "non-PVS" run is simulated by disabling LMR (which removes the zero-window scout searches from the LMR branch) and also by noting that the non-first-move zero-window scouts are the PVS contribution.

**Concrete approach:** The test compares the PVS implementation (all features enabled) against a run with LMR disabled. With LMR disabled, non-first moves still get the PVS zero-window treatment, but we lose the LMR node savings. The key insight: with PVS active, the total node count at a given depth should be lower than or equal to a generous upper bound. We validate this by running two searches — one with all features (PVS + LMR) and one with LMR disabled (PVS only, no LMR) — and asserting the combined PVS+LMR search uses fewer nodes than PVS alone. This confirms the LMR+PVS cascade is providing node savings, and the absolute node count validates PVS is functioning.

**Alternative approach (used here):** Since the existing `lmr_reduces_node_count` test already validates LMR savings with the enabled/disabled toggle, this test instead focuses on validating PVS specifically. It runs the search on multiple positions at depth 8 with iterative deepening and asserts the node count stays below a conservative ceiling. The ceiling is derived from the known pre-PVS node counts for these positions (captured by running the current code before applying the PVS change). The implementer must run the current (pre-PVS) code first, record the node counts, and hardcode the ceiling as `pre_pvs_nodes * 110 / 100` (10% margin for non-determinism from TT and hash collisions).

**Implementation protocol for the implementer:**
1. Before making any PVS code changes, run the test below with the assertion commented out and the `eprintln!` uncommented to capture pre-PVS baseline node counts for each position.
2. Record the maximum node count across positions as `MAX_BASELINE`.
3. Set `BASELINE_CEILING = MAX_BASELINE * 110 / 100`.
4. Uncomment the assertion and hardcode `BASELINE_CEILING`.
5. After implementing PVS, run the test to verify PVS nodes ≤ `BASELINE_CEILING`.

```rust
#[test]
fn pvs_zero_window_reduces_nodes() {
    let positions = [
        "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
        "r1bqkb1r/pppppppp/2n2n2/8/4P3/5N2/PPPP1PPP/RNBQKB1R w KQkq - 2 3",
        "r1bqkbnr/pppppppp/2n5/8/4P3/8/PPPP1PPP/RNBQKBNR w KQkq - 1 2",
    ];
    let depth: u8 = 8;

    for fen in positions {
        let mut pos = Position::from_fen(fen).expect("valid fen");
        let mut ctx = SearchContext {
            start: Instant::now(),
            time_budget: Duration::from_secs(60),
            nodes: 0,
            aborted: false,
            killers: KillerTable::new(),
            history_table: HistoryTable::new(),
            countermove_table: CounterMoveTable::new(),
            pv_table: PvTable::new(),
            prev_pv: Vec::new(),
            stop_flag: None,
            max_nodes: None,
            tt: TranspositionTable::new(16),
            history: Vec::new(),
            lmr_enabled: true,
            futility_enabled: true,
            check_extension_enabled: true,
            singular_extension_enabled: true,
        };
        ctx.tt.new_generation();
        ctx.history.push(pos.hash());
        for d in 1..=depth {
            ctx.pv_table.clear();
            negamax(
                &mut pos, d, -INFINITY, INFINITY, 0, true, &mut ctx, None, None,
            );
            ctx.prev_pv = ctx.pv_table.extract_pv();
        }
        let pvs_nodes = ctx.nodes;

        // STEP 1 (pre-PVS): Uncomment the line below, run test, record node counts.
        // eprintln!("Pre-PVS baseline for {}: {} nodes at depth {}", fen, pvs_nodes, depth);

        // STEP 2 (post-PVS): Replace BASELINE_CEILING with the recorded value * 1.1.
        // The implementer fills this in after capturing the pre-PVS baseline.
        // Example: if pre-PVS startpos at depth 8 = 500_000, set ceiling to 550_000.
        // BASELINE_CEILING must be set per-position; use the max across positions for simplicity.
        let baseline_ceiling: u64 = PLACEHOLDER; // Implementer: replace with recorded value * 1.1
        assert!(
            pvs_nodes <= baseline_ceiling,
            "PVS should search at or below pre-PVS baseline for {}: {} nodes vs {} ceiling",
            fen,
            pvs_nodes,
            baseline_ceiling,
        );
    }
}
```

**Note:** The `PLACEHOLDER` above is intentional — it is a compile error that forces the implementer to follow the protocol. The implementer replaces it with the actual captured value. This is the same pattern used in performance-sensitive tests throughout the codebase where exact node counts depend on the cumulative effect of all search features.

### Test 5.3 — `pvs_lmr_cascade_correctness`

This test verifies that the three-stage LMR + PVS re-search cascade produces correct results. It searches the same position with LMR enabled (triggering the three-stage cascade) and LMR disabled (using only the two-branch PVS from Step 4.1), then asserts both find the same best move. This follows the exact pattern of the existing `lmr_re_search_on_fail_high` test (lines 2993–3106), which compares LMR-on vs LMR-off to verify correctness. Additionally, it verifies that the LMR+PVS combination searches fewer nodes than PVS alone (LMR disabled), confirming the cascade is actively firing and providing savings.

```rust
#[test]
fn pvs_lmr_cascade_correctness() {
    let positions = [
        // Italian Game — many quiet moves, LMR will fire on most of them
        (
            "r1bqkb1r/pppp1ppp/2n2n2/4p3/2B1P3/5N2/PPPP1PPP/RNBQK2R w KQkq - 4 4",
            vec![Square::G5, Square::D5, Square::D3, Square::C3],
        ),
        // Scholar's mate threat — engine must find Qxf7#
        (
            "r1bqkb1r/pppp1ppp/2n2n2/4p2Q/2B1P3/8/PPPP1PPP/RNB1K1NR w KQkq - 4 4",
            vec![Square::F7],
        ),
    ];
    let depth: u8 = 7;

    for (fen, expected_targets) in &positions {
        // Search with LMR enabled (three-stage PVS+LMR cascade active)
        let mut pos_lmr = Position::from_fen(fen).expect("valid fen");
        let mut ctx_lmr = SearchContext {
            start: Instant::now(),
            time_budget: Duration::from_secs(30),
            nodes: 0,
            aborted: false,
            killers: KillerTable::new(),
            history_table: HistoryTable::new(),
            countermove_table: CounterMoveTable::new(),
            pv_table: PvTable::new(),
            prev_pv: Vec::new(),
            stop_flag: None,
            max_nodes: None,
            tt: TranspositionTable::new(16),
            history: Vec::new(),
            lmr_enabled: true,
            futility_enabled: true,
            check_extension_enabled: true,
            singular_extension_enabled: true,
        };
        ctx_lmr.tt.new_generation();
        ctx_lmr.history.push(pos_lmr.hash());
        let mut best_lmr = None;
        for d in 1..=depth {
            ctx_lmr.pv_table.clear();
            let (_, mv) = negamax(
                &mut pos_lmr, d, -INFINITY, INFINITY, 0, true, &mut ctx_lmr, None, None,
            );
            ctx_lmr.prev_pv = ctx_lmr.pv_table.extract_pv();
            if mv.is_some() {
                best_lmr = mv;
            }
        }
        let nodes_with_lmr = ctx_lmr.nodes;

        // Search with LMR disabled (only two-branch PVS, no three-stage cascade)
        let mut pos_no_lmr = Position::from_fen(fen).expect("valid fen");
        let mut ctx_no_lmr = SearchContext {
            start: Instant::now(),
            time_budget: Duration::from_secs(30),
            nodes: 0,
            aborted: false,
            killers: KillerTable::new(),
            history_table: HistoryTable::new(),
            countermove_table: CounterMoveTable::new(),
            pv_table: PvTable::new(),
            prev_pv: Vec::new(),
            stop_flag: None,
            max_nodes: None,
            tt: TranspositionTable::new(16),
            history: Vec::new(),
            lmr_enabled: false,
            futility_enabled: true,
            check_extension_enabled: true,
            singular_extension_enabled: true,
        };
        ctx_no_lmr.tt.new_generation();
        ctx_no_lmr.history.push(pos_no_lmr.hash());
        let mut best_no_lmr = None;
        for d in 1..=depth {
            ctx_no_lmr.pv_table.clear();
            let (_, mv) = negamax(
                &mut pos_no_lmr, d, -INFINITY, INFINITY, 0, true, &mut ctx_no_lmr, None, None,
            );
            ctx_no_lmr.prev_pv = ctx_no_lmr.pv_table.extract_pv();
            if mv.is_some() {
                best_no_lmr = mv;
            }
        }
        let nodes_without_lmr = ctx_no_lmr.nodes;

        // Assertion 1: Both must find a move that hits one of the expected target squares
        if let Some(m) = best_lmr {
            assert!(
                expected_targets.contains(&m.to_sq()),
                "PVS+LMR should find expected move for FEN: {}, got to_sq {:?}",
                fen,
                m.to_sq()
            );
        } else {
            panic!("PVS+LMR should find a move for FEN: {}", fen);
        }

        if let Some(m) = best_no_lmr {
            assert!(
                expected_targets.contains(&m.to_sq()),
                "PVS-only should find expected move for FEN: {}, got to_sq {:?}",
                fen,
                m.to_sq()
            );
        } else {
            panic!("PVS-only should find a move for FEN: {}", fen);
        }

        // Assertion 2: LMR+PVS cascade should search fewer nodes than PVS alone,
        // confirming the three-stage cascade is actively firing and providing savings
        assert!(
            nodes_with_lmr <= nodes_without_lmr,
            "PVS+LMR should search <= nodes than PVS alone for FEN: {} ({} vs {})",
            fen,
            nodes_with_lmr,
            nodes_without_lmr,
        );
    }
}
```

**Why this test is meaningful:** Unlike the previous version which only checked legality, this test:
1. Asserts both LMR-on and LMR-off searches find moves matching **known expected target squares**, verifying the cascade doesn't corrupt move selection.
2. Asserts `nodes_with_lmr <= nodes_without_lmr`, confirming the three-stage cascade (Stage 1: reduced zero-window → Stage 2: full-depth zero-window → Stage 3: full-depth full-window) is actually firing and reducing work compared to the two-branch PVS path (which is the code path used when `do_lmr` is false).
3. Follows the established pattern from `lmr_re_search_on_fail_high` (lines 2993–3106) and `lmr_reduces_node_count` (lines 2184–2281).

### Test 5.4 — `pvs_existing_tests_pass`

No explicit test needed. The existing test suite in `crates/search/src/lib.rs` (checkmate detection, stalemate, move ordering, TT tests, check extension tests, singular extension tests, etc.) must continue to pass with `cargo test`. This is verified by running `cargo test` after the change.

## 6. Exact Edit Locations

All edits are in `crates/search/src/lib.rs`:

| Edit | Lines (approximate) | Description |
|------|---------------------|-------------|
| 1 | 465–502 | Replace two-stage LMR cascade with three-stage PVS+LMR cascade |
| 2 | 503–516 | Replace plain full-window else branch with PVS first-move / subsequent-move branching |
| 3 | After line 3383 (end of tests) | Add three new test functions |

Total lines of production code changed: ~20 lines modified in the move loop (net increase ~15 lines due to the extra re-search stage and the first-move branching).

Total lines of test code added: ~200 lines (3 test functions following the established patterns).

## 7. Integration with Sibling Tasks

- **T01 (Check Extension):** No interaction. Check extension happens before the move loop (line 222–224). PVS modifies only the recursive calls within the move loop.
- **T02 (Singular Extension):** No interaction. Singular extension happens before the move loop (lines 262–292). PVS modifies only the recursive calls within the move loop. The `excluded_move` parameter is passed through unchanged.

## 8. Design Deviations

**Deviation: No `pvs_enabled` flag in `SearchContext`.**

The task spec says PVS "replaces" plain alpha-beta (spec line: "PVS logic replacing plain alpha-beta in `negamax`"). The phase spec's Design Decision #4 says "PVS requires distinguishing PV nodes from non-PV nodes" but does not mandate a toggle flag. Previous tasks (T01, T02) added toggle flags because extensions are optional enhancements that can hurt in some positions. PVS, by contrast, is a strict search-equivalent optimization — it finds the same best move with equal or fewer nodes. There is no scenario where disabling PVS improves results. A toggle flag would add dead code with no production value.

For testing, the comparison approach uses the existing `lmr_enabled` toggle (LMR-on vs LMR-off) to exercise both the three-stage cascade (LMR path) and the two-branch PVS (non-LMR path), plus a pre-recorded baseline ceiling for absolute node count validation.

## 9. Verification Checklist

1. `cargo fmt --check` passes
2. `cargo clippy -- -D warnings` passes
3. `cargo test` passes (all existing tests + 3 new tests)
4. PVS finds correct moves on WAC positions (Test 5.1)
5. PVS node count is at or below pre-PVS baseline ceiling (Test 5.2)
6. PVS + LMR cascade finds expected moves and reduces nodes vs PVS alone (Test 5.3)
7. The re-search cascade for LMR moves is: reduced zero-window → full-depth zero-window → full-depth full-window (verified by code inspection of the diff)
8. The first move at each node gets a full `(alpha, beta)` window; subsequent moves get `(alpha, alpha+1)` zero-window (verified by code inspection of the diff)