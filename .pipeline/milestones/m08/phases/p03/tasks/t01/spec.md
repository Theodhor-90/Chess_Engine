# T01 — CuteChess-CLI Integration

## Objective

Build an automated tournament runner that invokes CuteChess-CLI to play matches between the chess engine and at least 3 reference engines (Stockfish at reduced depth, Ethereal, Rustic), producing Elo estimates with 95% confidence intervals of ±30 or narrower from at least 500 games per engine pair.

## Deliverables

- A script or binary under `tools/tournament/` that configures and invokes CuteChess-CLI for engine-vs-engine tournaments
- Configuration for at least 3 reference engines: Stockfish at reduced depth, Ethereal, and Rustic
- Elo estimation with 95% confidence interval output parsed from tournament results
- Demonstration of ±30 or narrower confidence intervals from at least 500 games per engine pair

## Key Implementation Details

- CuteChess-CLI is the external tool that drives engine-vs-engine matches via UCI
- The integration layer consists of scripts and configuration that invoke CuteChess-CLI with appropriate arguments: engine paths, time controls, opening books/positions, concurrency settings, and game counts
- Parse CuteChess-CLI PGN and log output to extract Elo estimates and confidence intervals
- Reference engines (Stockfish, Ethereal, Rustic) are downloaded during setup rather than committed to the repo — keep the repository lightweight while ensuring reproducible tournament conditions
- Tournament and SPRT tools are placed under `tools/` consistent with the master plan project structure
- Stockfish should be configured at reduced depth/nodes to provide a meaningful opponent at the engine's strength level

## Files to Create or Modify

- `tools/tournament/` — new directory containing the tournament runner script/binary
- Configuration files for engine paths, time controls, and opening sets

## Dependencies

- Phase P01 (Test Infrastructure) must be complete: the self-play framework provides game-playing infrastructure
- Phase P02 (Parameter Tuning) must be complete: the tuned evaluation and search parameters are what get measured for strength
- External tools required: CuteChess-CLI binary, reference engine binaries (Stockfish, Ethereal, Rustic)
- No dependencies on other tasks within this phase

## Verification Criteria

1. The tournament runner can be invoked from the command line and successfully launches CuteChess-CLI matches
2. Matches run against all 3 reference engines (Stockfish at reduced depth, Ethereal, Rustic)
3. Tournament output is parsed to produce Elo estimates with 95% confidence intervals
4. Running at least 500 games per engine pair produces confidence intervals of ±30 or narrower
5. The tool handles configuration of time controls, opening positions, concurrency, and game counts