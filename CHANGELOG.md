# Changelog

## [0.16.0](https://github.com/jwallace145/rusty-chess-v2/compare/v0.15.0...v0.16.0) (2026-10-28)


### Features

* **search:** probe and store the transposition table in quiescence ([#66](https://github.com/jwallace145/rusty-chess-v2/issues/66)) ([4c5c5b1](https://github.com/jwallace145/rusty-chess-v2/commit/4c5c5b13842120ff959b7d24ef99fead1f4e7f6a))

## [0.15.0](https://github.com/jwallace145/rusty-chess-v2/compare/v0.14.0...v0.15.0) (2026-10-28)


### Features

* **search:** fix quiescence in check and skip losing captures by static exchange ([#63](https://github.com/jwallace145/rusty-chess-v2/issues/63)) ([a22c879](https://github.com/jwallace145/rusty-chess-v2/commit/a22c87996dcaaed56e516bdd86a91825b0d34f5b))


### Documentation

* tick the mate-distance and checking-move pruning item with its sprt result ([#64](https://github.com/jwallace145/rusty-chess-v2/issues/64)) ([7372037](https://github.com/jwallace145/rusty-chess-v2/commit/7372037d70c742b1b8872e4a94dac6d5d0bd3a40))

### Benchmarks

* bench signature 2,566,403, instructions per node 2164, search 3.70M NPS, perft 202.65M NPS (pass (63)), fail-high first 89.9%, EBF 2.02, TT hit 27.9%
* since v0.14.0: fail-high first -0.4 pt, EBF +0.23, instructions per node +39.6%, search NPS −58.0%
* full history: [docs/benchmarks.md](docs/benchmarks.md)

## [0.14.0](https://github.com/jwallace145/rusty-chess-v2/compare/v0.13.0...v0.14.0) (2026-10-28)


### Features

* **bench:** raise the default bench depth from 6 to 10 ([#60](https://github.com/jwallace145/rusty-chess-v2/issues/60)) ([e81bac4](https://github.com/jwallace145/rusty-chess-v2/commit/e81bac4e796a97847a70bfd7ecd7ce56e3ac2506))
* **search:** add mate-distance pruning and keep checking moves out of late move pruning ([#57](https://github.com/jwallace145/rusty-chess-v2/issues/57)) ([4561886](https://github.com/jwallace145/rusty-chess-v2/commit/45618862137dd1d1c687548cea6ed0ea55676e4c))

### Benchmarks

* bench signature 3,775,127, instructions per node 1550, search 8.82M NPS, perft 371.94M NPS (pass (63)), fail-high first 90.3%, EBF 1.79, TT hit 28.6%
* since v0.13.0: fail-high first +1.2 pt, EBF -0.12, instructions per node −2.6%, search NPS +84.3%
* full history: [docs/benchmarks.md](docs/benchmarks.md)

## [0.13.0](https://github.com/jwallace145/rusty-chess-v2/compare/v0.12.0...v0.13.0) (2026-10-28)


### Features

* **search:** add late move pruning of quiet moves at shallow null-window nodes ([#55](https://github.com/jwallace145/rusty-chess-v2/issues/55)) ([678a3ac](https://github.com/jwallace145/rusty-chess-v2/commit/678a3ac98178c4ec046437a70ccabad9df7d9c2a))

### Benchmarks

* bench signature 387,631, instructions per node 1592, search 4.79M NPS, perft 256.73M NPS (pass (63)), fail-high first 89.2%, EBF 1.91, TT hit 35.8%
* since v0.12.0: fail-high first -0.6 pt, EBF -0.26, instructions per node +22.0%, search NPS −25.9%
* full history: [docs/benchmarks.md](docs/benchmarks.md)

## [0.12.0](https://github.com/jwallace145/rusty-chess-v2/compare/v0.11.0...v0.12.0) (2026-10-28)


### Features

* **search:** add reverse futility pruning at shallow null-window nodes ([#53](https://github.com/jwallace145/rusty-chess-v2/issues/53)) ([f982be1](https://github.com/jwallace145/rusty-chess-v2/commit/f982be1800205954e40f75b84535826f14e9b205))

### Benchmarks

* bench signature 601,010, instructions per node 1305, search 6.46M NPS, perft 262.19M NPS (pass (63)), fail-high first 89.8%, EBF 2.17, TT hit 28.7%
* since v0.11.0: fail-high first -3.3 pt, EBF -0.01, instructions per node −0.0%, search NPS +1.1%
* full history: [docs/benchmarks.md](docs/benchmarks.md)

## [0.11.0](https://github.com/jwallace145/rusty-chess-v2/compare/v0.10.0...v0.11.0) (2026-10-27)


### Features

* add a lichess bot setup that connects the engine through lichess-bot ([#49](https://github.com/jwallace145/rusty-chess-v2/issues/49)) ([cba4af9](https://github.com/jwallace145/rusty-chess-v2/commit/cba4af93140eb9480dafd92b3b2fb8c9cfced61a))
* **search:** add late move reductions with a log-based table ([#48](https://github.com/jwallace145/rusty-chess-v2/issues/48)) ([afa0140](https://github.com/jwallace145/rusty-chess-v2/commit/afa0140aad91911ba4e5301f2fbe688d47a6d658))
* **search:** add null-move pruning with a non-pawn-material guard ([#47](https://github.com/jwallace145/rusty-chess-v2/issues/47)) ([b83e55b](https://github.com/jwallace145/rusty-chess-v2/commit/b83e55b51b901842e24a58c0c5a8c40e55b8c9d6))


### Bug Fixes

* **lichess:** pin lichess-bot to a master commit and match its current config keys ([#52](https://github.com/jwallace145/rusty-chess-v2/issues/52)) ([7f781c4](https://github.com/jwallace145/rusty-chess-v2/commit/7f781c4b8a4550e3aeace5327734ba37d20ac487))

### Benchmarks

* bench signature 716,057, instructions per node 1305, search 6.39M NPS, perft 201.46M NPS (pass (63)), fail-high first 93.1%, EBF 2.17, TT hit 36.0%
* since v0.10.0: fail-high first -4.4 pt, EBF -1.73, instructions per node +7.4%, search NPS −43.4%
* full history: [docs/benchmarks.md](docs/benchmarks.md)

## [0.10.0](https://github.com/jwallace145/rusty-chess-v2/compare/v0.9.0...v0.10.0) (2026-10-27)


### Features

* **search:** search each iteration in an aspiration window around the previous score ([#46](https://github.com/jwallace145/rusty-chess-v2/issues/46)) ([619a192](https://github.com/jwallace145/rusty-chess-v2/commit/619a192623b1aa46b3359cf444cc36169cf6e501))

### Benchmarks

* bench signature 7,082,718, instructions per node 1215, search 11.30M NPS, perft 281.40M NPS (pass (63)), fail-high first 97.5%, EBF 3.90, TT hit 35.6%
* since v0.9.0: fail-high first +0.0 pt, EBF +0.03, instructions per node +0.3%, search NPS −20.9%
* full history: [docs/benchmarks.md](docs/benchmarks.md)

## [0.9.0](https://github.com/jwallace145/rusty-chess-v2/compare/v0.8.0...v0.9.0) (2026-10-27)


### Features

* **search:** search non-first moves with a null window (principal variation search) ([#44](https://github.com/jwallace145/rusty-chess-v2/issues/44)) ([1879cc4](https://github.com/jwallace145/rusty-chess-v2/commit/1879cc41b44b4ba02127c410a33593a0dae34219))


### Bug Fixes

* **ci:** rate older refs by building only the engine from them ([#42](https://github.com/jwallace145/rusty-chess-v2/issues/42)) ([240056c](https://github.com/jwallace145/rusty-chess-v2/commit/240056c829e6dc7a0bce02f203438f88d5ff279c))


### Documentation

* move the ebf gate into phase 4 and mark phase 3 complete ([#45](https://github.com/jwallace145/rusty-chess-v2/issues/45)) ([3570b5c](https://github.com/jwallace145/rusty-chess-v2/commit/3570b5c7fc944f6413ae4907060018eba15ea309))

### Benchmarks

* bench signature 7,143,901, instructions per node 1212, search 14.29M NPS, perft 368.37M NPS (pass (63)), fail-high first 97.5%, EBF 3.86, TT hit 35.7%
* since v0.8.0: fail-high first -0.0 pt, EBF -0.32, instructions per node +1.0%, search NPS +93.8%
* full history: [docs/benchmarks.md](docs/benchmarks.md)

## [0.8.0](https://github.com/jwallace145/rusty-chess-v2/compare/v0.7.0...v0.8.0) (2026-10-27)


### Features

* **ci:** estimate strength against a stockfish elo ladder on every release ([#41](https://github.com/jwallace145/rusty-chess-v2/issues/41)) ([aa2a9c6](https://github.com/jwallace145/rusty-chess-v2/commit/aa2a9c6604b4601e0f4b868b97354a5b0308fd91))


### Documentation

* close out the phase 3 checklist and set the efficiency bars ([#38](https://github.com/jwallace145/rusty-chess-v2/issues/38)) ([ece6f04](https://github.com/jwallace145/rusty-chess-v2/commit/ece6f040d98afba1aad6815983991fc562dd38f8))

### Benchmarks

* bench signature 8,243,750, instructions per node 1199, search 7.37M NPS, perft 199.85M NPS (pass (63)), fail-high first 97.5%, EBF 4.18, TT hit 34.8%
* since v0.7.0: fail-high first +0.0 pt, EBF +0.00, instructions per node −0.0%, search NPS −6.7%
* full history: [docs/benchmarks.md](docs/benchmarks.md)

## [0.7.0](https://github.com/jwallace145/rusty-chess-v2/compare/v0.6.0...v0.7.0) (2026-10-27)


### Features

* **ci:** judge speed by paired runs and instruction counts instead of runner-dependent nps ([#36](https://github.com/jwallace145/rusty-chess-v2/issues/36)) ([e6f465e](https://github.com/jwallace145/rusty-chess-v2/commit/e6f465eb83b2d7da8d70deea3593e57850fde3a9))
* **search:** order quiet moves by butterfly history with gravity ([#34](https://github.com/jwallace145/rusty-chess-v2/issues/34)) ([0472ecf](https://github.com/jwallace145/rusty-chess-v2/commit/0472ecf5e6ddcbceceecadab7fa2f784a59d40aa))

### Benchmarks

* bench signature 8,243,750, instructions per node 1199, search 7.90M NPS, perft 201.52M NPS (pass (63)), fail-high first 97.5%, EBF 4.18, TT hit 34.8%
* since v0.6.0: fail-high first +0.7 pt, EBF -0.15, search NPS −9.7%
* full history: [docs/benchmarks.md](docs/benchmarks.md)

## [0.6.0](https://github.com/jwallace145/rusty-chess-v2/compare/v0.5.0...v0.6.0) (2026-10-27)


### Features

* **search:** add two killer moves per ply ([#33](https://github.com/jwallace145/rusty-chess-v2/issues/33)) ([83807d9](https://github.com/jwallace145/rusty-chess-v2/commit/83807d912288ac51c5f36c3e4323768d00070280))

### Benchmarks

* bench signature 8,986,067, search 8.75M NPS, perft 203.09M NPS (pass (63)), fail-high first 96.8%, EBF 4.33, TT hit 33.7%
* since v0.5.0: fail-high first +11.2 pt, EBF -0.41, search NPS −30.7%
* full history: [docs/benchmarks.md](docs/benchmarks.md)

## [0.5.0](https://github.com/jwallace145/rusty-chess-v2/compare/v0.4.0...v0.5.0) (2026-10-27)


### Features

* **ci:** record benchmark history in the changelog, release notes and docs per release ([#30](https://github.com/jwallace145/rusty-chess-v2/issues/30)) ([7b2aeb1](https://github.com/jwallace145/rusty-chess-v2/commit/7b2aeb131cbe5fc49f1ced85235febd7762e0d15))
* **search:** add a staged move picker with see-filtered captures ([#29](https://github.com/jwallace145/rusty-chess-v2/issues/29)) ([be646b5](https://github.com/jwallace145/rusty-chess-v2/commit/be646b5d6a04159e77dc6e0d188d4c6fc647a4d4))


### Bug Fixes

* **ci:** stop the release benchmark loop and commit the ignored raw benchmark data ([#32](https://github.com/jwallace145/rusty-chess-v2/issues/32)) ([9487c3b](https://github.com/jwallace145/rusty-chess-v2/commit/9487c3bd7260f2c4a92a4edbda3b196a5e9a1af0))

### Benchmarks

* bench signature 14,942,735, search 12.62M NPS, perft 299.32M NPS (pass (63)), fail-high first 85.6%, EBF 4.74, TT hit 29.4%
* since v0.4.0: fail-high first +7.0 pt, EBF +0.13, search NPS +99.6%
* full history: [docs/benchmarks.md](docs/benchmarks.md)

## [0.4.0](https://github.com/jwallace145/rusty-chess-v2/compare/v0.3.0...v0.4.0) (2026-10-27)


### Features

* **ci:** run sprt on a standardized runner at fixed nodes with our own openings ([#24](https://github.com/jwallace145/rusty-chess-v2/issues/24)) ([61c67a4](https://github.com/jwallace145/rusty-chess-v2/commit/61c67a4f8c30befcb1a88483c516ee5424dd7e60))
* **search:** add a transposition table with bucketed replacement and tt-move ordering ([#26](https://github.com/jwallace145/rusty-chess-v2/issues/26)) ([7cb7920](https://github.com/jwallace145/rusty-chess-v2/commit/7cb79209bb5a65aeee7910e0bb6b2df014578a20))


### Documentation

* tick the sprt-on-ci and openings items and record the first sprt learnings ([#27](https://github.com/jwallace145/rusty-chess-v2/issues/27)) ([d3402fb](https://github.com/jwallace145/rusty-chess-v2/commit/d3402fbed09f66ffd7764332785fe74f73ceda5b))
* tick the transposition table item with its sprt and bench numbers ([#28](https://github.com/jwallace145/rusty-chess-v2/issues/28)) ([adb9216](https://github.com/jwallace145/rusty-chess-v2/commit/adb9216c34893ea7321bbe6bc503604ad15634d4))
* turn the roadmap into a living checklist with measured status and learnings ([#23](https://github.com/jwallace145/rusty-chess-v2/issues/23)) ([a9fc610](https://github.com/jwallace145/rusty-chess-v2/commit/a9fc61023c8a70ee94f3ff6e58077e7dbd2f08bc))

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
