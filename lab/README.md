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
