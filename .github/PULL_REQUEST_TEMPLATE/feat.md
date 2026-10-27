<!--
PR title must be a valid Conventional Commit, e.g. `feat(scope): add something`.
Squash merges use the title as the commit message.
-->

## Summary

<!-- One or two sentences on what this PR does. -->

## Motivation and context

<!-- Why is this change needed? Link issues or discussions. -->

## What changed

<!-- Bullet the changes. Call out anything that changes search behaviour, the UCI surface, or the JSON lines schema. -->

-

## How it was verified

<!-- e.g. cargo fmt/clippy/test locally, perft suite, bench run, the CI bench comment reviewed. -->

-

## Bench signature

<!-- From `rusty-chess bench` on a quiet machine, before and after this change. A changed signature means search behaviour changed. -->

- Bench before: `Bench: <nodes>`
- Bench after: `Bench: <nodes>`

## SPRT result

<!-- fastchess SPRT against `main`, e.g. 8+0.08s with an opening book. A `feat` that touches search or eval does not merge without a passing SPRT. -->

- Elo: `<elo> ± <error>`
- LOS: `<los>`
- Games: `<wins> / <losses> / <draws>`

## Checklist

- [ ] PR title is a valid Conventional Commit (`feat`)
- [ ] Every commit is a single line (only `Co-authored-by` trailers may follow)
- [ ] Docs updated (README, docs/, CONTRIBUTING, AGENTS) where behaviour changed
