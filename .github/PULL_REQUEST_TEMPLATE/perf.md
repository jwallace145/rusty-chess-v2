<!--
PR title must be a valid Conventional Commit, e.g. `perf(scope): speed up something`.
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

<!-- From `rusty-chess bench` on a quiet machine, before and after this change. A changed signature means search behaviour changed. CI checks the LAST `Bench:` line (the after) against its own run. -->

- Bench before: `Bench: <nodes>`
- Bench after: `Bench: <nodes>`

## NPS

<!-- `rusty-chess bench` on a quiet machine, same signature before and after. -->

- NPS before: `<nps>`
- NPS after: `<nps>`

## Checklist

- [ ] PR title is a valid Conventional Commit (`perf`)
- [ ] Every commit is a single line (only `Co-authored-by` trailers may follow)
- [ ] Docs updated (README, docs/, CONTRIBUTING, AGENTS) where behaviour changed
