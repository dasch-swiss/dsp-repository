# Changelog

## [0.4.0](https://github.com/dasch-swiss/dsp-repository/compare/v0.3.1...v0.4.0) (2026-05-04)


### Features

* **dpe-server:** add MSSL (086A) project metadata ([d535812](https://github.com/dasch-swiss/dsp-repository/commit/d5358124e965f5f2d83e4f0c8747164b85a478d4))
* Project JSON API (DEV-6224) ([2ae503a](https://github.com/dasch-swiss/dsp-repository/commit/2ae503a2251690c099f6d98aabd4e3bdcb49a009))


### Bug Fixes

* **dpe-server:** add dsp-app data link to nietzsche-me (DEV-6293) ([76ba783](https://github.com/dasch-swiss/dsp-repository/commit/76ba7839d6c9066e1aede92a8681a80d9a9fa083))
* **dpe-server:** add MSSL (086A) project image (DEV-6279) ([#200](https://github.com/dasch-swiss/dsp-repository/issues/200)) ([68b6450](https://github.com/dasch-swiss/dsp-repository/commit/68b6450010a0be44e3c9963f92bf3de6f803602c))
* **dpe-server:** normalize MSSL howToCite formatting and spelling ([046e9d1](https://github.com/dasch-swiss/dsp-repository/commit/046e9d134c7e4f140c36219a19065837141fa6ca))
* **dpe-server:** restore source spellings on MSSL (086A) ([aab86fc](https://github.com/dasch-swiss/dsp-repository/commit/aab86fcf3e6ddec6e3d2a81dccbdd01ef6053d36))
* **dpe-server:** use BCP 47 codes for MSSL dataLanguage ([9c9492f](https://github.com/dasch-swiss/dsp-repository/commit/9c9492fe1b7f65eed00056663b61183fd93d25cd))

## [0.3.1](https://github.com/dasch-swiss/dsp-repository/compare/v0.3.0...v0.3.1) (2026-04-15)


### Bug Fixes

* **dpe:** update Fathom excluded domains from dpe to repository subdomain ([3eba4ba](https://github.com/dasch-swiss/dsp-repository/commit/3eba4ba088161c003c6c8bf1215a293fafd88eef))

## [0.3.0](https://github.com/dasch-swiss/dsp-repository/compare/v0.2.1...v0.3.0) (2026-04-12)


### Features

* **dpe-core,dpe-server:** add DPE_SHOW_PLACEHOLDER_VALUES config flag ([41d210f](https://github.com/dasch-swiss/dsp-repository/commit/41d210f21542cca02aba48ee44289b15ec5edb75))
* **dpe-core:** switch dataLanguage from multilingual maps to BCP 47 codes ([2d8eb78](https://github.com/dasch-swiss/dsp-repository/commit/2d8eb781f1b46f2995ccde0982d3e107f976c601))
* **dpe-web:** hide placeholder values in production, show red in dev ([9ab0a8a](https://github.com/dasch-swiss/dsp-repository/commit/9ab0a8ac4cd90d8030de132b2e3e4f3cd733e98c))


### Bug Fixes

* **dpe-server:** apply post-migration metadata corrections ([44485f8](https://github.com/dasch-swiss/dsp-repository/commit/44485f8c7b59c0d8e5aa8b72cf255f63ca52f441))
* **dpe-server:** sync project images from dsp-app ([40e1b28](https://github.com/dasch-swiss/dsp-repository/commit/40e1b280d224db02e5271fd3d4f7ea6da63e8512))


### Documentation

* add observability guide, update project structure and conventions ([289d4fe](https://github.com/dasch-swiss/dsp-repository/commit/289d4fe8b0b956c9da8ddf10beb59b74a6459e8f))
* add security page and update deployment docs for CI changes ([543f402](https://github.com/dasch-swiss/dsp-repository/commit/543f402fd65d82badf2a6a080ca81ff1285b2cfd))
* **dpe:** document DPE_SHOW_PLACEHOLDER_VALUES env var ([a064022](https://github.com/dasch-swiss/dsp-repository/commit/a064022d41a642bb0558fad24b0c29ee28bd946b))


### Refactoring

* **dpe-api-oai:** replace hardcoded "MISSING" checks with is_placeholder() ([761fb24](https://github.com/dasch-swiss/dsp-repository/commit/761fb24a5932460df46f1651f4ccfa823a410f14))


### Build System

* add Nix flake devShell for reproducible development environment ([2c22b0f](https://github.com/dasch-swiss/dsp-repository/commit/2c22b0fe1297ffa887ddb1181bef4daffaa2bc4d))
* correct cargo-leptos configuration and justfile watch targets ([b258c69](https://github.com/dasch-swiss/dsp-repository/commit/b258c6995a879e9fec479a948310fb18daf1a0aa))
* **dpe-server:** add OpenTelemetry tracing and browser telemetry ([b85fbcf](https://github.com/dasch-swiss/dsp-repository/commit/b85fbcf2e75d4141969ee4d5f379551804175177))
* **dpe-server:** add Pyroscope continuous profiling and fix tracer name ([b2408d1](https://github.com/dasch-swiss/dsp-repository/commit/b2408d11e2440caca9e17f17cfcd19d075fe90c8))
* **dpe-server:** enable OTel metrics and log export for local dev ([5b4cfbd](https://github.com/dasch-swiss/dsp-repository/commit/5b4cfbdb77065cf150ed639adc1ec74efc0ff214))
* **dpe-telemetry:** add telemetry types crate and fuzz targets ([70fc7d8](https://github.com/dasch-swiss/dsp-repository/commit/70fc7d8be1a37e14858659af039d7e999f93d771))
* fix nix devShell for cargo +nightly fmt and pnpm install ([051e8ce](https://github.com/dasch-swiss/dsp-repository/commit/051e8ce23a33a8ae15f7f983646e5723850b85be))

## [0.2.1](https://github.com/dasch-swiss/dsp-repository/compare/v0.2.0...v0.2.1) (2026-04-02)


### Documentation

* consolidate documentation with single source of truth in docs/ ([1c656b2](https://github.com/dasch-swiss/dsp-repository/commit/1c656b2594fe8e4ecff041b61ad5b557d02f21c4))


### Refactoring

* **dpe-web:** rename modules/dpe/app to modules/dpe/web ([10446f3](https://github.com/dasch-swiss/dsp-repository/commit/10446f3ec038a2773bbc7b1dd1601ea4151ec1e9))
* **mosaic:** rename demo to playground, demo_macro to playground_macro ([fbdc7bc](https://github.com/dasch-swiss/dsp-repository/commit/fbdc7bcce9cd6e64c7f6bfa40eb4f2707784037c))
