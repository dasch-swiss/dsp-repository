# Changelog

## [0.9.0](https://github.com/dasch-swiss/dsp-repository/compare/v0.8.0...v0.9.0) (2026-07-15)


### Features

* **dpe-api-oai:** add resumption-token paging to ListRecords and ListIdentifiers (DEV-6684) ([625142d](https://github.com/dasch-swiss/dsp-repository/commit/625142dbd93c1f3aa979054382013d79b81b1730))
* **dpe-api-oai:** expose record file MIME type and download link in OAI output (DEV-6684) ([b281376](https://github.com/dasch-swiss/dsp-repository/commit/b2813760ce37c4333b9d1d94ab81b1178d4333e9))
* **dpe,mosaic:** browser live-reload in the dev loops (DEV-6728) ([8f1283a](https://github.com/dasch-swiss/dsp-repository/commit/8f1283a91adda697aaeaf262fb0ec4ea461e04ce))
* OAI-PMH endpoint rate-limiting (DEV-6724) ([9c38251](https://github.com/dasch-swiss/dsp-repository/commit/9c38251b345c0e002ce3d1963585a6dc7d4bdb16))


### Documentation

* clarify that PRs should default to a single commit ([73c3c17](https://github.com/dasch-swiss/dsp-repository/commit/73c3c172e602177cc1006836543bc7cc1a4563f1))


### Refactoring

* **dpe,mosaic:** remove Leptos — migrate to Maud + Axum + Datastar (DEV-6642) ([b89eaf4](https://github.com/dasch-swiss/dsp-repository/commit/b89eaf4fc1f031d463b6385ef0859c1a7f41ead8))


### Build System

* add cargo-machete unused-dependency check to just check ([29bfa59](https://github.com/dasch-swiss/dsp-repository/commit/29bfa5913d67c65217cf63062372629ec477a6fb))

## [0.8.0](https://github.com/dasch-swiss/dsp-repository/compare/v0.7.1...v0.8.0) (2026-07-06)


### Features

* **dpe-api-oai:** add optional dateInformation to DataCiteDate ([4946a7d](https://github.com/dasch-swiss/dsp-repository/commit/4946a7d171b621b737f8b01d1c4decc8c0d632ce))
* **dpe-api-oai:** resolve temporal coverage to DataCite date ranges ([dbf039e](https://github.com/dasch-swiss/dsp-repository/commit/dbf039e42dac7c8f2a3486e1ba59841677c4b861))
* **dpe-core:** add W3CDTF formatter and ChronOntology period cache ([695a8fd](https://github.com/dasch-swiss/dsp-repository/commit/695a8fdc4197b4b06d0d5e99435f7be72f24cbae))
* **dpe:** add temporal-coverage enrichment tool and table ([0aaa7f1](https://github.com/dasch-swiss/dsp-repository/commit/0aaa7f1d49041226dd5acbbdcbcea03b4d0484cd))


### Bug Fixes

* **dpe-api-oai:** classify DaSCH record creator as Organizational (DEV-6524) ([3675afd](https://github.com/dasch-swiss/dsp-repository/commit/3675afd506b97fb5f2790a1deb5594363b2ac928))
* **dpe-core:** use RKMS-ISO8601 open-range form for temporal coverage ([6d3025a](https://github.com/dasch-swiss/dsp-repository/commit/6d3025a019a120360e55d31dbe79b2726128d8d9))
* **dpe-data:** update Eva Pibiri's job title to Associate Professor ([73a0d20](https://github.com/dasch-swiss/dsp-repository/commit/73a0d209b55eb9e5330666d79606e46e8be4d839))


### Documentation

* add DPE JSON API reference page ([49c0bae](https://github.com/dasch-swiss/dsp-repository/commit/49c0baeee116b98b068b3fe8766266ff567da974))
* add v2 metadata model documentation ([#257](https://github.com/dasch-swiss/dsp-repository/issues/257)) ([7f35d89](https://github.com/dasch-swiss/dsp-repository/commit/7f35d8902a735e85178d01aa04d567ad8322e601))
* **dpe:** correct temporal-coverage resolution description ([0de8062](https://github.com/dasch-swiss/dsp-repository/commit/0de8062583b79185b82aa0bdd532f1e36b6a3be9))


### Refactoring

* **dpe-api-oai:** drop speculative temporal-coverage dedup guard ([3c23873](https://github.com/dasch-swiss/dsp-repository/commit/3c23873a63f02b619d0cad3178150432384f3ae6))
* **dpe-api-oai:** resolve temporal coverage via *_in pure functions ([cc5be13](https://github.com/dasch-swiss/dsp-repository/commit/cc5be132b770c11c6369c625f1dfa7c0399183e5))
* **dpe:** make temporal enrichment fully LLM-generated ([a41d858](https://github.com/dasch-swiss/dsp-repository/commit/a41d85856b6284ccc594cdad368f5d7cb5e3b576))


### Build System

* **deps:** bump the backend-dependencies group across 1 directory with 6 updates ([7b6d2f1](https://github.com/dasch-swiss/dsp-repository/commit/7b6d2f1ef6c0f0f416102342e82dc1a6b861f58f))

## [0.7.1](https://github.com/dasch-swiss/dsp-repository/compare/v0.7.0...v0.7.1) (2026-06-16)


### Bug Fixes

* **dpe-api-oai:** Make OAI-PMH baseURL configurable; correct identifier namespace ([#256](https://github.com/dasch-swiss/dsp-repository/issues/256)) ([319a3a6](https://github.com/dasch-swiss/dsp-repository/commit/319a3a6382066dc270d809a5f4bf73d24e22f83b))
* **dpe-core:** correct inverted temporal/spatial coverage in project data ([#252](https://github.com/dasch-swiss/dsp-repository/issues/252)) ([b5b6a86](https://github.com/dasch-swiss/dsp-repository/commit/b5b6a86faabb03bf6bb1675b59a4babb2f9baf59))
* let Kodus emit branch-protection so assess.py can override it ([b93a07f](https://github.com/dasch-swiss/dsp-repository/commit/b93a07f70cc58d6094f87943f1a7126f95708c03))
* Move project roles out of person job titles; add validate guard (DEV-6626, DEV-6630) ([#253](https://github.com/dasch-swiss/dsp-repository/issues/253)) ([3abcdc4](https://github.com/dasch-swiss/dsp-repository/commit/3abcdc436a0299b7d395953e898f25459ece3bce))

## [0.7.0](https://github.com/dasch-swiss/dsp-repository/compare/v0.6.0...v0.7.0) (2026-06-15)


### Features

* **dpe-api-oai:** Resolve contributor information in OAI-PMH output (DEV-6575) ([c1b9ed2](https://github.com/dasch-swiss/dsp-repository/commit/c1b9ed22936666261939ef5a6a6a695c148bb5a6))
* **oai:** add project and cluster set filters ([dba4f48](https://github.com/dasch-swiss/dsp-repository/commit/dba4f48ba96cbef2d92be00fe18ecbcfb1fbcef3))


### Bug Fixes

* Move remaining 0803 roles from job titles to attributions (DEV-6620) ([f61b9eb](https://github.com/dasch-swiss/dsp-repository/commit/f61b9ebbae03159807f0a912ff0f3f1cbaafb48b))
* Reduce possibility of discrepant ARKs (DEV-6603) ([12ac7bd](https://github.com/dasch-swiss/dsp-repository/commit/12ac7bd961af966081b3cc4b943c961ffe54d117))
* Represent project leader as attribution, not job title (DEV-6620) ([2c49749](https://github.com/dasch-swiss/dsp-repository/commit/2c49749883964391b2a080a6e30bd51a60308943))


### Documentation

* add OAI-PMH endpoint usage page ([0889cd0](https://github.com/dasch-swiss/dsp-repository/commit/0889cd0bc3296b3999920a92e844a5c1a8436e7c))
* document project and cluster OAI set filters (DEV-6526) ([0310fdc](https://github.com/dasch-swiss/dsp-repository/commit/0310fdc5eeb59d906f775686e479091b53da49ce))
* fix stale /oai paths in observability guide ([227aa39](https://github.com/dasch-swiss/dsp-repository/commit/227aa39fefb07fb69c193afbaa0bb6bc4fe2241c))


### Refactoring

* **dpe-core:** drop dead prefixed-organization-id heuristic ([fad0557](https://github.com/dasch-swiss/dsp-repository/commit/fad0557cacef093344694e5289cea947d01b2d97))
* extract cluster reverse-lookup into dpe-core helpers ([1be7b6e](https://github.com/dasch-swiss/dsp-repository/commit/1be7b6ef4f72705c2bb5f03b617f4d0fe6a89c81))

## [0.6.0](https://github.com/dasch-swiss/dsp-repository/compare/v0.5.7...v0.6.0) (2026-06-10)


### Features

* In-memory records in a HashMap, default record cache (DEV-6525) ([f5c1359](https://github.com/dasch-swiss/dsp-repository/commit/f5c1359ffa1d85b9da57b560931c01a2e2f7bd64))


### Bug Fixes

* **dpe-server:** remove duplicated attributions and legalInfo on 0854 and 083B ([9ba056f](https://github.com/dasch-swiss/dsp-repository/commit/9ba056f77385f321556d7d372a7d38bdf2492960))

## [0.5.7](https://github.com/dasch-swiss/dsp-repository/compare/v0.5.6...v0.5.7) (2026-06-08)


### Bug Fixes

* do not render project URL buttons for placeholder values ([1099585](https://github.com/dasch-swiss/dsp-repository/commit/10995851c998790c74e1951da1d9054aabb2c218))

## [0.5.6](https://github.com/dasch-swiss/dsp-repository/compare/v0.5.5...v0.5.6) (2026-06-03)


### Bug Fixes

* **data:** repair malformed JSON in person-412 (Barbara Piatti) ([9618799](https://github.com/dasch-swiss/dsp-repository/commit/961879945af5044db70d031a9627043cf090f1cd))

## [0.5.5](https://github.com/dasch-swiss/dsp-repository/compare/v0.5.4...v0.5.5) (2026-05-26)


### Bug Fixes

* **dpe-server:** correct metadata JSON API path from v1 to v2 ([1843908](https://github.com/dasch-swiss/dsp-repository/commit/1843908972abf7846b1b2949ccafce367cc619a9))
* **mosaic/playground:** upgrade base packages in runtime image to patch CVEs ([e5db763](https://github.com/dasch-swiss/dsp-repository/commit/e5db76335b7fab6a5e271e183350a1b087e3d41b))
* **mosaic/tiles:** harden tailwind download with status check and retries ([2cf1099](https://github.com/dasch-swiss/dsp-repository/commit/2cf10999b6c6349f013fb68517e24674131c58f3))


### Build System

* **deps:** bump rand from 0.9.2 to 0.9.4 ([f0818e8](https://github.com/dasch-swiss/dsp-repository/commit/f0818e8c9cfc9ec9732a632dc7c8c03e1c865ddb))
* **deps:** bump the backend-dependencies group across 1 directory with 10 updates ([0592939](https://github.com/dasch-swiss/dsp-repository/commit/0592939bc2e5cc0cf53a189511613d350cb8833e))

## [0.5.4](https://github.com/dasch-swiss/dsp-repository/compare/v0.5.3...v0.5.4) (2026-05-19)


### Bug Fixes

* **dpe-web:** point access rights filter info link to new dasch.swiss page (DEV-6223) ([acb660f](https://github.com/dasch-swiss/dsp-repository/commit/acb660fa421255005317c8e4d8d05604a0e3ae55))

## [0.5.3](https://github.com/dasch-swiss/dsp-repository/compare/v0.5.2...v0.5.3) (2026-05-07)


### Bug Fixes

* **dpe-server:** avoid duplicate `message` key in browser-error logs ([48be1eb](https://github.com/dasch-swiss/dsp-repository/commit/48be1ebc71f824b71467832c2e3d7c649e120ef9))
* **dpe-web:** resolve remaining domain calls synchronously to stop SSR disposal panics ([09a1fc3](https://github.com/dasch-swiss/dsp-repository/commit/09a1fc39894b7a5ae3a8305a7c4555812ad394f2))
* **mosaic-tiles:** read Icon class once at component creation ([00496b7](https://github.com/dasch-swiss/dsp-repository/commit/00496b7584e344a793a17073a05027028879492f))

## [0.5.2](https://github.com/dasch-swiss/dsp-repository/compare/v0.5.1...v0.5.2) (2026-05-07)


### Bug Fixes

* **dpe-server:** only call default panic hook on structured-emission failure ([f8d5127](https://github.com/dasch-swiss/dsp-repository/commit/f8d5127d6cdee89679141ac672333d78cc22e096))
* **dpe:** match project shortcodes case-insensitively in get_project ([1f485e9](https://github.com/dasch-swiss/dsp-repository/commit/1f485e9fc69203fbf92c16608ddaa6a2c031488b))


### Build System

* **deps:** bump rand from 0.8.5 to 0.8.6 ([019cfbe](https://github.com/dasch-swiss/dsp-repository/commit/019cfbeb1c58d3e40b82ee8e750d80d0bba9567a))
* **deps:** bump rustls-webpki from 0.103.10 to 0.103.13 ([e55f80a](https://github.com/dasch-swiss/dsp-repository/commit/e55f80ab734cd4ed9bc01e454d18664a3ba3a15b))

## [0.5.1](https://github.com/dasch-swiss/dsp-repository/compare/v0.5.0...v0.5.1) (2026-05-07)


### Bug Fixes

* **dpe:** produce useful panic backtraces in production ([07cf69c](https://github.com/dasch-swiss/dsp-repository/commit/07cf69c2320784f1083e57f87e73069823d1c53b))

## [0.5.0](https://github.com/dasch-swiss/dsp-repository/compare/v0.4.0...v0.5.0) (2026-05-07)


### Features

* **dpe-server:** route panics through tracing for structured Grafana logs ([c712106](https://github.com/dasch-swiss/dsp-repository/commit/c712106a0aad9000b89a1004afc921f8582e875e))


### Bug Fixes

* **dpe-web:** redirect /dpe to /dpe/projects ([6bd3e77](https://github.com/dasch-swiss/dsp-repository/commit/6bd3e7784f43f4451543faa74ef950bcdffe4ce5))
* **dpe-web:** resolve sidebar entities synchronously to stop SSR disposal panics ([f78ee42](https://github.com/dasch-swiss/dsp-repository/commit/f78ee427901edfced8a504ab81547c2d11ebd020))
* Make project lookup by shortcode case-insensitive (quick fix) (DEV-6224) ([12beea8](https://github.com/dasch-swiss/dsp-repository/commit/12beea823f38ab29c838dab27bf955111a15d6ae))

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
