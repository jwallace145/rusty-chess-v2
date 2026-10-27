# rusty-chess on Lichess

Play the engine, or watch it play, on Lichess through a bot account. The
bridge is [lichess-bot](https://github.com/lichess-bot-devs/lichess-bot), the
standard program that connects a UCI engine to Lichess's bot API; the engine
itself is unchanged. `run.sh` sets everything up under `lichess/.lichess-bot/`.

## One-time setup

1. **Create a bot account.** Register a new Lichess account for the bot (for
   example `rusty-chess-bot`). It must never have played a game; once
   upgraded to a bot it cannot play as a human again.
2. **Create a token.** Logged in as that account, go to
   https://lichess.org/account/oauth/token/create?scopes[]=bot:play&description=rusty-chess
   and copy the token (it starts with `lip_`). Keep it out of the repo.
3. **Upgrade the account to a bot** (once):

   ```sh
   LICHESS_TOKEN=lip_... lichess/run.sh --upgrade
   ```

## Playing

```sh
LICHESS_TOKEN=lip_... lichess/run.sh
```

This builds the engine in release mode, fetches lichess-bot at the pinned
commit (it publishes no release tags), installs it into a virtual environment, copies the binary and the
config, and starts accepting challenges. Leave it running; press Ctrl-C to
stop (games in progress are resigned by the server after `abort_time`).

- **Watch or play:** open `https://lichess.org/@/<bot-username>` and send a
  challenge (standard chess, bullet to rapid, casual or rated). Spectators see
  the games live on the bot's profile.
- **Let it find opponents:** set `matchmaking.allow_matchmaking: true` in
  `config.yml` and it challenges other bots within 300 rating points while idle.
- **Games are saved** as PGN under `lichess/.lichess-bot/game_records/`.

Delete `lichess/.lichess-bot/` to refetch everything.

`config.yml` is versioned; the token is injected from the environment and
never written to the repo. The engine's hash size is set there (`Hash: 128`).

## Where it runs

Anywhere with Rust, Python 3.10 or newer and a network: this laptop while it is awake, or a
small always-on box later (Phase 5). The bot only plays while `run.sh` is
running.

## Rating

Lichess gives the bot a rating after enough games: the real-world number the
roadmap's Phase 5 asks for, next to the Stockfish-ladder estimate each release
records.
