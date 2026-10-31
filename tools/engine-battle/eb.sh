#!/usr/bin/env bash
# Play rusty-chess against other UCI engines locally in EngineBattle
# (https://github.com/lepned/EngineBattle), and watch the games live.
#
#   tools/engine-battle/eb.sh setup                     # install EngineBattle and two opponents, register them and this tree's rusty-chess
#   tools/engine-battle/eb.sh bots                      # add a ladder of opponents from about 1100 to 3000: Maia, Stockfish levels, old rusty-chess
#   tools/engine-battle/eb.sh add-rusty [--ref REF] [--net NET] [--name NAME]
#   tools/engine-battle/eb.sh add-engine NAME PATH [--uci OPTION=VALUE ...] [--policy]
#   tools/engine-battle/eb.sh list
#   tools/engine-battle/eb.sh play --engines A,B[,C...] [--mode match|rr|gauntlet] [--tc 2:00+1 | --nodes N]
#                                  [--rounds N] [--parallel N] [--name NAME] [--console]
#   tools/engine-battle/eb.sh gui
#
# A local experimentation and viewing tool, not a strength test: SPRT, Match
# and the rating ladder (on standardized hardware) stay the record. See
# tools/engine-battle/README.md.
#
# Everything downloaded or generated goes under tools/engine-battle/.local/
# (ignored by git): EngineBattle, the engines, their definitions, the
# tournaments and the PGNs. Delete it to start over. Needs macOS (Apple silicon
# or Intel), curl, unzip, python3 and Rust.
set -euo pipefail

# Pinned downloads: bump deliberately, with the new SHA-256 from the release.
EB_VERSION="v1.13.0"
EB_SHA256_ARM64="d6b4cf914e96fc1bd8d0a119800db88dc7bf917bd59219d3f626c79d13e5fc69"
EB_SHA256_X64="2e4ef7bf5cb2faee4bcf432e0cc93b1b70e492d173adf00923e66fc702bda45a"
SF_TAG="sf_19"
SF_SHA256="a1f0e3bcc5a6927a11fe6fc8e54a779754645f3c2bae2cf13420fd1957adaa77" # stockfish-macos-universal.tar.gz
VIR_TAG="v20.0.0"
VIR_SHA256="9ab84379f0241d94f926666eef8385ab032c585bb3055f27a3fbae423b75fd41" # viridithas-20-macos-aarch64 (Apple silicon only)
# Maia (https://github.com/CSSLab/maia-chess): Lc0 nets trained to play like
# Lichess players of each rating, at one node a move.
MAIA_COMMIT="1303068186418e38e2e1f93f058e8efd9efccefd"
maia_sha256() {
  case "$1" in
    1100) echo e1cf1cd0c96b8a4fa6a275f4b9fd54ed1ffebf9fe44641b9fceded310e9619c4 ;;
    1200) echo ead4ba953f233ae732999ebc1e2b675378148527ebcfad2f0acbc5e4c224d98e ;;
    1300) echo 36195f87bf4761834baa0bf87472b18509a7261a9d7d6f1a8443261369a733f2 ;;
    1400) echo d5353ea6766356dad2d28920c6692f37a5f30963767f1a3105d33b4d0af011e8 ;;
    1500) echo 35ab6f20421d59e1df3b17c5a5016947af4c6761368ef84044a9a9c7619a9a00 ;;
    1600) echo d2c9e5948581acf4b9fc0b1e720c5dc0fe64ce80cfc4a239d3f8a42e1176c876 ;;
    1700) echo d277eacd792d340a30abb464dc65127254e65cac57abca17facc469889b96478 ;;
    1800) echo 0031ad7c4256b1fd09fbebd28418d644d68b26cd2a45df4967ccf5c7ec9c4965 ;;
    1900) echo e2f565f42d7cd9f122557e6dc4eb84e5bbaedceda1d404dc485d3611c7c97a12 ;;
  esac
}
# Old releases worth playing, with their ladder ratings (release notes).
RUSTY_LADDER="v0.3.0 v0.9.0 v0.11.0 v0.17.0" # about 1900, 2180, 2350, 2600

