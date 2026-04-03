Now I have comprehensive understanding of the project. Let me produce the implementation plan.

## 1. Deliverables

| File | Purpose |
|------|---------|
| `lichess-bot/config.yml` | `lichess-bot` bridge configuration file specifying the engine binary path, accepted time controls, challenge acceptance rules, and UCI options |
| `lichess-bot/README.md` | Documentation for setting up the Lichess bot account, obtaining an API token, installing `lichess-bot`, and running the bot |

Both files live in a `lichess-bot/` directory at the repository root, providing a self-contained deployment package.

## 2. Dependencies

- **External tool**: [`lichess-bot`](https://github.com/lichess-bot-devs/lichess-bot) — Python-based bridge between the Lichess API and UCI engines. Installed via `pip install lichess-bot` or by cloning the repository.
- **Python 3.8+**: Required by the `lichess-bot` bridge.
- **Lichess bot account**: A Lichess account upgraded to bot status via the Lichess API.
- **Lichess API token**: A personal access token with `bot:play` scope, obtained from https://lichess.org/account/oauth/token.
- **Engine binary**: The compiled engine binary from Phase P01's CI build (or a local `cargo build --release -p engine`).

No new Rust crates or `Cargo.toml` changes are required — this task is entirely configuration and documentation.

## 3. Implementation Details

### `lichess-bot/config.yml`

**Purpose**: Configuration file for the `lichess-bot` bridge that tells it how to launch the UCI engine and which challenges to accept.

**Key sections**:

- **`token`**: Placeholder `"xxxxxxxxxxxxxxxxxxxxxxxx"` — the Lichess API token. A comment directs users to set this via the `LICHESS_BOT_TOKEN` environment variable or directly in the file.
- **`engine`**:
  - `dir: "./"`  — directory containing the engine binary (users copy or symlink the engine here)
  - `name: "engine"` — executable name matching the Cargo binary output
  - `protocol: "uci"` — the engine communicates via UCI
  - `uci_options` — engine-specific UCI options:
    - `EvalFile: "nn.nnue"` — path to the NNUE weight file relative to the engine directory
    - `EvalMode: "nnue"` — use NNUE evaluation
    - `BookFile: ""` — no opening book by default (can be configured)
    - `SyzygyPath: ""` — no tablebase by default (can be configured)
- **`challenge`**:
  - `concurrency: 1` — play one game at a time
  - `sort_by: "best"` — prioritize highest-rated challengers
  - `accept_bot: true` — accept challenges from other bots
  - `accept_human: true` — accept challenges from humans
  - `min_rating: 0` — no minimum rating filter
  - `max_rating: 4000` — no maximum rating filter
  - `variants` — accept `standard` only (no variants like crazyhouse or antichess)
  - `time_controls` — accept `bullet`, `blitz`, `rapid`, and `classical`
  - `modes` — accept both `casual` and `rated` games

### `lichess-bot/README.md`

**Purpose**: Step-by-step guide for deploying the engine as a Lichess bot.

**Sections**:

1. **Prerequisites** — Python 3.8+, a compiled engine binary, a Lichess account
2. **Create a Lichess Bot Account** — Steps to create an account and upgrade it to bot status via the API (`curl` command using the Lichess API endpoint `POST /api/bot/account/upgrade`)
3. **Obtain an API Token** — Instructions to generate a token with `bot:play` scope at https://lichess.org/account/oauth/token
4. **Install lichess-bot** — `pip install lichess-bot` or clone from the repository
5. **Configure** — Copy the engine binary and `nn.nnue` into the `lichess-bot/` directory, set the token in `config.yml` or via environment variable
6. **Run the Bot** — Command to start: `lichess-bot -c config.yml`
7. **Verify** — How to confirm the bot is online: check the Lichess profile page, send a challenge, confirm the bot plays and responds

## 4. API Contracts

N/A — this task produces configuration files and documentation, not code with an API surface.

## 5. Test Plan

### Verification criteria from the task spec:

1. **Configuration file exists and is valid YAML**
   - Parse `lichess-bot/config.yml` with a YAML linter to confirm it is syntactically valid
   - Verify it contains required keys: `token`, `engine` (with `dir`, `name`, `protocol`, `uci_options`), `challenge` (with time control and acceptance settings)

2. **Engine binary path is correctly configured**
   - Verify the `engine.name` field matches the Cargo binary name (`engine`)
   - Verify the `engine.protocol` is `"uci"`

3. **Time controls and challenge acceptance rules are configured**
   - Verify `challenge.time_controls` includes bullet, blitz, rapid, and classical
   - Verify `challenge.variants` includes `standard`
   - Verify challenge acceptance settings (`accept_bot`, `accept_human`) are present

4. **Documentation covers bot account setup and API token**
   - Verify `lichess-bot/README.md` contains sections for creating a bot account, obtaining an API token, installing the bridge, configuring, and running

5. **Bot can connect to Lichess (deployment verification)**
   - This is verified manually: with a valid token and engine binary in place, running `lichess-bot -c config.yml` should authenticate with Lichess. The task spec acknowledges this is "verified once deployed" — the configuration is set up to produce a working connection.

### Test setup:
- YAML validation via `python3 -c "import yaml; yaml.safe_load(open('lichess-bot/config.yml'))"` (or equivalent)
- File existence checks for both deliverables

## 6. Implementation Order

1. **Create `lichess-bot/` directory** at the repository root
2. **Write `lichess-bot/config.yml`** — the complete bridge configuration with engine path, UCI options, time controls, and challenge acceptance rules
3. **Write `lichess-bot/README.md`** — deployment documentation covering account creation, token setup, bridge installation, configuration, running, and verification

## 7. Verification Commands

```bash
# Verify the directory and files exist
ls -la lichess-bot/config.yml lichess-bot/README.md

# Validate YAML syntax
python3 -c "import yaml; data = yaml.safe_load(open('lichess-bot/config.yml')); print('Valid YAML'); assert 'token' in data; assert 'engine' in data; assert 'challenge' in data; print('Required keys present')"

# Verify the config references the correct engine binary name
grep -q 'name.*engine' lichess-bot/config.yml && echo "Engine name configured"

# Verify time controls are configured
grep -q 'bullet' lichess-bot/config.yml && grep -q 'blitz' lichess-bot/config.yml && grep -q 'rapid' lichess-bot/config.yml && grep -q 'classical' lichess-bot/config.yml && echo "Time controls configured"

# Verify documentation covers key setup steps
grep -q 'bot:play' lichess-bot/README.md && echo "API token docs present"
grep -q 'lichess-bot' lichess-bot/README.md && echo "Bridge install docs present"

# Verify cargo project still compiles (no regressions)
cargo check -p engine
```

## 8. Design Deviations

**Deviation 1: Deliverable file placement**

- **Parent spec states**: "Lichess bot deployment configuration" as the deliverable, without specifying an exact directory structure.
- **Approach chosen**: Place configuration and documentation in a dedicated `lichess-bot/` directory at the repository root rather than scattering files across the project. This keeps the deployment artifacts self-contained — users can copy this directory alongside their engine binary to deploy. This follows the `lichess-bot` bridge's convention where configuration lives alongside the engine.

**Deviation 2: Verification of live Lichess connection**

- **Task spec requires**: "Bot connects to Lichess successfully" and "Bot accepts and plays games" and "Bot has a public rating on Lichess."
- **Why full live verification is not automatable in CI**: These criteria require a valid Lichess API token (a secret), a bot account, network access to Lichess servers, and a human or bot to issue a challenge. These are inherently manual deployment steps.
- **Alternative**: The configuration is written to produce a working connection when a valid token and engine binary are provided. The `lichess-bot/README.md` documents the exact verification steps a human operator performs. The automated verification commands validate that the configuration is syntactically correct and contains all required fields.