<!--
PR title must be a valid Conventional Commit, e.g. `fix(scope): correct something`.
Squash merges use the title as the commit message.
-->

## Summary

<!-- One or two sentences on what this PR does. -->

## Motivation and context

<!-- Why is this change needed? Link issues or discussions. -->

## Bug

<!-- What is wrong, as observed. Include engine output or a failing perft/FEN if short. -->

## Root cause

<!-- Why it happens. -->

## Reproduction

<!-- Steps, a FEN, or a command that shows the bug before this change. -->

## What changed

<!-- Bullet the changes. Call out anything that changes search behaviour, the UCI surface, or the JSON lines schema. -->

-

## How it was verified

<!-- e.g. cargo fmt/clippy/test locally, perft suite, bench run, the CI bench comment reviewed. -->

-

## Checklist

- [ ] PR title is a valid Conventional Commit (`fix`)
- [ ] Every commit is a single line (only `Co-authored-by` trailers may follow)
- [ ] Docs updated (README, docs/, CONTRIBUTING, AGENTS) where behaviour changed