here="$(cd "$(dirname "$0")" && pwd)"
root="$(cd "$here/../.." && pwd)"
local_dir="$here/.local"
app="$local_dir/EngineBattle"
engines="$local_dir/engines"
defs="$local_dir/EngineDefs"
results="$local_dir/results"
tournaments="$local_dir/tournaments"
downloads="$local_dir/downloads"

die() { echo "eb: $*" >&2; exit 1; }
need() { command -v "$1" >/dev/null || die "needs $1"; }

fetch() { # fetch URL FILE SHA256
  local url=$1 out=$2 sha=$3
  if [ -f "$out" ] && echo "$sha  $out" | shasum -a 256 -c - >/dev/null 2>&1; then return 0; fi
  echo "==> downloading $(basename "$url")"
  curl -fL --progress-bar -o "$out.part" "$url"
  echo "$sha  $out.part" | shasum -a 256 -c - >/dev/null || die "checksum mismatch for $url"
  mv "$out.part" "$out"
}

# Write a def for an executable: probe it with `eb-cli engine new`, then name
# it, give it one thread and 64 MB of hash where it has those options, and
# apply the --uci overrides.
register() { # register NAME PATH [OPTION=VALUE ...]
  local name=$1 path=$2; shift 2
  [ -x "$app/eb-cli" ] || die "run '$0 setup' first"
  [[ "$name" =~ ^[A-Za-z0-9._-]+$ ]] || die "engine names are letters, digits, '.', '_' and '-': $name"
  [ -x "$path" ] || die "not an executable: $path"
  local tmp
  tmp=$(mktemp -d)
  (cd "$app" && ./eb-cli engine new "$path" --out-dir "$tmp" >"$tmp/probe.log" 2>&1) \
    || { cat "$tmp/probe.log" >&2; rm -rf "$tmp"; die "EngineBattle could not probe $path over UCI"; }
  mkdir -p "$defs"
  python3 - "$tmp" "$defs/$name.json" "$name" "$@" <<'PY'
import glob, json, sys
tmp, out, name, overrides = sys.argv[1], sys.argv[2], sys.argv[3], sys.argv[4:]
src = [f for f in glob.glob(f"{tmp}/*.json")]
if len(src) != 1:
    sys.exit(f"expected one def from the probe, found {src}")
d = json.load(open(src[0], encoding="utf-8-sig"))
opts = d.setdefault("Options", {})
if "Threads" in opts:
    opts["Threads"] = 1
if "Hash" in opts:
    opts["Hash"] = 64
tcid = 1
for o in overrides:
    if o == "--policy":  # plays one node a move: time control 2 in every tournament
        tcid = 2
        continue
    k, _, v = o.partition("=")
    if k not in opts:
        sys.exit(f"{name}: the engine has no UCI option '{k}' (it has: {', '.join(opts)})")
    old = opts[k]
    opts[k] = (v.lower() == "true") if isinstance(old, bool) else int(v) if isinstance(old, int) else v
d["Version"] = d.get("Name", "")
d["Name"] = name
d["Alias"] = name
d["TimeControlID"] = tcid
json.dump(d, open(out, "w"), indent=2)
shown = ", ".join(f"{k}={v}" for k, v in opts.items() if k in ("Threads", "Hash", "UCI_LimitStrength", "UCI_Elo", "EvalFile", "WeightsFile"))
print(f"registered {name}: {d['Path']} ({shown}{', one node a move' if tcid == 2 else ''})")
PY
  rm -rf "$tmp"
  last_registered=$name
}
last_registered=""

