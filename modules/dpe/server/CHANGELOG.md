# Changelog

## [0.1.2](https://github.com/dasch-swiss/dsp-repository/compare/server-v0.1.1...server-v0.1.2) (2026-03-27)


### Features

* **design-system:** Add example page ([65e4245](https://github.com/dasch-swiss/dsp-repository/commit/65e4245246795489048b9cde9702908a3c73675c))
* metadata v1-to-v2 migration data and Rust fixes ([#162](https://github.com/dasch-swiss/dsp-repository/issues/162)) ([470c473](https://github.com/dasch-swiss/dsp-repository/commit/470c473fcbfa81e0d295e404645829714e5c92dd))
* OAI-PMH 2.0 endpoint, with goldens and XSD specs ([#131](https://github.com/dasch-swiss/dsp-repository/issues/131)) ([acaf46f](https://github.com/dasch-swiss/dsp-repository/commit/acaf46fb547d0fbc7e3013bafd224e8844207563))
* OAI-PMH with records (DEV-5946) ([#153](https://github.com/dasch-swiss/dsp-repository/issues/153)) ([d6aefdb](https://github.com/dasch-swiss/dsp-repository/commit/d6aefdb676f7628ce0219d87ef714528f1c4a409))
* replace dummy data with migrated v1 metadata (1/2: initial placement) ([#151](https://github.com/dasch-swiss/dsp-repository/issues/151)) ([651dd0f](https://github.com/dasch-swiss/dsp-repository/commit/651dd0f1e6533d75264346bf29b6211859e431e2))
* split BEOL into 5 sub-projects ([#159](https://github.com/dasch-swiss/dsp-repository/issues/159)) ([4420ffc](https://github.com/dasch-swiss/dsp-repository/commit/4420ffc6112bf5345a600b8e8b597398ac4bfae2))


### Bug Fixes

* /project/xxx: return 404, if project is not found (DEV-5664) ([ee58236](https://github.com/dasch-swiss/dsp-repository/commit/ee5823691623942e5ac0669b6dffe0729c435e24))
* correct entity deduplication with reviewed merges ([#157](https://github.com/dasch-swiss/dsp-repository/issues/157)) ([2b16100](https://github.com/dasch-swiss/dsp-repository/commit/2b16100ee8d1dd0d9b5a7d7a1056aeef54822adc))
* rename package `leptos-server` to `server` to match bin-package config ([#160](https://github.com/dasch-swiss/dsp-repository/issues/160)) ([50bad85](https://github.com/dasch-swiss/dsp-repository/commit/50bad854ecc7b75b7179fc1b785bf10130e71ef5))

## [0.1.1](https://github.com/dasch-swiss/dsp-repository/compare/leptos-server-v0.1.0...leptos-server-v0.1.1) (2026-03-11)


### Features

* add leptos-dpe website ([#92](https://github.com/dasch-swiss/dsp-repository/issues/92)) ([87d1f6e](https://github.com/dasch-swiss/dsp-repository/commit/87d1f6e69042727bfb0b958aa8f63ad3f341eb62))
* add new dataset ([#135](https://github.com/dasch-swiss/dsp-repository/issues/135)) ([404530e](https://github.com/dasch-swiss/dsp-repository/commit/404530ef22bb819e9b68f4a9f4a772a06a7ce8cb))
* Use mosaic tiles components for CLAUDE.md ([#129](https://github.com/dasch-swiss/dsp-repository/issues/129)) ([cf971cf](https://github.com/dasch-swiss/dsp-repository/commit/cf971cf9670fe0c86e5db374738c789d64f382a0))


### Bug Fixes

* Add publish = false to leptos-dpe crates (DEV-5960) ([#116](https://github.com/dasch-swiss/dsp-repository/issues/116)) ([e889760](https://github.com/dasch-swiss/dsp-repository/commit/e889760a652e6e50de074dbba27404b37f4e9e2c))
* Add version to all path dependencies for cargo package (DEV-5960) ([#119](https://github.com/dasch-swiss/dsp-repository/issues/119)) ([efdaa7d](https://github.com/dasch-swiss/dsp-repository/commit/efdaa7ddd54843e0fb9dab95f1ced758c7f7c42f))
* Remove publish = false from Cargo.toml to fix GitHub releases (DEV-5960) ([#117](https://github.com/dasch-swiss/dsp-repository/issues/117)) ([9362378](https://github.com/dasch-swiss/dsp-repository/commit/936237841dd9db54602c1c5b29516417d8e8a0e4))
