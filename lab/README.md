# Lab scripts (research only)

The scripts the 2026-11-12 overnight research session used for its exploratory
local screens. They are copies: the session ran them from its scratch directory,
whose absolute path they still carry. Nothing here is part of the engine or of
CI, and this branch is never merged. The record of what they measured is
`docs/research-log.md` on the session's docs PR.

- `screen.sh`: one fixed-node match of the lab binary with options against the
  same binary with defaults, with the CI SPRT's openings and adjudication.
- `queue.sh`: runs queued screens one at a time.
- `gate.sh`: a variant's bench signature, fail-high-first, EBF and counters.
- `screens_table.py`: renders a screen's row for the research log from its
  fastchess log.
- `spsa.py`: SPSA tuning of the lab parameters by self-play.
- `sprt-live.sh`: reads a running SPRT's task log (read-only).
- `crates/search/examples/piece_values.rs`: what the net thinks a piece is
  worth (remove each piece in turn, re-read the static evaluation).

## What the session left here

Added at the end of the session, so the research log's exploratory tables can
be checked after the scratch directory is gone.

- `runs/<screen>/fc.log` and `config.json`: each local screen's fastchess log
  and settings, the source of the log's screens table (`render_screens.py`
  reads them; `runs/summary.tsv` and `runs/screens-meta.json` are its inputs
  too). `runs/s1-lmr-cut/fc.log` is the log an editing mistake truncated, kept
  as it was left. `runs/pilot-identical` is the harness's negative control.
- `runs/spsa-s1/` (`state.json`, `spsa.log`), `runs/spsa-test/` and the
  `spsa-s1-*` and `v1-spsa.log` files: SPSA run S1, its dry run and its
  validation screen.
- `runs/matches.log`, `match-queue.txt`, `match-done.txt`, `match-hold.txt`,
  `queue.txt`, `done.txt`, `deferred.txt`, `queue.log`: what the two queue
  runners dispatched, finished and withdrew. The instance matches' own reports
  and games are the artifacts of their `Match` runs on GitHub.
- `analysis/clock_use.py`, `analysis/depth_by_phase.py`,
  `analysis/pooled_interval.py`: the three one-off calculations behind the
  log's clock table, experiment 2's depth table and match M18's pooled row,
  verbatim. The first two read SPRT artifacts, which are not copied here.
- `sprt_rows.py`: renders an SPRT run's result rows from its `sprt.md`
  artifacts, so the record's rows are not typed.

Not kept: the job logs of two failed runs (they name the AWS account; the runs
are linked from the log), the SPRT and match artifacts (on GitHub under their
run ids), and the binaries (each is a build of a commit the log names).

The repository's commit hooks normalised the end of each file and trailing
whitespace in these copies; nothing else differs from what the session read.
