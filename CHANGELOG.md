# Changelog

## [0.3.0](https://github.com/jwallace145/rusty-chess-v2/compare/v0.2.0...v0.3.0) (2026-10-27)


### Features

* **ci:** explain each bench comment row in a collapsed section and note perft in release notes ([#16](https://github.com/jwallace145/rusty-chess-v2/issues/16)) ([1d391d5](https://github.com/jwallace145/rusty-chess-v2/commit/1d391d500f6c0d04d1b223d0858e517b2b7d2bc5))
* **search:** add alpha-beta search with quiescence, uci go and a real bench ([#19](https://github.com/jwallace145/rusty-chess-v2/issues/19)) ([1e051b4](https://github.com/jwallace145/rusty-chess-v2/commit/1e051b48c7fdd8671ff63eb3c6c6264db116b8bc))
* **search:** add tapered pesto evaluation and the incrementally updated search board ([#18](https://github.com/jwallace145/rusty-chess-v2/issues/18)) ([47ec8f4](https://github.com/jwallace145/rusty-chess-v2/commit/47ec8f4d01b241e92bc3f472ce446ff07ab3e772))


### Documentation

* add the v1 post-mortem and turn its three recurring search bugs into agent rules ([#20](https://github.com/jwallace145/rusty-chess-v2/issues/20)) ([e36ca65](https://github.com/jwallace145/rusty-chess-v2/commit/e36ca650f898e5bdf2d3a46d91ca5cc8859cda23))

## [0.2.0](https://github.com/jwallace145/rusty-chess-v2/compare/v0.1.0...v0.2.0) (2026-10-27)


### Features

* **ci:** attach bench telemetry to every release and compare prs against it ([#12](https://github.com/jwallace145/rusty-chess-v2/issues/12)) ([32c2a84](https://github.com/jwallace145/rusty-chess-v2/commit/32c2a84c120701f147775ad2e57a26bd5676b4e8))
* **ci:** post a bench comment with deltas against main on every pr push ([#11](https://github.com/jwallace145/rusty-chess-v2/issues/11)) ([cc1e519](https://github.com/jwallace145/rusty-chess-v2/commit/cc1e5194e17cdaa4994d56d5f85b7a4de2b6a4ff))
* **core:** add squares, bitboards, leaper tables and magic bitboard sliders ([#13](https://github.com/jwallace145/rusty-chess-v2/issues/13)) ([760ea24](https://github.com/jwallace145/rusty-chess-v2/commit/760ea240cccaa50a893d5fe44c5910c0d5bdff3c))
* **core:** add the position, make/unmake, legal move generation and perft ([#14](https://github.com/jwallace145/rusty-chess-v2/issues/14)) ([5135719](https://github.com/jwallace145/rusty-chess-v2/commit/513571925fdd6b57fa932c8f410e9d14f02c1986))
* **tools:** add the perft suite runner and the performance gate ([#10](https://github.com/jwallace145/rusty-chess-v2/issues/10)) ([1cf2e26](https://github.com/jwallace145/rusty-chess-v2/commit/1cf2e26b0b3c1810f6eb7450fedeefcc035033d0))


### Documentation

* describe the root package layout and release type that release please needs ([#8](https://github.com/jwallace145/rusty-chess-v2/issues/8)) ([2c0c213](https://github.com/jwallace145/rusty-chess-v2/commit/2c0c213f93dfe4bc980f9e0ee1a13ca17576c3f8))

## 0.1.0 (2026-10-27)


### Features

* add stats feature, json lines telemetry, bench and perft subcommands ([#3](https://github.com/jwallace145/rusty-chess-v2/issues/3)) ([23b883a](https://github.com/jwallace145/rusty-chess-v2/commit/23b883a272d78ae8cd7adca9d9c88542727e76ee))


### Bug Fixes

* make the workspace root the engine package so release please can bump versions ([#6](https://github.com/jwallace145/rusty-chess-v2/issues/6)) ([957318d](https://github.com/jwallace145/rusty-chess-v2/commit/957318d6785712ef7286c69602037dc6873fe403))
