# Local engine battles with EngineBattle

Play rusty-chess against other UCI engines on your own machine, watch the
games live on a board, and keep the PGNs. It uses
[EngineBattle](https://github.com/lepned/EngineBattle), a web GUI with a
console (`eb-cli`), installed as an external program and never vendored here.

> **Not a strength test.** These games run on whatever machine you happen to
> use, at whatever settings you pick, against whichever opponent. The record of
> strength is still the SPRT and Match workflows and the rating ladder, all on
> standardized hardware ([docs/roadmap.md](../../docs/roadmap.md)). Use this to
> watch the engine play, try an opponent, compare two builds by eye, or look at
> a position that went wrong.

## Setup

```sh
tools/engine-battle/eb.sh setup
```

On macOS (Apple silicon or Intel), with `curl`, `unzip`, `python3` and Rust,
this does the following in about half a minute:

1. **Downloads EngineBattle v1.13.0** (the self-contained build, so no .NET
   needed), Stockfish 19 and Viridithas 20. Viridithas is Apple silicon only.
   Every download is pinned to a release and checked against its SHA-256.
2. **Builds this working tree's rusty-chess** in release mode.
3. **Registers the engines.** EngineBattle starts each one over UCI and writes
   an engine definition from what it reports. Every engine gets one thread and
   64 MB of hash.

Everything lands in `tools/engine-battle/.local/`, which git ignores:

| Path | What it holds |
|:--|:--|
| `EngineBattle/` | the app |
| `engines/` | the binaries |
| `EngineDefs/` | the definitions |
| `tournaments/` | generated configs |
| `results/` | PGNs |

Delete the directory to start over. EngineBattle is GPL-3.0, Stockfish GPL-3.0,
Viridithas AGPL-3.0, and Lc0 and the Maia nets (from `bots`) GPL-3.0. They are downloaded for local use only, never
committed or redistributed.

## Engines

| Command | Registers |
|:--|:--|
| `eb.sh add-rusty` | this working tree's build, as `rusty-chess-dev-<sha>` (`-dirty` with uncommitted engine changes) |
| `eb.sh add-rusty --ref v0.30.0` | any tag, branch or commit, built in a temporary worktree, as `rusty-chess-v0.30.0` |
| `eb.sh add-rusty --net net1-l050-r41022503872` | a build with another evaluation, through the UCI `EvalFile` option: an embedded net's id, `pesto`, or a `.nnue` path |
| `eb.sh add-engine NAME PATH [--uci OPTION=VALUE ...] [--policy]` | any UCI executable, e.g. `--uci UCI_LimitStrength=true --uci UCI_Elo=2800`. `--policy` makes it play one node a move in tournaments, like Maia |
| `eb.sh list` | lists what's registered |

**Each Rusty Chess build is kept as its own binary,** so versions and nets
stay separate competitors and nothing in the engine's source changes to switch
between them.

**`setup` registers these opponents:**
- `stockfish-19`: full strength, far stronger than us;
- `stockfish-19-elo3000`: limited to `UCI_Elo` 3000, about our level;
- `viridithas-20`.

## Bots from about 1100 to 3500

```sh
tools/engine-battle/eb.sh bots
```

This registers a ladder of opponents for you, or for rusty-chess, to play,
in about a minute. It installs Lc0 with Homebrew if it's missing.

| Bot | Strength (approximate) | What it is |
|:--|:--|:--|
| `maia-1100` … `maia-1900` (9 levels) | Lichess 1100–1900 | [Maia](https://github.com/CSSLab/maia-chess) nets on [Lc0](https://lczero.org): trained on human Lichess games of each rating to make the moves such a player would. **Human-like, mistakes included.** One node a move |
| `stockfish-19-elo1500`, `-elo2000`, `-elo2500`, `-elo3000` | Stockfish's `UCI_Elo` 1500–3000 | Stockfish weakened to a target rating. It plays strongly and then blunders on purpose, which feels less human than Maia |
| `rusty-chess-v0.3.0` | about 1900 | our first search release (alpha-beta, PeSTO) |
| `rusty-chess-v0.9.0` | about 2180 | with the transposition table, move ordering and PVS |
| `rusty-chess-v0.11.0` | about 2350 | with null-move pruning and late move reductions |
| `rusty-chess-v0.17.0` | about 2600 | the end of Phase 4: the full classical search, still PeSTO |
| `rusty-chess-dev-…` | about 3000 | today's engine with its net |
| `viridithas-20`, `stockfish-19` | about 3500 and up | full strength: superhuman |

**Where the ratings come from:**
- **Maia** is labelled by the Lichess ratings it was trained on.
- **Stockfish** uses its own `UCI_Elo` scale.
- **rusty-chess** uses its release ladder ratings, against limited Stockfish.

These scales are related but not equal, so treat the numbers as a ladder, not
exact ratings.

**Maia must play at one node a move.** More search makes it stronger and less
human. Its definitions use time control 2, which every tournament from `play`
sets to one node, so Maia plays like that whatever the others play.

## Playing a bot yourself

1. **Start the GUI:** `tools/engine-battle/eb.sh gui`.
2. **Open the page:** Play & Analysis → **Play vs Computer**.
3. **Load the bot:** **Load Engine**, then pick its definition in
   `tools/engine-battle/.local/EngineDefs/`, for example `maia-1500.json` or
   `rusty-chess-v0.11.0.json`.
4. **Pick a time control:**
   - **for Maia:** Custom → **Nodes**, 1;
   - **for anything else:** a clock, for example 5+0 or 10+5.
5. **Play:** choose your colour and **Start Game**.

Take back, premoves, Save PGN and Game Review are all on that page.

## Playing

```sh
# Watch a 2+1 blitz match live: opens EngineBattle in the browser; press Ctrl+R to start
tools/engine-battle/eb.sh play --engines rusty-chess-dev-75017a5,stockfish-19-elo3000

# The same in the terminal, 4 games at a time
tools/engine-battle/eb.sh play --engines rusty-chess-dev-75017a5,stockfish-19-elo3000 --console --parallel 4

# A gauntlet (the first engine plays each of the others) at a fixed 100k nodes a move
tools/engine-battle/eb.sh play --mode gauntlet --nodes 100000 --rounds 20 \
  --engines rusty-chess-dev-75017a5,rusty-chess-v0.30.0,stockfish-19-elo3000,viridithas-20

# A round robin of three builds
tools/engine-battle/eb.sh play --mode rr --engines rusty-chess-v0.30.0,rusty-chess-dev-75017a5,rusty-chess-dev-75017a5-net1-l050-r41022503872
```

| Option | Default | |
|:--|:--|:--|
| `--engines A,B,...` | required | registered names (`eb.sh list`). In a gauntlet the first is the challenger |
| `--mode` | `match` | `match` (two engines), `rr` (round robin) or `gauntlet` |
| `--tc` | `2:00+1` | time control as base+increment: `2:00+1` is 2 minutes plus 1 second; `60+0.6` is 60 seconds plus 0.6 (a bare number is seconds) |
| `--nodes N` | | a fixed node budget per move instead of a clock, for controlled comparisons |
| `--rounds N` | 10 | rounds; each opening is played from both sides (paired), so 2 games a pair per round |
| `--parallel N` | 1 | games at once. Above 1, the GUI shows a board per game |
| `--console` | off | play in the terminal with `eb-cli`, no GUI |
| `--name NAME` | the engine names | names the tournament and its PGN |

**How a game is set up:**
- **Openings:** from `tests/openings/openings.epd`, the book the SPRT uses,
  each played with both colours.
- **Adjudication:** a game is adjudicated drawn when both engines' scores stay
  within ±0.10 for 5 moves after move 40, and won at ±10.00 for 5 moves.
- **Pondering:** off.

These defaults are in [`tournament.template.json`](tournament.template.json),
and EngineBattle's own settings page changes the rest.

### In the GUI

`play` writes the tournament into the app's `wwwroot/tournament.json` and starts
EngineBattle, which opens `http://localhost:5018`.

| Key | Action |
|:--|:--|
| **Ctrl+R** | starts the tournament (Ctrl+V validates it first) |
| **Ctrl+C** | stops it |
| **Ctrl+P** | shows the tournament view |

**During a game, the page shows:**
- the board and moves as they're played;
- each engine's evaluation, principal variation, depth, nodes and speed;
- charts of evaluation, speed and time.

**After the tournament:**
- the GUI's game review (Analysis and PGN tools) replays the games;
- Rusty Chess reports everything EngineBattle shows except selective depth
  (the PGN's `sd=0`).

`eb.sh gui` reopens the app with the last tournament.

### Results

**Where the games go:** every game is written as it finishes to
`.local/results/<date>-<name>.pgn`. After parallel games there's also a copy
sorted by round (`_ordered.pgn`).

**Each move's comment has the engine's search:**
`{wv=1.56, mt=672, s=2152630, n=1446568, d=16, pd=exf5, tl=9101, pv=...}`.
That's the evaluation in pawns, the move time in ms, nodes per second, nodes,
depth, the expected reply, the time left and the principal variation.

**Standings:** the console and the GUI print them, with Elo and pentanomial
statistics. With a handful of games their error bars are huge; that is what
the statistical pipeline is for.

**Checking the tournament file:** `eb-cli tournament validate <file>` checks a
generated tournament, and `eb.sh play` runs that check before every start.

## Updating

**EngineBattle or an engine:** change the version and SHA-256 at the top of
`eb.sh` (the release page lists each asset's digest), then run `eb.sh setup`
again. A changed EngineBattle version reinstalls the app. Engine definitions
and results are kept.

**Rusty Chess:** to pick up new engine changes, run `eb.sh add-rusty` again.
A new commit gets a new name, and older builds stay registered.