cmd_setup() {
  [ "$(uname -s)" = Darwin ] || die "setup supports macOS; on another OS install EngineBattle by hand and use add-engine"
  need curl; need unzip; need python3; need cargo
  local arch asset sha
  arch=$(uname -m)
  case "$arch" in
    arm64) asset="EngineBattle-osx-arm64.zip"; sha=$EB_SHA256_ARM64 ;;
    x86_64) asset="EngineBattle-osx-x64.zip"; sha=$EB_SHA256_X64 ;;
    *) die "unsupported Mac: $arch" ;;
  esac
  mkdir -p "$downloads" "$engines" "$defs" "$results" "$tournaments"

  fetch "https://github.com/lepned/EngineBattle/releases/download/$EB_VERSION/$asset" "$downloads/$asset" "$sha"
  if [ "$(cat "$app/.version" 2>/dev/null)" != "$EB_VERSION" ]; then
    echo "==> installing EngineBattle $EB_VERSION"
    rm -rf "$app"; mkdir -p "$app"
    unzip -q "$downloads/$asset" -d "$app"
    echo "$EB_VERSION" > "$app/.version"
  fi

  fetch "https://github.com/official-stockfish/Stockfish/releases/download/$SF_TAG/stockfish-macos-universal.tar.gz" \
    "$downloads/stockfish-$SF_TAG.tar.gz" "$SF_SHA256"
  tar -xzf "$downloads/stockfish-$SF_TAG.tar.gz" -C "$downloads"
  cp "$downloads/stockfish/stockfish-macos-universal" "$engines/stockfish-19"
  if [ "$arch" = arm64 ]; then
    fetch "https://github.com/cosmobobak/viridithas/releases/download/$VIR_TAG/viridithas-20-macos-aarch64" \
      "$downloads/viridithas-$VIR_TAG" "$VIR_SHA256"
    cp "$downloads/viridithas-$VIR_TAG" "$engines/viridithas-20"
  fi
  chmod +x "$engines"/* "$app/EngineBattle" "$app/eb-cli"
  # The releases are unsigned; downloads via curl aren't quarantined, but a
  # browser download or an archive tool may add the flag.
  xattr -dr com.apple.quarantine "$app" "$engines" 2>/dev/null || true

  echo "==> registering the opponents"
  register stockfish-19 "$engines/stockfish-19"
  register stockfish-19-elo3000 "$engines/stockfish-19" UCI_LimitStrength=true UCI_Elo=3000
  [ -x "$engines/viridithas-20" ] && register viridithas-20 "$engines/viridithas-20"
  cmd_add_rusty
  echo
  echo "Ready. Try:  $0 play --engines $last_registered,stockfish-19-elo3000"
}

cmd_bots() {
  [ -x "$app/eb-cli" ] || die "run '$0 setup' first"
  if command -v brew >/dev/null; then
    brew list --formula lc0 >/dev/null 2>&1 || { echo "==> brew install lc0"; brew install --quiet lc0; }
  else
    echo "eb: no Homebrew: skipping Maia (it needs lc0)" >&2
  fi
  if command -v lc0 >/dev/null; then
    mkdir -p "$local_dir/nets"
    local elo f
    for elo in 1100 1200 1300 1400 1500 1600 1700 1800 1900; do
      f="$local_dir/nets/maia-$elo.pb.gz"
      fetch "https://raw.githubusercontent.com/CSSLab/maia-chess/$MAIA_COMMIT/maia_weights/maia-$elo.pb.gz" "$f" "$(maia_sha256 $elo)"
      register "maia-$elo" "$(command -v lc0)" "WeightsFile=$f" --policy
    done
  fi
  local elo
  for elo in 1500 2000 2500; do
    register "stockfish-19-elo$elo" "$engines/stockfish-19" UCI_LimitStrength=true "UCI_Elo=$elo"
  done
  local ref
  for ref in $RUSTY_LADDER; do
    git -C "$root" rev-parse --verify --quiet "$ref^{commit}" >/dev/null || git -C "$root" fetch --quiet --tags origin
    cmd_add_rusty --ref "$ref"
  done
  echo
  echo "Added. 'eb.sh list' shows everything; tools/engine-battle/README.md has the ladder with approximate ratings."
}

cmd_add_rusty() {
  local ref="" net="" name=""
  while [ $# -gt 0 ]; do
    case "$1" in
      --ref) ref=$2; shift 2 ;;
      --net) net=$2; shift 2 ;;
      --name) name=$2; shift 2 ;;
      *) die "add-rusty: unknown argument $1" ;;
    esac
  done
  need cargo
  mkdir -p "$engines"
  local label src
  if [ -z "$ref" ]; then
    echo "==> building rusty-chess from this working tree (release)"
    (cd "$root" && cargo build --release --locked --quiet)
    label="dev-$(git -C "$root" rev-parse --short HEAD)$(git -C "$root" diff --quiet HEAD -- crates src Cargo.toml Cargo.lock || echo "-dirty")"
    src="$root/target/release/rusty-chess"
  else
    git -C "$root" rev-parse --verify --quiet "$ref^{commit}" >/dev/null || die "no such ref: $ref (try git fetch --tags)"
    label=$(echo "$ref" | tr '/' '-')
    local wt="$local_dir/build/$label"
    echo "==> building rusty-chess $ref (release)"
    rm -rf "$wt"; git -C "$root" worktree prune
    git -C "$root" worktree add --quiet --detach "$wt" "$ref"
    (cd "$wt" && CARGO_TARGET_DIR="$local_dir/build/target" cargo build --release --locked --quiet)
    git -C "$root" worktree remove --force "$wt"
    src="$local_dir/build/target/release/rusty-chess"
  fi
  # A copy per build, so each registered version keeps its own binary.
  local exe="$engines/rusty-chess-$label"
  cp "$src" "$exe"
  if [ -n "$net" ]; then
    register "${name:-rusty-chess-$label-$(basename "$net" .nnue)}" "$exe" "EvalFile=$net"
  else
    register "${name:-rusty-chess-$label}" "$exe"
  fi
}

cmd_add_engine() {
  [ $# -ge 2 ] || die "usage: add-engine NAME PATH [--uci OPTION=VALUE ...]"
  local name=$1 path=$2; shift 2
  local opts=()
  while [ $# -gt 0 ]; do
    case "$1" in
      --uci) opts+=("$2"); shift 2 ;;
      --policy) opts+=("--policy"); shift ;;
      *) die "add-engine: unknown argument $1" ;;
    esac
  done
  register "$name" "$(cd "$(dirname "$path")" && pwd)/$(basename "$path")" ${opts[@]+"${opts[@]}"}
}

cmd_list() {
  [ -d "$defs" ] || die "nothing registered; run '$0 setup'"
  python3 - "$defs" <<'PY'
import glob, json, os, sys
for f in sorted(glob.glob(f"{sys.argv[1]}/*.json")):
    d = json.load(open(f, encoding="utf-8-sig"))
    o = d.get("Options", {})
    name = os.path.basename(f)[:-5]
    extra = []
    if o.get("UCI_LimitStrength") is True:
        extra.append(f"UCI_Elo={o.get('UCI_Elo')}")
    if name.startswith("rusty-chess") and o.get("EvalFile") not in (None, "<default>"):
        extra.append(f"EvalFile={o['EvalFile']}")
    if d.get("TimeControlID") == 2:
        extra.append("one node a move")
    extra = ", ".join(extra)
    print(f"{name:<48} {d.get('Version', ''):<24} {extra}")
PY
}

cmd_play() {
  local engines_csv="" mode=match tc="2:00+1" nodes="" rounds=10 parallel=1 name="" console=0
  while [ $# -gt 0 ]; do
    case "$1" in
      --engines) engines_csv=$2; shift 2 ;;
      --mode) mode=$2; shift 2 ;;
      --tc) tc=$2; shift 2 ;;
      --nodes) nodes=$2; shift 2 ;;
      --rounds) rounds=$2; shift 2 ;;
      --parallel) parallel=$2; shift 2 ;;
      --name) name=$2; shift 2 ;;
      --console) console=1; shift ;;
      *) die "play: unknown argument $1" ;;
    esac
  done
  [ -n "$engines_csv" ] || die "play needs --engines A,B[,...] (see '$0 list')"
  [ -x "$app/EngineBattle" ] || die "run '$0 setup' first"
  mkdir -p "$results" "$tournaments"
  name=${name:-$(echo "$engines_csv" | tr ',' '_')}
  local stamp file
  stamp=$(date +%Y%m%d-%H%M%S)
  file="$tournaments/$stamp-$name.json"
  python3 - "$here/tournament.template.json" "$file" "$defs" "$engines_csv" "$mode" "$tc" "$nodes" "$rounds" "$parallel" \
    "$results/$stamp-$name.pgn" "$root/tests/openings/openings.epd" "$name" <<'PY'
import json, os, sys
tpl, out, defs, csv, mode, tc, nodes, rounds, parallel, pgn, book, name = sys.argv[1:]
t = json.load(open(tpl))
engines = [e for e in csv.split(",") if e]
for e in engines:
    if not os.path.exists(f"{defs}/{e}.json"):
        sys.exit(f"no engine named {e}; registered: {', '.join(sorted(f[:-5] for f in os.listdir(defs)))}")
if mode == "match":
    if len(engines) != 2:
        sys.exit("--mode match takes exactly two engines (use rr or gauntlet for more)")
    t["TournamentMode"] = "RR"
elif mode == "rr":
    t["TournamentMode"] = "RR"
elif mode == "gauntlet":
    if len(engines) < 2:
        sys.exit("--mode gauntlet takes a challenger and at least one opponent")
    t["TournamentMode"] = "Gauntlet"  # the first engine is the challenger
else:
    sys.exit(f"unknown --mode {mode} (match, rr or gauntlet)")
t["Name"] = name
t["Rounds"] = int(rounds)
t["NumberOfGamesInParallel"] = int(parallel)
# Id 2 is for policy players such as Maia (registered with --policy), which
# play one node a move whatever the others play.
t["TimeControl"]["TimeConfigs"] = [{"Id": 1, "Nodes": int(nodes)} if nodes else {"Id": 1, "Tc": tc}, {"Id": 2, "Nodes": 1}]
t["Opening"]["OpeningsPath"] = book
t["PgnOutPath"] = pgn
t["EngineSetup"] = {"EngineDefFolder": defs, "EngineDefList": [f"{e}.json" for e in engines]}
json.dump(t, open(out, "w"), indent=2)
limit = f"{nodes} nodes a move" if nodes else tc
print(f"{name}: {t['TournamentMode']} of {', '.join(engines)} at {limit}, {rounds} rounds (each opening from both sides), {parallel} at once")
print(f"tournament: {out}")
print(f"PGN:        {pgn}")
PY
  (cd "$app" && ./eb-cli tournament validate "$file" >/dev/null) \
    || (cd "$app" && ./eb-cli tournament validate "$file"; exit 1) || die "EngineBattle rejected the tournament"
  if [ "$console" = 1 ]; then
    cd "$app" && exec ./eb-cli tournament run "$file"
  fi
  # The WebGUI plays the tournament.json next to it; keep its own as a backup once.
  [ -f "$app/wwwroot/tournament.json.orig" ] || cp "$app/wwwroot/tournament.json" "$app/wwwroot/tournament.json.orig" 2>/dev/null || true
  cp "$file" "$app/wwwroot/tournament.json"
  echo "==> starting EngineBattle: press Ctrl+R on the Tournament page to play, Ctrl+C there to stop"
  cmd_gui
}

# EngineBattle processes started from this install, as "PID PORT" lines. A
# second copy can't take the first one's port and picks the next one, so the
# browser can end up on the old copy with an old tournament.
running_guis() {
  local pid cwd port
  for pid in $(pgrep -x EngineBattle 2>/dev/null); do
    cwd=$(lsof -a -p "$pid" -d cwd -Fn 2>/dev/null | sed -n 's/^n//p')
    [ "$cwd" = "$app" ] || continue
    port=$(lsof -a -p "$pid" -iTCP -sTCP:LISTEN -nP -Fn 2>/dev/null | sed -n 's/^n.*:\([0-9]*\)$/\1/p' | head -1)
    echo "$pid ${port:-?}"
  done
}

cmd_gui() {
  [ -x "$app/EngineBattle" ] || die "run '$0 setup' first"
  local running
  running=$(running_guis)
  if [ -n "$running" ]; then
    echo "EngineBattle is already running:" >&2
    echo "$running" | while read -r pid port; do
      echo "  http://localhost:$port  (stop it with: kill $pid)" >&2
    done
    die "open that page (it reads the new tournament when you open the Tournament page or press Ctrl+U), or stop it and run this again"
  fi
  cd "$app" && exec ./EngineBattle "$@"
}

case "${1:-}" in
  setup) shift; cmd_setup "$@" ;;
  bots) shift; cmd_bots "$@" ;;
  add-rusty) shift; cmd_add_rusty "$@" ;;
  add-engine) shift; cmd_add_engine "$@" ;;
  list) shift; cmd_list ;;
  play) shift; cmd_play "$@" ;;
  gui) shift; cmd_gui "$@" ;;
  *) sed -n '2,14p' "$0" | sed 's/^# \{0,1\}//'; exit 2 ;;
esac
