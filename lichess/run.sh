#!/usr/bin/env bash
# Run rusty-chess as a Lichess bot.
#
#   LICHESS_TOKEN=lip_... lichess/run.sh            # build, set up lichess-bot, play
#   LICHESS_TOKEN=lip_... lichess/run.sh --upgrade  # one-time: turn a fresh account into a bot
#
# Needs: a Lichess account that has never played a game, with an API token
# that has the bot:play scope; Rust; Python 3.10+. Everything is set up under
# lichess/.lichess-bot/ (ignored by git); delete that directory to refetch.
set -euo pipefail

# lichess-bot publishes no tags (its last, 1.1.3, is from 2018), so pin a
# master commit instead. Bump deliberately and re-check config.yml.default.
LICHESS_BOT_REF="${LICHESS_BOT_REF:-afa430da25cb6d506c878d3ca2fa155be8da625d}" # 2026-10-27
here="$(cd "$(dirname "$0")" && pwd)"
root="$(cd "$here/.." && pwd)"
work="$here/.lichess-bot"

: "${LICHESS_TOKEN:?set LICHESS_TOKEN to a Lichess API token with the bot:play scope}"

echo "==> building rusty-chess (release)"
(cd "$root" && cargo build --release --locked)

if [ ! -d "$work" ]; then
  echo "==> fetching lichess-bot at $LICHESS_BOT_REF"
  git init --quiet "$work"
  git -C "$work" fetch --quiet --depth 1 https://github.com/lichess-bot-devs/lichess-bot.git "$LICHESS_BOT_REF"
  git -C "$work" -c advice.detachedHead=false checkout --quiet FETCH_HEAD
fi

if [ ! -d "$work/venv" ]; then
  echo "==> creating the Python environment"
  python3 -m venv "$work/venv"
  "$work/venv/bin/pip" install --quiet --upgrade pip
  "$work/venv/bin/pip" install --quiet -r "$work/requirements.txt"
fi

mkdir -p "$work/engines"
cp "$root/target/release/rusty-chess" "$work/engines/rusty-chess"
sed "s|LICHESS_TOKEN_PLACEHOLDER|$LICHESS_TOKEN|" "$here/config.yml" > "$work/config.yml"

cd "$work"
if [ "${1:-}" = "--upgrade" ]; then
  echo "==> upgrading the account to a bot (irreversible for that account)"
  exec venv/bin/python lichess-bot.py -u
fi
echo "==> playing; watch at https://lichess.org/@/<your-bot-username>"
exec venv/bin/python lichess-bot.py -v
