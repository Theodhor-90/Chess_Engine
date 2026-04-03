# T03 — Lichess Bot Integration

## Objective

Deploy the chess engine as a Lichess bot using the `lichess-bot` bridge, so it can play live games and obtain a public rating on Lichess.

## Deliverable

- Lichess bot deployment configuration (bridge config files)
- Verified bot that connects to Lichess, plays games, and obtains a public rating

## Implementation Details

### `lichess-bot` Bridge Setup
- Configure the `lichess-bot` bridge to run the chess engine binary via UCI protocol
- The engine already supports full UCI (from Milestone M04), so the bridge wraps the existing UCI interface — no Lichess API implementation needed in the engine itself

### Configuration Requirements
- **Engine binary path**: Point to the built engine binary
- **Accepted time controls**: Configure which time controls the bot accepts (bullet, blitz, rapid, classical)
- **Challenge acceptance rules**: Define rules for auto-accepting or filtering incoming challenges
- **Bot token**: Lichess API token with bot:play scope (documentation for obtaining this)

### Verification Steps
- Bot connects to Lichess successfully
- Bot accepts and plays games according to configured time controls
- Bot obtains a public rating on Lichess

## Constraints

- Uses the external `lichess-bot` bridge tool — do not implement the Lichess API directly
- The engine communicates via UCI protocol (already implemented in M04)
- Requires a Lichess bot account and API token

## Dependencies

- **T01 (README)** and **T02 (Architecture Documentation)**: No hard dependency, but T03 completes the phase
- Phase P01 (Build & Distribution) must be complete — the deployed binary comes from the build pipeline
- All prior milestones (M01–M10) must be complete — the bot runs the full engine
- External dependency: `lichess-bot` bridge tool

## Verification Criteria

1. `lichess-bot` bridge configuration exists with engine binary path, time controls, and challenge acceptance rules
2. Documentation for setting up the Lichess bot account and obtaining an API token
3. Bot can connect to Lichess (verified by successful authentication)
4. Bot accepts challenges and plays games according to configured time controls
5. Bot has a public rating on Lichess (or configuration is verified to produce one once deployed)