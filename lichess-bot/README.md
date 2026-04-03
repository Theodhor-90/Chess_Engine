# Lichess Bot Deployment

Deploy the chess engine as a Lichess bot using the [`lichess-bot`](https://github.com/lichess-bot-devs/lichess-bot) bridge.

## Prerequisites

- Python 3.8+
- A compiled engine binary (`cargo build --release -p engine`)
- A Lichess account (dedicated to the bot — cannot be reverted to a human account)

## Create a Lichess Bot Account

1. Create a new Lichess account at https://lichess.org/signup (or use an existing account that has never played a game).
2. Generate an API token (see next section).
3. Upgrade the account to bot status:

```bash
curl -d '' https://lichess.org/api/bot/account/upgrade \
  -H "Authorization: Bearer YOUR_TOKEN"
```

> **Warning**: Upgrading to a bot account is permanent. The account can no longer play games as a human.

## Obtain an API Token

1. Go to https://lichess.org/account/oauth/token.
2. Create a new personal access token with the **`bot:play`** scope.
3. Copy the token — you will need it for the configuration step.

## Install lichess-bot

```bash
pip install lichess-bot
```

Or clone the repository:

```bash
git clone https://github.com/lichess-bot-devs/lichess-bot.git
cd lichess-bot
pip install -r requirements.txt
```

## Configure

1. Copy the engine binary into the `lichess-bot/` directory:

```bash
cp target/release/engine lichess-bot/
```

2. Copy the NNUE weights file:

```bash
cp nn.nnue lichess-bot/
```

3. Set your API token in `config.yml` by replacing the placeholder:

```yaml
token: "your-actual-token-here"
```

Or set it via environment variable:

```bash
export LICHESS_BOT_TOKEN="your-actual-token-here"
```

4. (Optional) Configure additional UCI options in `config.yml`:
   - `BookFile`: path to a Polyglot opening book
   - `SyzygyPath`: path to Syzygy tablebase files

## Run the Bot

```bash
cd lichess-bot
lichess-bot -c config.yml
```

The bot will authenticate with Lichess and begin accepting challenges according to the configured rules.

## Verify

1. Check the bot's Lichess profile page — it should show as online.
2. Send a challenge to the bot from another account.
3. Confirm the bot accepts the challenge and plays the game to completion.
4. After a few rated games, the bot will obtain a public rating on Lichess.
