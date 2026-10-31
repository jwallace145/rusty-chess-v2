#!/usr/bin/env bash
# Play rusty-chess against other UCI engines locally in EngineBattle
# (https://github.com/lepned/EngineBattle), and watch the games live.
#
#   tools/engine-battle/eb.sh setup                     # install EngineBattle and two opponents, register them and this tree's rusty-chess
#   tools/engine-battle/eb.sh add-rusty [--ref REF] [--net NET] [--name NAME]
#   tools/engine-battle/eb.sh add-engine NAME PATH [--uci OPTION=VALUE ...]
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
for o in overrides:
    k, _, v = o.partition("=")
    if k not in opts:
        sys.exit(f"{name}: the engine has no UCI option '{k}' (it has: {', '.join(opts)})")
    old = opts[k]
    opts[k] = (v.lower() == "true") if isinstance(old, bool) else int(v) if isinstance(old, int) else v
d["Version"] = d.get("Name", "")
d["Name"] = name
d["Alias"] = name
d["TimeControlID"] = 1
json.dump(d, open(out, "w"), indent=2)
print(f"registered {name}: {d['Path']} ({', '.join(f'{k}={v}' for k, v in opts.items())})")
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
t["TimeControl"]["TimeConfigs"] = [{"Id": 1, "Nodes": int(nodes)} if nodes else {"Id": 1, "Tc": tc}]
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

cmd_gui() {
  [ -x "$app/EngineBattle" ] || die "run '$0 setup' first"
  cd "$app" && exec ./EngineBattle "$@"
}

case "${1:-}" in
  setup) shift; cmd_setup "$@" ;;
  add-rusty) shift; cmd_add_rusty "$@" ;;
  add-engine) shift; cmd_add_engine "$@" ;;
  list) shift; cmd_list ;;
  play) shift; cmd_play "$@" ;;
  gui) shift; cmd_gui "$@" ;;
  *) sed -n '2,13p' "$0" | sed 's/^# \{0,1\}//'; exit 2 ;;
esac
