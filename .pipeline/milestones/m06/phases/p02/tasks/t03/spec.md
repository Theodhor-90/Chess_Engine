# T03 — Static Exchange Evaluation (SEE)

## Objective

Implement a `see` function that evaluates a capture sequence on a target square using bitboard attacker/defender sets and a least-valuable-attacker swap loop. Integrate SEE into move ordering (good captures above killers, bad captures below quiet moves) and into the search for bad capture pruning.

## Deliverables

- A new file `crates/search/src/see.rs` containing the `see` function
- Updated `crates/search/src/ordering.rs` — capture ordering uses SEE scores (good captures scored above killers, bad captures scored below quiet moves)
- Updated search loop to prune captures with negative SEE scores (bad capture pruning)
- Comprehensive unit tests covering all specified exchange scenarios

## Implementation Details

### SEE Algorithm
- Input: a position and a capture move (attacker square, target square)
- Uses bitboard attack/defend sets for the target square
- Iterates through attackers from least valuable to most valuable, simulating the capture exchange
- Produces a final material gain/loss score (positive = winning exchange, zero = equal, negative = losing)
- Must handle X-ray attacks (when a piece moves off a file/diagonal, it may reveal a sliding piece behind it)

### Swap Loop
1. Start with the initial capture's material gain
2. Find the least valuable attacker of the opposing side that can reach the target square
3. Simulate the recapture, updating the gain/loss
4. Continue alternating sides until no more attackers remain or a side chooses not to continue (stand-pat when continuing would lose material)
5. Propagate the scores backward through the sequence to determine the final evaluation

### SEE Dual Usage
1. **Capture ordering**: Replace or augment MVV-LVA for distinguishing good from bad captures. Good captures (SEE ≥ 0) scored above killers. Bad captures (SEE < 0) scored below quiet moves.
2. **Bad capture pruning**: In the search, skip captures with negative SEE scores to avoid wasting time on losing exchanges. Apply this in non-PV nodes or with appropriate conditions.

### Updated Move Ordering Hierarchy
Final ordering after all T01–T03 changes: TT move > PV move > good captures (positive SEE) > killer moves > counter-move > history-ordered quiet moves > bad captures (negative SEE)

## Required Unit Tests

The following specific scenarios must be tested:
1. **PxN** — pawn captures knight (winning, +220 approx.)
2. **NxB** — knight captures bishop (roughly equal, ~+10)
3. **BxR** — bishop captures rook (winning, +170 approx.)
4. **RxQ** — rook captures queen (winning, +400 approx.)
5. **PxN with NxP recapture** — pawn captures knight, knight recaptures (equal exchange)
6. **Undefended piece capture** — capturing a piece with no defenders (winning, full piece value)
7. **Multi-piece exchange** — e.g., PxN, BxP, RxB, QxR — net losing for the initiator
8. **Capture into a defended pawn chain** — losing exchange

## Dependencies

- **T01 (History Heuristic)** and **T02 (Counter-Move Heuristic)** — ordering infrastructure must be in place so SEE integrates into the final ordering hierarchy
- Depends on bitboard attack generation from M01/M02 (sliding piece attacks, pawn attacks, knight attacks, king attacks)
- Depends on piece values from M03 evaluation
- Depends on existing move ordering functions in `crates/search/src/ordering.rs`

## Verification Criteria

1. All 8 specified unit test scenarios pass with correct SEE scores
2. SEE is used for capture ordering — good captures (SEE ≥ 0) scored above killers, bad captures (SEE < 0) scored below quiet moves
3. SEE is used for bad capture pruning in the search
4. `cargo test` passes with no regressions
5. `cargo clippy -- -D warnings` and `cargo fmt --check` pass
6. No regression in solve rate on the WAC tactical test suite at fixed depth compared to the P01 baseline
7. The final move ordering hierarchy is: TT move > PV move > good captures (positive SEE) > killer moves > counter-move > history-ordered quiet moves > bad captures (negative SEE)