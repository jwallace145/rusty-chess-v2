#!/usr/bin/env bash
# Run rusty-chess as a Lichess bot.
#
#   LICHESS_TOKEN=lip_... lichess/run.sh            # build, set up lichess-bot, play
#   LICHESS_TOKEN=lip_... lichess/run.sh --upgrade  # one-time: turn the account into a bot
#
# Needs: a Lichess account that has never played a game, with an API token
# that has the bot:play scope; Rust; Python 3.9+. Everything is set up under
# lichess/.lichess-bot/ (ignored by git).
set -euo pipefail

LICHESS_BOT_VERSION="${LICHESS_BOT_VERSION:-1.1.3}"
here="$(cd "$(dirname "$0")" && pwd)"
root="$(cd "$here/.." && pwd)"
work="$here/.lichess-bot"

: "${LICHESS_TOKEN:?set LICHESS_TOKEN to a Lichess API token with the bot:play scope}"

echo "==> building rusty-chess (release)"
(cd "$root" && cargo build --release --locked)

if [ ! -d "$work" ]; then
  echo "==> fetching lichess-bot $LICHESS_BOT_VERSION"
  git clone --quiet --depth 1 --branch "v$LICHESS_BOT_VERSION" https://github.com/lichess-bot-devs/lichess-bot.git "$work" 2>/dev/null \
    || git clone --quiet --depth 1 --branch "$LICHESS_BOT_VERSION" https://github.com/lichess-bot-devs/lichess-bot.git "$work"
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
  exec venv/bin/python lichess-bot.py --upgrade
fi
echo "==> playing; watch at https://lichess.org/@/<your-bot-username>"
exec venv/bin/python lichess-bot.py -v
