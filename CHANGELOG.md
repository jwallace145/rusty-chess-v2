# Changelog

## [0.51.0](https://github.com/jwallace145/rusty-chess-v2/compare/v0.50.0...v0.51.0) (2026-11-13)


### Features

* **nnue:** embed net8x160, net8's recipe at twice the schedule, as the default ([#256](https://github.com/jwallace145/rusty-chess-v2/issues/256)) ([e80876f](https://github.com/jwallace145/rusty-chess-v2/commit/e80876fa36f09d5d7c18a91f36e903fd5cf19f0d))

## [0.50.0](https://github.com/jwallace145/rusty-chess-v2/compare/v0.49.0...v0.50.0) (2026-11-13)


### Features

* **search:** one more ply of late-move reduction at expected cut nodes ([#254](https://github.com/jwallace145/rusty-chess-v2/issues/254)) ([059caa3](https://github.com/jwallace145/rusty-chess-v2/commit/059caa34e1c201f406d3c0c48198b5368d805c83))

### Benchmarks

* bench signature 958,984, instructions per node 4035, search 1.46M NPS, perft 200.28M NPS (pass (63)), fail-high first 87.6%, EBF 1.85, TT hit 55.4%
* since v0.49.0: fail-high first -0.2 pt, EBF -0.05, instructions per node +0.0%, search NPS −48.6%
* full history: [docs/benchmarks.md](docs/benchmarks.md)

## [0.49.0](https://github.com/jwallace145/rusty-chess-v2/compare/v0.48.1...v0.49.0) (2026-11-13)


### Features

* **search:** adaptive null-move reduction, deeper with depth and with the margin over beta ([#246](https://github.com/jwallace145/rusty-chess-v2/issues/246)) ([cd51dee](https://github.com/jwallace145/rusty-chess-v2/commit/cd51deee9fefe22ceb1ec7c962040a0323481080))

### Benchmarks

* bench signature 939,252, instructions per node 4033, search 2.84M NPS, perft 356.12M NPS (pass (63)), fail-high first 87.7%, EBF 1.90, TT hit 52.2%
* since v0.48.1: fail-high first +0.3 pt, EBF +0.02, instructions per node −0.8%, search NPS +56.2%
* full history: [docs/benchmarks.md](docs/benchmarks.md)

## [0.48.1](https://github.com/jwallace145/rusty-chess-v2/compare/v0.48.0...v0.48.1) (2026-11-13)


### Documentation

* add the project logo and use it in the README ([#245](https://github.com/jwallace145/rusty-chess-v2/issues/245)) ([824b4da](https://github.com/jwallace145/rusty-chess-v2/commit/824b4da84a49a449c9c432bfc87abb207c47a6cf))
* **nnue:** add versioned architecture diagrams, manifests and a gallery for every net ([#242](https://github.com/jwallace145/rusty-chess-v2/issues/242)) ([57e7d90](https://github.com/jwallace145/rusty-chess-v2/commit/57e7d90da93455663329e6fda1d20f87ccf2bd46))
* **nnue:** render the architecture SVGs with their natural size so GitHub shows them full size ([#244](https://github.com/jwallace145/rusty-chess-v2/issues/244)) ([80dd817](https://github.com/jwallace145/rusty-chess-v2/commit/80dd817a1a990ac1977f6d964ec0fa2822be491d))

### Benchmarks

* bench signature 970,013, instructions per node 4066, search 1.82M NPS, perft 267.35M NPS (pass (63)), fail-high first 87.4%, EBF 1.88, TT hit 52.5%
* since v0.48.0: fail-high first +0.0 pt, EBF +0.00, instructions per node −0.0%, search NPS −0.2%
* full history: [docs/benchmarks.md](docs/benchmarks.md)

## [0.48.0](https://github.com/jwallace145/rusty-chess-v2/compare/v0.47.2...v0.48.0) (2026-11-09)


### Features

* **nnue:** embed net8, net7rl012's recipe on generations 0-6, as the default ([#235](https://github.com/jwallace145/rusty-chess-v2/issues/235)) ([37068b2](https://github.com/jwallace145/rusty-chess-v2/commit/37068b22cad8688ea1b9ff1f0d718827fe162b7a))


### Documentation

* **nnue:** record the training of net8 and its held-out comparison with net7rl012 ([#236](https://github.com/jwallace145/rusty-chess-v2/issues/236)) ([ae5e2e9](https://github.com/jwallace145/rusty-chess-v2/commit/ae5e2e9373f61be2ec6a95f3244fdd26be9a1950))

### Benchmarks

* bench signature 970,013, instructions per node 4066, search 1.82M NPS, perft 291.57M NPS (pass (63)), fail-high first 87.4%, EBF 1.88, TT hit 52.5%
* since v0.47.2: fail-high first -0.5 pt, EBF -0.08, instructions per node +0.6%, search NPS +26.8%
* full history: [docs/benchmarks.md](docs/benchmarks.md)

## [0.47.2](https://github.com/jwallace145/rusty-chess-v2/compare/v0.47.1...v0.47.2) (2026-11-09)


### Documentation

* **datasets:** record generation 6, played by net7rl012 on spot, and its passing full audit ([#232](https://github.com/jwallace145/rusty-chess-v2/issues/232)) ([3ffeb5a](https://github.com/jwallace145/rusty-chess-v2/commit/3ffeb5a2b7570fa22646ec7506f775fd32d59690))
* **nnue:** plan net8, net7rl012's recipe on generations 0-6 ([#234](https://github.com/jwallace145/rusty-chess-v2/issues/234)) ([3293c50](https://github.com/jwallace145/rusty-chess-v2/commit/3293c50f08b1ec990b70bba791d562ff69fbd55e))

### Benchmarks

* bench signature 997,279, instructions per node 4044, search 1.43M NPS, perft 202.96M NPS (pass (63)), fail-high first 87.9%, EBF 1.95, TT hit 51.4%
* since v0.47.1: fail-high first +0.0 pt, EBF +0.00, instructions per node +0.0%, search NPS −51.9%
* full history: [docs/benchmarks.md](docs/benchmarks.md)

## [0.47.1](https://github.com/jwallace145/rusty-chess-v2/compare/v0.47.0...v0.47.1) (2026-11-08)


### Documentation

* **search:** plan a major-piece correction table from the residual-signal measurement ([#227](https://github.com/jwallace145/rusty-chess-v2/issues/227)) ([122e881](https://github.com/jwallace145/rusty-chess-v2/commit/122e88190d25509834957be1b4bb27429e564464))
* **search:** record the correction history's SPRT and pre-test, and plan its schedule ([#223](https://github.com/jwallace145/rusty-chess-v2/issues/223)) ([2acd8d4](https://github.com/jwallace145/rusty-chess-v2/commit/2acd8d42f6351e6db1b210190979273458914f18))
* **search:** record the dropped correction-history schedule and why its pre-test misled ([#226](https://github.com/jwallace145/rusty-chess-v2/issues/226)) ([4c37b49](https://github.com/jwallace145/rusty-chess-v2/commit/4c37b497a8c314122107bbda996216dfba21f664))
* **search:** record the dropped major-piece correction table and what it says about the pre-test ([#229](https://github.com/jwallace145/rusty-chess-v2/issues/229)) ([f945326](https://github.com/jwallace145/rusty-chess-v2/commit/f945326453f151e0d8dd40ca0bb7dae7572ce172))

### Benchmarks

* bench signature 997,279, instructions per node 4044, search 2.99M NPS, perft 379.28M NPS (pass (63)), fail-high first 87.9%, EBF 1.95, TT hit 51.4%
* since v0.47.0: fail-high first +0.0 pt, EBF +0.00, instructions per node −0.0%, search NPS +114.4%
* full history: [docs/benchmarks.md](docs/benchmarks.md)

## [0.47.0](https://github.com/jwallace145/rusty-chess-v2/compare/v0.46.2...v0.47.0) (2026-11-08)


### Features

* **search:** pawn-structure correction history for the static evaluation ([#221](https://github.com/jwallace145/rusty-chess-v2/issues/221)) ([ff672f3](https://github.com/jwallace145/rusty-chess-v2/commit/ff672f392c2e3403e5029387e91dda67f4decab8))


### Documentation

* **search:** plan pawn-structure correction history, with the measured error it could correct ([#219](https://github.com/jwallace145/rusty-chess-v2/issues/219)) ([58f8e89](https://github.com/jwallace145/rusty-chess-v2/commit/58f8e89339f719bf148cd3acbe92b7af362891a6))

### Benchmarks

* bench signature 997,279, instructions per node 4044, search 1.39M NPS, perft 225.45M NPS (pass (63)), fail-high first 87.9%, EBF 1.95, TT hit 51.4%
* since v0.46.2: fail-high first +0.1 pt, EBF +0.11, instructions per node +0.7%, search NPS −3.7%
* full history: [docs/benchmarks.md](docs/benchmarks.md)

## [0.46.2](https://github.com/jwallace145/rusty-chess-v2/compare/v0.46.1...v0.46.2) (2026-11-08)


### Performance

* **nnue:** cache accumulators per view so king-bucket rebuilds apply only the pieces that moved ([#217](https://github.com/jwallace145/rusty-chess-v2/issues/217)) ([2fd48ff](https://github.com/jwallace145/rusty-chess-v2/commit/2fd48ffb0f608dc559aadbc3b1f4a15544adbecb))

### Benchmarks

* bench signature 976,528, instructions per node 4018, search 1.45M NPS, perft 226.31M NPS (pass (63)), fail-high first 87.8%, EBF 1.84, TT hit 51.9%
* since v0.46.1: fail-high first +0.0 pt, EBF +0.00, instructions per node −2.6%, search NPS −19.3%
* full history: [docs/benchmarks.md](docs/benchmarks.md)

## [0.46.1](https://github.com/jwallace145/rusty-chess-v2/compare/v0.46.0...v0.46.1) (2026-11-07)


### Documentation

* **nnue:** profile the NNUE cost at 512 and 768 and plan a refresh cache for king-bucket rebuilds ([#215](https://github.com/jwallace145/rusty-chess-v2/issues/215)) ([821b79b](https://github.com/jwallace145/rusty-chess-v2/commit/821b79b96fdffb5e71f6b1a34664f96f0cdeeaf2))
* **nnue:** record net7rl012w768's training and its failed SPRT against net7rl012 ([#213](https://github.com/jwallace145/rusty-chess-v2/issues/213)) ([e61c3d1](https://github.com/jwallace145/rusty-chess-v2/commit/e61c3d1a691ab329d4beb885049f4117400bd2bc))

### Benchmarks

* bench signature 976,528, instructions per node 4123, search 1.79M NPS, perft 286.96M NPS (pass (63)), fail-high first 87.8%, EBF 1.84, TT hit 51.9%
* since v0.46.0: fail-high first +0.0 pt, EBF +0.00, instructions per node +0.0%, search NPS −12.7%
* full history: [docs/benchmarks.md](docs/benchmarks.md)

## [0.46.0](https://github.com/jwallace145/rusty-chess-v2/compare/v0.45.1...v0.46.0) (2026-11-07)


### Features

* **nnue:** read 768-neuron nets and plan net7rl012w768, net7rl012's recipe at 768 hidden ([#211](https://github.com/jwallace145/rusty-chess-v2/issues/211)) ([5d7a571](https://github.com/jwallace145/rusty-chess-v2/commit/5d7a5716e66956c5d0acce85757427de5be56136))


### Documentation

* **nnue:** record net7rl012345's failed SPRT against net7rl012 and drop it ([#212](https://github.com/jwallace145/rusty-chess-v2/issues/212)) ([f098dea](https://github.com/jwallace145/rusty-chess-v2/commit/f098deabdf13597cf609a2d7fe9bc22a0246dc82))
* **nnue:** record the training of net7rl012345 and its held-out comparison with net7rl012 ([#208](https://github.com/jwallace145/rusty-chess-v2/issues/208)) ([ad0a9fb](https://github.com/jwallace145/rusty-chess-v2/commit/ad0a9fbb940a27c05f4123e85eff0e92470397ba))

### Benchmarks

* bench signature 976,528, instructions per node 4123, search 2.05M NPS, perft 288.99M NPS (pass (63)), fail-high first 87.8%, EBF 1.84, TT hit 51.9%
* since v0.45.1: fail-high first +0.0 pt, EBF +0.00, instructions per node −0.1%, search NPS +11.6%
* full history: [docs/benchmarks.md](docs/benchmarks.md)

## [0.45.1](https://github.com/jwallace145/rusty-chess-v2/compare/v0.45.0...v0.45.1) (2026-11-07)


### Documentation

* **datasets:** record net7rl012's relabelling of generations 3-5 and its passing audit ([#205](https://github.com/jwallace145/rusty-chess-v2/issues/205)) ([a9892ef](https://github.com/jwallace145/rusty-chess-v2/commit/a9892efad0b49376b9d3682a63cde65b3a799223))
* **nnue:** plan net7rl012345, net7rl012's recipe with generations 3-5 relabelled by net7rl012 ([#206](https://github.com/jwallace145/rusty-chess-v2/issues/206)) ([50b588e](https://github.com/jwallace145/rusty-chess-v2/commit/50b588eb511c572838725245b5526e9e2e60547d))

### Benchmarks

* bench signature 976,528, instructions per node 4126, search 1.84M NPS, perft 287.47M NPS (pass (63)), fail-high first 87.8%, EBF 1.84, TT hit 51.9%
* since v0.45.0: fail-high first +0.0 pt, EBF +0.00, instructions per node +0.0%, search NPS +18.1%
* full history: [docs/benchmarks.md](docs/benchmarks.md)

## [0.45.0](https://github.com/jwallace145/rusty-chess-v2/compare/v0.44.4...v0.45.0) (2026-11-05)


### Features

* **nnue:** embed net7rl012, net7's recipe on relabelled generations 0-2, as the default ([#202](https://github.com/jwallace145/rusty-chess-v2/issues/202)) ([5e59bbd](https://github.com/jwallace145/rusty-chess-v2/commit/5e59bbdabff209895ef69738cc053eac5c4fd154))


### Documentation

* **nnue:** record the training of net7rl012 and its held-out comparison with net7 ([#201](https://github.com/jwallace145/rusty-chess-v2/issues/201)) ([39dffc8](https://github.com/jwallace145/rusty-chess-v2/commit/39dffc87d3de95aaf7f3256d092c6e4c965055b7))

### Benchmarks

* bench signature 976,528, instructions per node 4126, search 1.56M NPS, perft 225.76M NPS (pass (63)), fail-high first 87.8%, EBF 1.84, TT hit 51.9%
* since v0.44.4: fail-high first +0.3 pt, EBF +0.04, instructions per node +0.5%, search NPS −21.4%
* full history: [docs/benchmarks.md](docs/benchmarks.md)

## [0.44.4](https://github.com/jwallace145/rusty-chess-v2/compare/v0.44.3...v0.44.4) (2026-11-05)


### Documentation

* **nnue:** record label sets and which labels each net trained on ([#198](https://github.com/jwallace145/rusty-chess-v2/issues/198)) ([844d473](https://github.com/jwallace145/rusty-chess-v2/commit/844d473dc7023207d16ae73861b0820f6a7b06d1))

### Benchmarks

* bench signature 1,031,896, instructions per node 4106, search 1.98M NPS, perft 287.59M NPS (pass (63)), fail-high first 87.5%, EBF 1.81, TT hit 53.0%
* since v0.44.3: fail-high first +0.0 pt, EBF +0.00, instructions per node +0.0%, search NPS +36.7%
* full history: [docs/benchmarks.md](docs/benchmarks.md)

## [0.44.3](https://github.com/jwallace145/rusty-chess-v2/compare/v0.44.2...v0.44.3) (2026-11-04)


### Documentation

* **nnue:** state that both sprt legs are required to merge ([#195](https://github.com/jwallace145/rusty-chess-v2/issues/195)) ([743faa5](https://github.com/jwallace145/rusty-chess-v2/commit/743faa594e9381e49096624968ed4dfcdfca5367))

### Benchmarks

* bench signature 1,031,896, instructions per node 4106, search 1.45M NPS, perft 202.27M NPS (pass (63)), fail-high first 87.5%, EBF 1.81, TT hit 53.0%
* since v0.44.2: fail-high first +0.0 pt, EBF +0.00, instructions per node +0.0%, search NPS −24.3%
* full history: [docs/benchmarks.md](docs/benchmarks.md)

## [0.44.2](https://github.com/jwallace145/rusty-chess-v2/compare/v0.44.1...v0.44.2) (2026-11-04)


### Documentation

* bring the nnue ledgers up to date and require experiments to update docs ([#193](https://github.com/jwallace145/rusty-chess-v2/issues/193)) ([fade9d9](https://github.com/jwallace145/rusty-chess-v2/commit/fade9d9b94d40d3805c08b28e626847b655011fe))

### Benchmarks

* bench signature 1,031,896, instructions per node 4106, search 1.91M NPS, perft 342.85M NPS (pass (63)), fail-high first 87.5%, EBF 1.81, TT hit 53.0%
* since v0.44.1: fail-high first +0.0 pt, EBF +0.00, instructions per node −0.0%, search NPS −3.9%
* full history: [docs/benchmarks.md](docs/benchmarks.md)

## [0.44.1](https://github.com/jwallace145/rusty-chess-v2/compare/v0.44.0...v0.44.1) (2026-11-02)


### Bug Fixes

* **tools:** relabel shards whose manifests have no checksums ([#187](https://github.com/jwallace145/rusty-chess-v2/issues/187)) ([b3104fb](https://github.com/jwallace145/rusty-chess-v2/commit/b3104fbbf0f95139b68da61f1a23f82e02a119dc))

### Benchmarks

* bench signature 1,031,896, instructions per node 4106, search 1.99M NPS, perft 289.89M NPS (pass (63)), fail-high first 87.5%, EBF 1.81, TT hit 53.0%
* since v0.44.0: fail-high first +0.0 pt, EBF +0.00, instructions per node +0.0%, search NPS +33.2%
* full history: [docs/benchmarks.md](docs/benchmarks.md)

## [0.44.0](https://github.com/jwallace145/rusty-chess-v2/compare/v0.43.0...v0.44.0) (2026-11-02)


### Features

* **nnue:** embed only the default net and list the retired ones with their checksums ([#181](https://github.com/jwallace145/rusty-chess-v2/issues/181)) ([b1f0493](https://github.com/jwallace145/rusty-chess-v2/commit/b1f0493ff62279a957352a4f3a0281d83dca5f59))
* **tools:** add relabel, re-scoring stored self-play shards with any embedded net ([#179](https://github.com/jwallace145/rusty-chess-v2/issues/179)) ([8c2bbcf](https://github.com/jwallace145/rusty-chess-v2/commit/8c2bbcfc42a60549e58e38ff124952d5787077b5))

### Benchmarks

* bench signature 1,031,896, instructions per node 4106, search 1.50M NPS, perft 225.99M NPS (pass (63)), fail-high first 87.5%, EBF 1.81, TT hit 53.0%
* since v0.43.0: fail-high first +0.0 pt, EBF +0.00, instructions per node −0.0%, search NPS −3.6%
* full history: [docs/benchmarks.md](docs/benchmarks.md)

## [0.43.0](https://github.com/jwallace145/rusty-chess-v2/compare/v0.42.1...v0.43.0) (2026-11-02)


### Features

* **nnue:** embed net7, net6k4's recipe on generations 0-5, as the default and generation 6's net ([#180](https://github.com/jwallace145/rusty-chess-v2/issues/180)) ([4e2878e](https://github.com/jwallace145/rusty-chess-v2/commit/4e2878e9b81e4cea8d9ed2679c441fbfb871f8fe))

### Benchmarks

* bench signature 1,031,896, instructions per node 4106, search 1.55M NPS, perft 202.21M NPS (pass (63)), fail-high first 87.5%, EBF 1.81, TT hit 53.0%
* since v0.42.1: fail-high first -0.4 pt, EBF -0.05, instructions per node −0.9%, search NPS +9.4%
* full history: [docs/benchmarks.md](docs/benchmarks.md)

## [0.42.1](https://github.com/jwallace145/rusty-chess-v2/compare/v0.42.0...v0.42.1) (2026-11-01)


### Documentation

* **nnue:** record generation 5 and its audit, and net6k4's SPRT and rating ([#177](https://github.com/jwallace145/rusty-chess-v2/issues/177)) ([1d50071](https://github.com/jwallace145/rusty-chess-v2/commit/1d500712f0231699bd4cd274a145ab2d5999e3c8))

### Benchmarks

* bench signature 1,046,838, instructions per node 4143, search 1.42M NPS, perft 202.21M NPS (pass (63)), fail-high first 87.9%, EBF 1.86, TT hit 51.8%
* since v0.42.0: fail-high first +0.0 pt, EBF +0.00, instructions per node +0.0%, search NPS −9.6%
* full history: [docs/benchmarks.md](docs/benchmarks.md)

## [0.42.0](https://github.com/jwallace145/rusty-chess-v2/compare/v0.41.0...v0.42.0) (2026-11-01)


### Features

* **nnue:** embed net6k4, net6 with 4 mirrored king buckets, as the default and generation 5's net ([#175](https://github.com/jwallace145/rusty-chess-v2/issues/175)) ([540cfca](https://github.com/jwallace145/rusty-chess-v2/commit/540cfcaf1280510755216e358f6bc07a36e61d4e))

### Benchmarks

* bench signature 1,046,838, instructions per node 4143, search 1.57M NPS, perft 200.35M NPS (pass (63)), fail-high first 87.9%, EBF 1.86, TT hit 51.8%
* since v0.41.0: fail-high first +0.6 pt, EBF -0.08, instructions per node +8.8%, search NPS −7.2%
* full history: [docs/benchmarks.md](docs/benchmarks.md)

## [0.41.0](https://github.com/jwallace145/rusty-chess-v2/compare/v0.40.0...v0.41.0) (2026-11-01)


### Features

* **nnue:** read nets with 4 mirrored king buckets, single-view nets unchanged ([#172](https://github.com/jwallace145/rusty-chess-v2/issues/172)) ([1adb797](https://github.com/jwallace145/rusty-chess-v2/commit/1adb7978372c754c6a81fd3eb2c73dfd7cb6878d))
* **trainer:** train nets with 4 mirrored king buckets and a factoriser ([#173](https://github.com/jwallace145/rusty-chess-v2/issues/173)) ([ca9a87a](https://github.com/jwallace145/rusty-chess-v2/commit/ca9a87aa4d4d847bc1d9c4dec914bd358aa7e570))

### Benchmarks

* bench signature 1,099,777, instructions per node 3807, search 1.69M NPS, perft 198.10M NPS (pass (63)), fail-high first 87.3%, EBF 1.94, TT hit 51.6%
* since v0.40.0: fail-high first +0.0 pt, EBF +0.00, instructions per node +0.2%, search NPS −21.2%
* full history: [docs/benchmarks.md](docs/benchmarks.md)

## [0.40.0](https://github.com/jwallace145/rusty-chess-v2/compare/v0.39.2...v0.40.0) (2026-11-01)


### Features

* **nnue:** embed net6, net5b8's recipe on generations 0-4, as the default and generation 5's net ([#170](https://github.com/jwallace145/rusty-chess-v2/issues/170)) ([1905792](https://github.com/jwallace145/rusty-chess-v2/commit/190579269bdc5546381ff14650be4c0cfba66cae))

### Benchmarks

* bench signature 1,099,777, instructions per node 3799, search 2.15M NPS, perft 289.25M NPS (pass (63)), fail-high first 87.3%, EBF 1.94, TT hit 51.6%
* since v0.39.2: fail-high first +0.3 pt, EBF +0.05, instructions per node +0.5%, search NPS +5.0%
* full history: [docs/benchmarks.md](docs/benchmarks.md)

## [0.39.2](https://github.com/jwallace145/rusty-chess-v2/compare/v0.39.1...v0.39.2) (2026-11-01)


### Documentation

* **nnue:** record net5b8's SPRT, generation 4 and v0.38.0's rating in the ledger and roadmap ([#168](https://github.com/jwallace145/rusty-chess-v2/issues/168)) ([743326f](https://github.com/jwallace145/rusty-chess-v2/commit/743326f149382e2a1af207a2b86feb9ed3126c98))

### Benchmarks

* bench signature 1,048,958, instructions per node 3779, search 2.04M NPS, perft 290.79M NPS (pass (63)), fail-high first 87.0%, EBF 1.89, TT hit 52.7%
* since v0.39.1: fail-high first +0.0 pt, EBF +0.00, instructions per node −0.0%, search NPS +0.8%
* full history: [docs/benchmarks.md](docs/benchmarks.md)

## [0.39.1](https://github.com/jwallace145/rusty-chess-v2/compare/v0.39.0...v0.39.1) (2026-11-01)


### Documentation

* **datagen:** record generation 4, its spot interruption, top-up and full audit ([#165](https://github.com/jwallace145/rusty-chess-v2/issues/165)) ([2f01b2f](https://github.com/jwallace145/rusty-chess-v2/commit/2f01b2f5efd91c9ca9d07b3fe4ad6718f287553b))

### Benchmarks

* bench signature 1,048,958, instructions per node 3779, search 2.03M NPS, perft 272.36M NPS (pass (63)), fail-high first 87.0%, EBF 1.89, TT hit 52.7%
* since v0.39.0: fail-high first +0.0 pt, EBF +0.00, instructions per node +0.0%, search NPS −27.1%
* full history: [docs/benchmarks.md](docs/benchmarks.md)

## [0.39.0](https://github.com/jwallace145/rusty-chess-v2/compare/v0.38.0...v0.39.0) (2026-11-01)


### Features

* **nnue:** embed net5b8, net 5 with 8 output buckets, as the default and generation 5's net ([#164](https://github.com/jwallace145/rusty-chess-v2/issues/164)) ([86d16aa](https://github.com/jwallace145/rusty-chess-v2/commit/86d16aace42eb3880a5ced563ea3bb4a4f377806))

### Benchmarks

* bench signature 1,048,958, instructions per node 3779, search 2.78M NPS, perft 361.44M NPS (pass (63)), fail-high first 87.0%, EBF 1.89, TT hit 52.7%
* since v0.38.0: fail-high first -0.4 pt, EBF +0.07, instructions per node +0.8%, search NPS +37.5%
* full history: [docs/benchmarks.md](docs/benchmarks.md)

## [0.38.0](https://github.com/jwallace145/rusty-chess-v2/compare/v0.37.1...v0.38.0) (2026-11-01)


### Features

* **nnue:** read nets with 8 output buckets chosen by piece count, single-bucket nets unchanged ([#162](https://github.com/jwallace145/rusty-chess-v2/issues/162)) ([3be2f5b](https://github.com/jwallace145/rusty-chess-v2/commit/3be2f5badd7abaeaf05f8e27e9e3bdc4f09966a1))
* **trainer:** train nets with 8 output buckets chosen by piece count ([#163](https://github.com/jwallace145/rusty-chess-v2/issues/163)) ([ca4c17a](https://github.com/jwallace145/rusty-chess-v2/commit/ca4c17a297c125750c2560eb8aee127190e5dd04))


### Documentation

* **nnue:** record net5x120, dropped on an inconclusive SPRT, and the next experiments ([#160](https://github.com/jwallace145/rusty-chess-v2/issues/160)) ([1a6c3d3](https://github.com/jwallace145/rusty-chess-v2/commit/1a6c3d345f6b4d39923ea3454b2d14fd29a70ac7))

### Benchmarks

* bench signature 1,101,129, instructions per node 3749, search 2.02M NPS, perft 287.34M NPS (pass (63)), fail-high first 87.5%, EBF 1.82, TT hit 54.1%
* since v0.37.1: fail-high first +0.0 pt, EBF +0.00, instructions per node +0.6%, search NPS +17.8%
* full history: [docs/benchmarks.md](docs/benchmarks.md)

## [0.37.1](https://github.com/jwallace145/rusty-chess-v2/compare/v0.37.0...v0.37.1) (2026-10-31)


### Documentation

* **nnue:** bring the ledgers, roadmap and nnue docs up to v0.37.0 and net 5 ([#157](https://github.com/jwallace145/rusty-chess-v2/issues/157)) ([1f450ed](https://github.com/jwallace145/rusty-chess-v2/commit/1f450ed9e0501e7f60a6ea881b9b19e7a91fea4f))

### Benchmarks

* bench signature 1,101,129, instructions per node 3725, search 1.72M NPS, perft 203.66M NPS (pass (63)), fail-high first 87.5%, EBF 1.82, TT hit 54.1%
* since v0.37.0: fail-high first +0.0 pt, EBF +0.00, instructions per node +0.0%, search NPS +2.2%
* full history: [docs/benchmarks.md](docs/benchmarks.md)

## [0.37.0](https://github.com/jwallace145/rusty-chess-v2/compare/v0.36.0...v0.37.0) (2026-10-31)


### Features

* **nnue:** embed net 5, 512 hidden on generations 0-3, as the default and generation 4's net ([#155](https://github.com/jwallace145/rusty-chess-v2/issues/155)) ([cd69a05](https://github.com/jwallace145/rusty-chess-v2/commit/cd69a051215304eb7ea6120084da9aebfcf9989d))

### Benchmarks

* bench signature 1,101,129, instructions per node 3725, search 1.68M NPS, perft 199.61M NPS (pass (63)), fail-high first 87.5%, EBF 1.82, TT hit 54.1%
* since v0.36.0: fail-high first -0.6 pt, EBF -0.09, instructions per node −2.0%, search NPS +4.9%
* full history: [docs/benchmarks.md](docs/benchmarks.md)

## [0.36.0](https://github.com/jwallace145/rusty-chess-v2/compare/v0.35.0...v0.36.0) (2026-10-31)


### Features

* **nnue:** read 512-neuron nets and embed net4w512, 512 hidden on net 4's data, as the default ([#152](https://github.com/jwallace145/rusty-chess-v2/issues/152)) ([08c8b1f](https://github.com/jwallace145/rusty-chess-v2/commit/08c8b1f898dcae82b986331723896a703d77388f))

### Benchmarks

* bench signature 1,203,006, instructions per node 3803, search 1.60M NPS, perft 203.02M NPS (pass (63)), fail-high first 88.0%, EBF 1.91, TT hit 51.2%
* since v0.35.0: fail-high first +0.2 pt, EBF +0.05, instructions per node +12.5%, search NPS −12.7%
* full history: [docs/benchmarks.md](docs/benchmarks.md)

## [0.35.0](https://github.com/jwallace145/rusty-chess-v2/compare/v0.34.0...v0.35.0) (2026-10-31)


### Features

* **nnue:** embed net 4, 256 hidden on net 3's data, as the default and generation 3's net ([#150](https://github.com/jwallace145/rusty-chess-v2/issues/150)) ([ddc636c](https://github.com/jwallace145/rusty-chess-v2/commit/ddc636c4ee5b5abff290dd091ceb697fcde77333))

### Benchmarks

* bench signature 1,190,429, instructions per node 3381, search 1.83M NPS, perft 202.02M NPS (pass (63)), fail-high first 87.9%, EBF 1.86, TT hit 53.6%
* since v0.34.0: fail-high first -0.7 pt, EBF -0.01, instructions per node +8.6%, search NPS −22.8%
* full history: [docs/benchmarks.md](docs/benchmarks.md)

## [0.34.0](https://github.com/jwallace145/rusty-chess-v2/compare/v0.33.0...v0.34.0) (2026-10-31)


### Features

* **nnue:** read 256-neuron nets beside 128, exact at both widths, with the 128 path unchanged ([#149](https://github.com/jwallace145/rusty-chess-v2/issues/149)) ([5ac652c](https://github.com/jwallace145/rusty-chess-v2/commit/5ac652c5b382ecc29d8f369909c1243666c10fdb))
* **trainer:** train nets of any hidden width, and skip the engine check for widths it doesn't read ([#146](https://github.com/jwallace145/rusty-chess-v2/issues/146)) ([f232622](https://github.com/jwallace145/rusty-chess-v2/commit/f232622fe24f6a9338150b3a6d574f1ad54f03dc))

### Benchmarks

* bench signature 1,128,154, instructions per node 3114, search 2.38M NPS, perft 200.84M NPS (pass (63)), fail-high first 88.6%, EBF 1.88, TT hit 52.8%
* since v0.33.0: fail-high first +0.0 pt, EBF +0.00, instructions per node +1.3%, search NPS −36.2%
* full history: [docs/benchmarks.md](docs/benchmarks.md)

## [0.33.0](https://github.com/jwallace145/rusty-chess-v2/compare/v0.32.0...v0.33.0) (2026-10-31)


### Features

* **nnue:** embed net 3, trained on generations 0-2, as the default and generation 3's net ([#145](https://github.com/jwallace145/rusty-chess-v2/issues/145)) ([47fc6bc](https://github.com/jwallace145/rusty-chess-v2/commit/47fc6bc171ae7aa86654f2e33cf2c7571f0d8826))

### Benchmarks

* bench signature 1,128,154, instructions per node 3075, search 3.72M NPS, perft 366.92M NPS (pass (63)), fail-high first 88.6%, EBF 1.88, TT hit 52.8%
* since v0.32.0: fail-high first +0.1 pt, EBF -0.02, instructions per node −1.2%, search NPS +77.3%
* full history: [docs/benchmarks.md](docs/benchmarks.md)

## [0.32.0](https://github.com/jwallace145/rusty-chess-v2/compare/v0.31.1...v0.32.0) (2026-10-31)


### Features

* **tools:** add local engine battles with EngineBattle against stockfish and other uci engines ([#141](https://github.com/jwallace145/rusty-chess-v2/issues/141)) ([76eb3c6](https://github.com/jwallace145/rusty-chess-v2/commit/76eb3c61a691497f4b56034c9a9420629a61d8c3))
* **tools:** audit whole generations by inventory and determinism, and record the net in manifests ([#143](https://github.com/jwallace145/rusty-chess-v2/issues/143)) ([66818d8](https://github.com/jwallace145/rusty-chess-v2/commit/66818d8fb96c076439659a33ef3b20b205a83e15))


### Documentation

* **nnue:** record the λ sweep, net2x80's sprt and ladder, and what they taught us ([#144](https://github.com/jwallace145/rusty-chess-v2/issues/144)) ([cd0ba7b](https://github.com/jwallace145/rusty-chess-v2/commit/cd0ba7be14c04fbef36b0b16b958d5027203ed0c))

### Benchmarks

* bench signature 1,146,589, instructions per node 3113, search 2.10M NPS, perft 202.83M NPS (pass (63)), fail-high first 88.4%, EBF 1.89, TT hit 54.2%
* since v0.31.1: fail-high first +0.0 pt, EBF +0.00, instructions per node −0.0%, search NPS −24.7%
* full history: [docs/benchmarks.md](docs/benchmarks.md)

## [0.31.1](https://github.com/jwallace145/rusty-chess-v2/compare/v0.31.0...v0.31.1) (2026-10-31)


### Performance

* **trainer:** parse shards in parallel in prepare, keeping its output byte-identical ([#138](https://github.com/jwallace145/rusty-chess-v2/issues/138)) ([88d3b1f](https://github.com/jwallace145/rusty-chess-v2/commit/88d3b1f64e73e87f3ff1dbd17d441aa3f45acb7d))

### Benchmarks

* bench signature 1,146,589, instructions per node 3113, search 2.79M NPS, perft 292.75M NPS (pass (63)), fail-high first 88.4%, EBF 1.89, TT hit 54.2%
* since v0.31.0: fail-high first +0.0 pt, EBF +0.00, instructions per node +0.0%, search NPS −7.1%
* full history: [docs/benchmarks.md](docs/benchmarks.md)

## [0.31.0](https://github.com/jwallace145/rusty-chess-v2/compare/v0.30.0...v0.31.0) (2026-10-30)


### Features

* **nnue:** make net2x80, net 2 trained twice as long, the default for generation 2 ([#134](https://github.com/jwallace145/rusty-chess-v2/issues/134)) ([cee5a01](https://github.com/jwallace145/rusty-chess-v2/commit/cee5a01334667fffd01d99d17d4deddf53fb0559))


### Documentation

* **training:** diagnose disk-bound training and correct the learning-rate schedule ([#132](https://github.com/jwallace145/rusty-chess-v2/issues/132)) ([8b284a2](https://github.com/jwallace145/rusty-chess-v2/commit/8b284a2aa8cb6f5a853e3f6c27a7706f99d10bab))

### Benchmarks

* bench signature 1,146,589, instructions per node 3113, search 3.00M NPS, perft 290.54M NPS (pass (63)), fail-high first 88.4%, EBF 1.89, TT hit 54.2%
* since v0.30.0: fail-high first -0.2 pt, EBF +0.01, instructions per node +1.2%, search NPS +39.3%
* full history: [docs/benchmarks.md](docs/benchmarks.md)

## [0.30.0](https://github.com/jwallace145/rusty-chess-v2/compare/v0.29.0...v0.30.0) (2026-10-30)


### Features

* **nnue:** embed net 2 as the default and keep net 1 selectable by id ([#129](https://github.com/jwallace145/rusty-chess-v2/issues/129)) ([83ca209](https://github.com/jwallace145/rusty-chess-v2/commit/83ca209d8b30d4bda0b439d8e4aa2c61203bcc92))


### Documentation

* **nnue:** start the dataset and net ledgers and bring phase 6 up to date ([#130](https://github.com/jwallace145/rusty-chess-v2/issues/130)) ([c026924](https://github.com/jwallace145/rusty-chess-v2/commit/c02692438dd83703629753cd50d3707779d5a6aa))

### Benchmarks

* bench signature 1,120,617, instructions per node 3076, search 2.16M NPS, perft 200.90M NPS (pass (63)), fail-high first 88.6%, EBF 1.88, TT hit 54.3%
* since v0.29.0: fail-high first -0.2 pt, EBF -0.01, instructions per node +1.3%, search NPS +14.5%
* full history: [docs/benchmarks.md](docs/benchmarks.md)

## [0.29.0](https://github.com/jwallace145/rusty-chess-v2/compare/v0.28.0...v0.29.0) (2026-10-30)


### Features

* **trainer:** score every checkpoint's validation loss and add a validate subcommand ([#121](https://github.com/jwallace145/rusty-chess-v2/issues/121)) ([897be30](https://github.com/jwallace145/rusty-chess-v2/commit/897be300997a4dcabbea9d786fa7043e227d181b))


### Performance

* **nnue:** vectorizable i32 output and one-pass accumulator updates ([#125](https://github.com/jwallace145/rusty-chess-v2/issues/125)) ([6b8d96b](https://github.com/jwallace145/rusty-chess-v2/commit/6b8d96b65c7651a81b83e06a04e670e2a48544bb))

### Benchmarks

* bench signature 1,201,313, instructions per node 3037, search 1.88M NPS, perft 222.91M NPS (pass (63)), fail-high first 88.8%, EBF 1.90, TT hit 53.1%
* since v0.28.0: fail-high first +0.0 pt, EBF +0.00, instructions per node −14.3%, search NPS −10.7%
* full history: [docs/benchmarks.md](docs/benchmarks.md)

## [0.28.0](https://github.com/jwallace145/rusty-chess-v2/compare/v0.27.0...v0.28.0) (2026-10-30)


### Features

* **nnue:** embed net 1 and make it the default evaluation ([#122](https://github.com/jwallace145/rusty-chess-v2/issues/122)) ([97c6f36](https://github.com/jwallace145/rusty-chess-v2/commit/97c6f3696a512f7a6c26520b171df19c74b037d6))


### Bug Fixes

* **ci:** export CUDA_PATH for bullet's CUDA build on gpu tasks ([#119](https://github.com/jwallace145/rusty-chess-v2/issues/119)) ([1505e24](https://github.com/jwallace145/rusty-chess-v2/commit/1505e240ac1db45f0c55a74735e2762f0c7cde7f))

### Benchmarks

* bench signature 1,201,313, instructions per node 3543, search 2.11M NPS, perft 235.87M NPS (pass (63)), fail-high first 88.8%, EBF 1.90, TT hit 53.1%
* since v0.27.0: fail-high first +0.5 pt, EBF -0.00, instructions per node +30.2%, search NPS −39.3%
* full history: [docs/benchmarks.md](docs/benchmarks.md)

## [0.27.0](https://github.com/jwallace145/rusty-chess-v2/compare/v0.26.0...v0.27.0) (2026-10-30)


### Features

* **nnue:** add the bullet trainer, the training task and the engine-to-trainer net check ([#115](https://github.com/jwallace145/rusty-chess-v2/issues/115)) ([2bd5d97](https://github.com/jwallace145/rusty-chess-v2/commit/2bd5d97df72595f2c49a6557004605816b9994bd))
* **tools:** add datagen-audit and harden datagen with checksums, tests and tuned settings ([#113](https://github.com/jwallace145/rusty-chess-v2/issues/113)) ([0c02524](https://github.com/jwallace145/rusty-chess-v2/commit/0c02524a8e77a82bfeaacf4a8d83ef073642a7a2))


### Documentation

* **roadmap:** close phase 4b, record phase 6 progress and the plan after net 1 ([#118](https://github.com/jwallace145/rusty-chess-v2/issues/118)) ([12d4e9c](https://github.com/jwallace145/rusty-chess-v2/commit/12d4e9c927c23aab1b7a1c07669e6939a4ac3601))

### Benchmarks

* bench signature 1,697,503, instructions per node 2721, search 3.47M NPS, perft 291.70M NPS (pass (63)), fail-high first 88.2%, EBF 1.90, TT hit 53.5%
* since v0.26.0: fail-high first +0.0 pt, EBF +0.00, instructions per node +0.0%, search NPS +23.5%
* full history: [docs/benchmarks.md](docs/benchmarks.md)

## [0.26.0](https://github.com/jwallace145/rusty-chess-v2/compare/v0.25.0...v0.26.0) (2026-10-30)


### Features

* **eval:** add nnue inference with an incremental accumulator and the evalfile option ([#112](https://github.com/jwallace145/rusty-chess-v2/issues/112)) ([cccdc0e](https://github.com/jwallace145/rusty-chess-v2/commit/cccdc0e50e11603a2de0edb05f85cc25f92569c6))


### Bug Fixes

* **datagen:** upload shards in parallel batches so a 64-core instance never falls behind ([#110](https://github.com/jwallace145/rusty-chess-v2/issues/110)) ([3e3bdbd](https://github.com/jwallace145/rusty-chess-v2/commit/3e3bdbdca83d522411fecbdc75ea5f524d9a803a))

### Benchmarks

* bench signature 1,697,503, instructions per node 2721, search 2.81M NPS, perft 203.34M NPS (pass (63)), fail-high first 88.2%, EBF 1.90, TT hit 53.5%
* since v0.25.0: fail-high first +0.0 pt, EBF +0.00, instructions per node +2.1%, search NPS +11.1%
* full history: [docs/benchmarks.md](docs/benchmarks.md)

## [0.25.0](https://github.com/jwallace145/rusty-chess-v2/compare/v0.24.0...v0.25.0) (2026-10-30)


### Features

* **uci:** ponder on the opponent's time with go ponder and ponderhit ([#108](https://github.com/jwallace145/rusty-chess-v2/issues/108)) ([af669ec](https://github.com/jwallace145/rusty-chess-v2/commit/af669ec9357a300b2cf197a1263f217ebe461235))

### Benchmarks

* bench signature 1,697,503, instructions per node 2664, search 2.53M NPS, perft 200.71M NPS (pass (63)), fail-high first 88.2%, EBF 1.90, TT hit 53.5%
* since v0.24.0: fail-high first +0.0 pt, EBF +0.00, instructions per node −0.1%, search NPS −49.2%
* full history: [docs/benchmarks.md](docs/benchmarks.md)

## [0.24.0](https://github.com/jwallace145/rusty-chess-v2/compare/v0.23.0...v0.24.0) (2026-10-30)


### Features

* **search:** adjust late move reductions by the quiet move's history ([#101](https://github.com/jwallace145/rusty-chess-v2/issues/101)) ([3b6eab6](https://github.com/jwallace145/rusty-chess-v2/commit/3b6eab6330d3d5d97f3558687c9ef65468859055))
* **tools:** add self-play data generation for nnue with shards uploaded to s3 ([#103](https://github.com/jwallace145/rusty-chess-v2/issues/103)) ([c657f78](https://github.com/jwallace145/rusty-chess-v2/commit/c657f78e56efbc22ce359c61522142c6fd385136))

### Benchmarks

* bench signature 1,697,503, instructions per node 2667, search 4.98M NPS, perft 364.87M NPS (pass (63)), fail-high first 88.2%, EBF 1.90, TT hit 53.5%
* since v0.23.0: fail-high first -0.2 pt, EBF +0.09, instructions per node +1.7%, search NPS +89.7%
* full history: [docs/benchmarks.md](docs/benchmarks.md)

## [0.23.0](https://github.com/jwallace145/rusty-chess-v2/compare/v0.22.0...v0.23.0) (2026-10-30)


### Features

* **search:** extend singular table moves a ply and cut nodes where several moves clear beta ([#99](https://github.com/jwallace145/rusty-chess-v2/issues/99)) ([86e9e97](https://github.com/jwallace145/rusty-chess-v2/commit/86e9e97c902f99dc98fee83db72eaf301586d4f9))


### Documentation

* record phase 4b results and drop the improving heuristic until nnue ([#98](https://github.com/jwallace145/rusty-chess-v2/issues/98)) ([e75f760](https://github.com/jwallace145/rusty-chess-v2/commit/e75f76072c58c064c3ea293f3c279fe306834089))

### Benchmarks

* bench signature 1,529,684, instructions per node 2621, search 2.62M NPS, perft 203.09M NPS (pass (63)), fail-high first 88.5%, EBF 1.81, TT hit 55.0%
* since v0.22.0: fail-high first -0.5 pt, EBF +0.22, instructions per node +0.2%, search NPS −11.2%
* full history: [docs/benchmarks.md](docs/benchmarks.md)

## [0.22.0](https://github.com/jwallace145/rusty-chess-v2/compare/v0.21.0...v0.22.0) (2026-10-30)


### Features

* **search:** skip quiet moves that lose material by static exchange at shallow depth ([#93](https://github.com/jwallace145/rusty-chess-v2/issues/93)) ([2e2fc69](https://github.com/jwallace145/rusty-chess-v2/commit/2e2fc6926e941100b8c9e97c20295e4abcde1474))

### Benchmarks

* bench signature 1,075,807, instructions per node 2617, search 2.96M NPS, perft 228.21M NPS (pass (63)), fail-high first 89.0%, EBF 1.59, TT hit 60.7%
* since v0.21.0: fail-high first -0.8 pt, EBF -0.19, instructions per node +3.0%, search NPS −15.1%
* full history: [docs/benchmarks.md](docs/benchmarks.md)

## [0.21.0](https://github.com/jwallace145/rusty-chess-v2/compare/v0.20.0...v0.21.0) (2026-10-30)


### Features

* **search:** search nodes without a table move one ply shallower (internal iterative reductions) ([#89](https://github.com/jwallace145/rusty-chess-v2/issues/89)) ([90a9023](https://github.com/jwallace145/rusty-chess-v2/commit/90a9023b97daebbb7df6f74fc49cff108cb52e8b))

### Benchmarks

* bench signature 1,416,355, instructions per node 2541, search 3.48M NPS, perft 265.07M NPS (pass (63)), fail-high first 89.9%, EBF 1.78, TT hit 57.0%
* since v0.20.0: fail-high first -0.7 pt, EBF +0.14, instructions per node +0.0%, search NPS +11.8%
* full history: [docs/benchmarks.md](docs/benchmarks.md)

## [0.20.0](https://github.com/jwallace145/rusty-chess-v2/compare/v0.19.0...v0.20.0) (2026-10-30)


### Features

* **infra:** add the task-runner component for remote ci tasks ([#85](https://github.com/jwallace145/rusty-chess-v2/issues/85)) ([da17ffb](https://github.com/jwallace145/rusty-chess-v2/commit/da17ffb02bbf281016f8c57a401c29466e15af5e))
* **search:** add futility pruning of quiet moves at shallow null-window nodes ([#81](https://github.com/jwallace145/rusty-chess-v2/issues/81)) ([22d6ac7](https://github.com/jwallace145/rusty-chess-v2/commit/22d6ac7a22eab5b613442ef8ee289dc78682cefd))

### Benchmarks

* bench signature 1,717,816, instructions per node 2540, search 3.11M NPS, perft 201.89M NPS (pass (63)), fail-high first 90.6%, EBF 1.64, TT hit 53.3%
* since v0.19.0: fail-high first +0.4 pt, EBF -0.13, instructions per node +7.2%, search NPS +1.9%
* full history: [docs/benchmarks.md](docs/benchmarks.md)

## [0.19.0](https://github.com/jwallace145/rusty-chess-v2/compare/v0.18.0...v0.19.0) (2026-10-29)


### Features

* **lichess:** play the bot's openings from the lichess masters opening explorer ([#86](https://github.com/jwallace145/rusty-chess-v2/issues/86)) ([17841eb](https://github.com/jwallace145/rusty-chess-v2/commit/17841eb0141c109e05e6db1ad64021596d892796))


### Performance

* **search:** store continuation history scores as i16 to halve the tables ([#80](https://github.com/jwallace145/rusty-chess-v2/issues/80)) ([fb44489](https://github.com/jwallace145/rusty-chess-v2/commit/fb444894e3da58ec08d3701d0688cc6f2d8fd591))

### Benchmarks

* bench signature 1,991,209, instructions per node 2370, search 3.05M NPS, perft 290.41M NPS (pass (63)), fail-high first 90.2%, EBF 1.77, TT hit 50.1%
* since v0.18.0: fail-high first +0.0 pt, EBF +0.00, instructions per node −5.0%, search NPS −38.3%
* full history: [docs/benchmarks.md](docs/benchmarks.md)

## [0.18.0](https://github.com/jwallace145/rusty-chess-v2/compare/v0.17.0...v0.18.0) (2026-10-29)


### Features

* **search:** order quiet moves with continuation history one and two plies back ([#79](https://github.com/jwallace145/rusty-chess-v2/issues/79)) ([a3ee221](https://github.com/jwallace145/rusty-chess-v2/commit/a3ee221e3bd7325254b7324f683187e5e50af70a))


### Documentation

* close phase 4, add the phase 4b search list and the book and ccrl items to phase 5 ([#78](https://github.com/jwallace145/rusty-chess-v2/issues/78)) ([849d2cb](https://github.com/jwallace145/rusty-chess-v2/commit/849d2cb2e96a21994aaf6532e6179b79268f5fc0))
* tick the quiescence items with their sprt results and releases ([#75](https://github.com/jwallace145/rusty-chess-v2/issues/75)) ([e52e0cb](https://github.com/jwallace145/rusty-chess-v2/commit/e52e0cbe989906d1965b02360eb091f58e3481dc))

### Benchmarks

* bench signature 1,991,209, instructions per node 2495, search 4.95M NPS, perft 364.67M NPS (pass (63)), fail-high first 90.2%, EBF 1.77, TT hit 50.1%
* since v0.17.0: fail-high first -0.3 pt, EBF -0.05, instructions per node +11.7%, search NPS −13.0%
* full history: [docs/benchmarks.md](docs/benchmarks.md)

## [0.17.0](https://github.com/jwallace145/rusty-chess-v2/compare/v0.16.0...v0.17.0) (2026-10-28)


### Features

* **uci:** add a move overhead option and keep it back from every time budget ([#71](https://github.com/jwallace145/rusty-chess-v2/issues/71)) ([bc5939f](https://github.com/jwallace145/rusty-chess-v2/commit/bc5939f25adf0400d2b2244474f3e2a1abf5f828))

### Benchmarks

* bench signature 2,077,084, instructions per node 2233, search 5.69M NPS, perft 374.28M NPS (pass (63)), fail-high first 90.5%, EBF 1.82, TT hit 50.1%
* since v0.16.0: fail-high first +0.0 pt, EBF +0.00, instructions per node −0.0%, search NPS +35.6%
* full history: [docs/benchmarks.md](docs/benchmarks.md)

## [0.16.0](https://github.com/jwallace145/rusty-chess-v2/compare/v0.15.0...v0.16.0) (2026-10-28)


### Features

* **search:** probe and store the transposition table in quiescence ([#66](https://github.com/jwallace145/rusty-chess-v2/issues/66)) ([4c5c5b1](https://github.com/jwallace145/rusty-chess-v2/commit/4c5c5b13842120ff959b7d24ef99fead1f4e7f6a))

### Benchmarks

* bench signature 2,077,084, instructions per node 2233, search 4.20M NPS, perft 360.44M NPS (pass (63)), fail-high first 90.5%, EBF 1.82, TT hit 50.1%
* since v0.15.0: fail-high first +0.6 pt, EBF -0.20, instructions per node +3.2%, search NPS +13.3%
* full history: [docs/benchmarks.md](docs/benchmarks.md)

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
