## Phase P02 — Documentation & Community

### Goal

Write the documentation needed for users and contributors, and connect the engine to Lichess for live play and a public rating. This phase ensures the engine is approachable for end users (via README and UCI option reference), understandable for contributors (via architecture documentation), and actively playing on Lichess.

### Deliverables

- **T01 — README**: Comprehensive README covering engine features, build instructions, UCI option reference, benchmark results, and current Elo estimates.
- **T02 — Architecture documentation**: Documentation of design decisions, core data structures (bitboards, position representation, move encoding, TT layout, NNUE architecture), and search algorithm explanations. Target audience is other engine authors and potential contributors.
- **T03 — Lichess Bot integration**: Engine deployed as a Lichess bot using the `lichess-bot` bridge. Configured accepted time controls and challenge acceptance rules. Verified that the bot connects, plays games, and obtains a public rating.

### Technical Decisions & Constraints

- README must cover: features, build instructions, UCI options, benchmark results, Elo estimates
- Architecture docs target engine authors and contributors — not end users; must explain *why* not just *what*
- Lichess Bot deployment uses the external `lichess-bot` bridge tool to connect the UCI engine to the Lichess API
- Time controls and challenge acceptance rules must be configurable
- Documentation should follow project conventions: `///` doc comments on public items, explain *why* not *what*
- The engine communicates via UCI — GUI is a non-goal (per MASTER_PLAN.md), but the bot bridge provides live play

### Dependencies

- **Phase P01 (Build & Distribution)** must be complete — the README references build instructions, release binaries, and versioning; architecture docs reference the final engine structure
- All prior milestones (M01–M10) must be complete — documentation covers the full engine
- `lichess-bot` bridge is an external dependency required for T03