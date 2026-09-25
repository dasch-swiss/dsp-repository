# Changelog

## [0.8.7](https://github.com/dasch-swiss/dsp-repository/compare/v0.8.6...v0.8.7) (2026-09-25)


### Features

* **editor-core,editor-collector:** Collect approved records into one pull request per project ([8bc1cea](https://github.com/dasch-swiss/dsp-repository/commit/8bc1cea1b7bcd8054f0a77c21819d9a4aaa5b95c))
* **editor-core,editor-server:** Retire an accepted entity proposal once its entity is published ([47822e6](https://github.com/dasch-swiss/dsp-repository/commit/47822e625537410e8153bac8cb6b9efbe7947ccc))
* **editor-core,editor-web,editor-server:** Show RDU where each approved record's collection stands ([bb5607f](https://github.com/dasch-swiss/dsp-repository/commit/bb5607f4930f111048fac616880c1e2ee2f182cf))


### Bug Fixes

* **analytics:** Pin Fathom to production by config, not by an exclusion list ([b059a84](https://github.com/dasch-swiss/dsp-repository/commit/b059a846b26f6599aee1e2a9bdce5a1264d7bf9f))
* **dpe-data:** Replace the 0803 and 081C covers and credit both ([787ef6b](https://github.com/dasch-swiss/dsp-repository/commit/787ef6b3dd4f28909c8796ca8e8dcb4e63ab3333))
* **dpe-web:** Fall back to the no-JS tab at once on a network failure ([510379a](https://github.com/dasch-swiss/dsp-repository/commit/510379a812d0fff4d0252fd96f5a940c114c4018))
* **dpe,editor:** Report Datastar request failures from datastar-fetch ([74efe86](https://github.com/dasch-swiss/dsp-repository/commit/74efe866d76a39172059d6f91fec5b7a2f04c26a))
* **editor-server,editor-web:** Confirm every no-JavaScript write where screen readers hear it ([373831d](https://github.com/dasch-swiss/dsp-repository/commit/373831d80a05ec1628c5ef2ad6f6bcb0bbff2746))


### Code Refactoring

* **dpe-data:** Pin the corpus size once, count it live everywhere else ([af197df](https://github.com/dasch-swiss/dsp-repository/commit/af197df449651e869a98dd3a8e368ba832796e6d))
* **dpe-server:** Reduce main.rs to a thin wrapper over startup modules ([2d3b21a](https://github.com/dasch-swiss/dsp-repository/commit/2d3b21a374dc193a4b6e5cfebd635e883e402d76))
* **editor-core,editor-server:** Retire list_uncollected and its partial index ([02a6287](https://github.com/dasch-swiss/dsp-repository/commit/02a628792a27e53cf17ae80efdf1b7df7551c270))
* **editor-core,editor-web,editor-server,editor-collector:** Move the editor to areas/deposit/editor ([54480c6](https://github.com/dasch-swiss/dsp-repository/commit/54480c61adc0839f702d9dcf707870b503bf9807))
* **editor-server:** Collapse the schema migrations into one baseline ([985ef59](https://github.com/dasch-swiss/dsp-repository/commit/985ef5910c095b59a1e00f75a581a381d82144e0))
* **editor-server:** Reduce main.rs to a thin wrapper over startup modules ([6d978ad](https://github.com/dasch-swiss/dsp-repository/commit/6d978adab8187daa234882c18c9569ff2709baab))


### Documentation

* **docs:** Add ordered rules for separate PRs, stacks, and multi-commit PRs ([0debe77](https://github.com/dasch-swiss/dsp-repository/commit/0debe7702cf8d97eb3a650962daf1b16d2b7b7e6))


### Miscellaneous Chores

* **ci,editor-collector:** Run the collector from a workflow, and record it ([ad36b1c](https://github.com/dasch-swiss/dsp-repository/commit/ad36b1cc6697da8996f8b6841311a11e9784cf9f))
* **docs,dsp-cli:** Correct stale agent instructions in CLAUDE.md files ([b81801d](https://github.com/dasch-swiss/dsp-repository/commit/b81801dad7f9836773b7055eacd4c15e1c5bf267))
* **docs:** Bind eng conventions, commands and reviewers in eng.yaml ([b6ca81b](https://github.com/dasch-swiss/dsp-repository/commit/b6ca81b44aa170cb192024e3515b7b629f95fe40))
* **dpe-data:** Remove the unused cover images at the repository root ([a3a1b0d](https://github.com/dasch-swiss/dsp-repository/commit/a3a1b0dc015d839ac0e162d92b2b695ca7cc3d3b))
* **dpe-data:** Strip stray whitespace from the committed project files ([6768507](https://github.com/dasch-swiss/dsp-repository/commit/6768507798d62956f2a7ee730aa906037f4d513e))
* **main:** Release dsp-cli 0.3.1 ([ffb09d0](https://github.com/dasch-swiss/dsp-repository/commit/ffb09d0eb41c52599b3b89b7cb31f035d2b2c593))

## [0.8.6](https://github.com/dasch-swiss/dsp-repository/compare/v0.8.5...v0.8.6) (2026-09-22)


### Features

* **dpe-server,dpe-api-oai:** DPE_ARK_RESOLVER_BASE_URL ([7ddba8c](https://github.com/dasch-swiss/dsp-repository/commit/7ddba8ce73ef0c9e7bbfd89ce6f78b3fe1d03393))
* **dpe-server,mosaic-tiles:** Put the metadata and Signposting on the landing page ([161a2ff](https://github.com/dasch-swiss/dsp-repository/commit/161a2ff88ab6eb6826f2388fc3f8b320a06b1905))
* **dpe-server:** Answer the ARKs a deployment publishes ([c51b8e4](https://github.com/dasch-swiss/dsp-repository/commit/c51b8e4557cb9354cf19306545be2a3030a5cba1))
* **dpe-server:** Configure the site's own public base URL ([71b92a8](https://github.com/dasch-swiss/dsp-repository/commit/71b92a829d1d2768b1aa61e4d3ff9f8c4ca88277))
* **dpe-server:** Redirect the landing page to a representation on Accept ([d0406b2](https://github.com/dasch-swiss/dsp-repository/commit/d0406b28a44c82723a4d6edf71d7a1086d805ec5))
* **dpe-server:** Serve the machine-readable representations ([b60d158](https://github.com/dasch-swiss/dsp-repository/commit/b60d158e46ad3189373edbf55db7ba7d4eb8d314))
* **dsp-cli:** Move dsp-cli from dsp-incubator into the workspace ([ca887a9](https://github.com/dasch-swiss/dsp-repository/commit/ca887a9f5a8451434c5429e7979e7a8f9408df53))
* **editor-core,editor-server:** Accept an advisory collection report from CI ([1a2c02b](https://github.com/dasch-swiss/dsp-repository/commit/1a2c02b527b3b5c9eb7ea1c93b1259ce8323a804))
* **editor-core,editor-server:** Publish approved records over a read-only endpoint ([173c013](https://github.com/dasch-swiss/dsp-repository/commit/173c013aa5311685fdc88142b0831815f183f82e))
* **shared-fair,dpe-api-oai:** Resolve the ARK host in the graph ([98184b2](https://github.com/dasch-swiss/dsp-repository/commit/98184b2efc83deebfaa05021d92ac421c30bcfe3))
* **shared-fair,dpe-server,dpe-api-oai:** Give /metadata.jsonld a byte budget ([82274cb](https://github.com/dasch-swiss/dsp-repository/commit/82274cb7378e419fe708d2988206fc8eeccb9767))
* **shared-fair,shared-metadata,dpe-api-oai:** Describe the files a project has ([de6672c](https://github.com/dasch-swiss/dsp-repository/commit/de6672cbccc39b3676c5b6a4573f0e008f283096))
* **shared-fair:** Assert provenance in PROV-O ([6342576](https://github.com/dasch-swiss/dsp-repository/commit/6342576e91b2d4f9cbe6b55462bd4315485770d2))
* **shared-fair:** Build the Signposting link set off the project graph ([0565c96](https://github.com/dasch-swiss/dsp-repository/commit/0565c96c21d2a37c7608240b1e0b765cde7b366e))
* **shared-fair:** Decide the landing page's one negotiation step ([74102e8](https://github.com/dasch-swiss/dsp-repository/commit/74102e883dec6727baa77f0d6ac5b201beb5ae6f))
* **shared-fair:** Render the Dublin Core record as meta tags ([0cdec47](https://github.com/dasch-swiss/dsp-repository/commit/0cdec47f033a65506e0d7fd56625a30db1343727))
* **shared-fair:** Resolve a project's facts once into ProjectGraph ([b1a2752](https://github.com/dasch-swiss/dsp-repository/commit/b1a2752178196ef4ea581d8222ee498a0e7e4573))
* **shared-fair:** Resolve a record's facts once into RecordGraph ([b82fdf2](https://github.com/dasch-swiss/dsp-repository/commit/b82fdf2c2a1497c0594b8bede739d6fdbed14dda))
* **shared-fair:** Write schema.org JSON-LD from the project graph ([5bf0912](https://github.com/dasch-swiss/dsp-repository/commit/5bf0912bfd68db2d77d207c105245220b677efab))
* **shared-fair:** Write the DataCite record as kernel-4 JSON ([db7acfa](https://github.com/dasch-swiss/dsp-repository/commit/db7acfafddd9a022bf803376ea620058ad77a175))


### Bug Fixes

* **dpe-web:** Draw the email envelope with the Mosaic icon ([baaa683](https://github.com/dasch-swiss/dsp-repository/commit/baaa683bc672e0008031ad6f480515d7389489d7))
* **oai:** Add ARK resolver for identifiers in the payload ([78c4a7c](https://github.com/dasch-swiss/dsp-repository/commit/78c4a7c7e3f13dcf610eda9b8c2c9f5ff11db8c2))
* **shared-fair,dpe-api-oai,docs:** Ask the ORCID question once, and stop overclaiming ([1a554e8](https://github.com/dasch-swiss/dsp-repository/commit/1a554e858ed5586b91a5c2245f2a93501eacf2d3))
* **shared-fair,dpe-api-oai:** Emit each licence as an IRI node ([cb18e12](https://github.com/dasch-swiss/dsp-repository/commit/cb18e12b6c6fc844cc2e366e1b6400c8eeb90f11))
* **shared-fair,dpe-api-oai:** Name the landing page in schema.org identifier ([b10328e](https://github.com/dasch-swiss/dsp-repository/commit/b10328ef917c84404bbe5ea8036dd613c577e6dc))
* **shared-fair,dpe-api-oai:** Stop asserting a publication year nobody recorded ([2ccd15e](https://github.com/dasch-swiss/dsp-repository/commit/2ccd15e6ad34e88dc514670a516c121e5f8908df))
* **shared-fair:** Count characters, not bytes, in extract_year ([e1d400e](https://github.com/dasch-swiss/dsp-repository/commit/e1d400e685aa48914a6962e44e417d0ab7cd0e10))


### Performance Improvements

* **dpe-core,dpe-api-oai,dpe-server:** Index the records by shortcode ([f946068](https://github.com/dasch-swiss/dsp-repository/commit/f946068887aab10395a13c10ada9ab8693ba0b00))


### Code Refactoring

* **dpe-core,dpe-api-oai:** Reach the raw project through the repository ([89d5450](https://github.com/dasch-swiss/dsp-repository/commit/89d545081da903a0b7ecc8980cf48dc74a698fc2))
* **dpe-core,shared-fair,dpe-api-oai,dpe-server:** Normalise the ARK host at ingress ([26ed938](https://github.com/dasch-swiss/dsp-repository/commit/26ed93894adfdb2e927cbec591b175e290a666bd))
* **dpe-core:** Name what resolution needs in one place ([4d10b56](https://github.com/dasch-swiss/dsp-repository/commit/4d10b5645fb98e1f24db4320686d3c7519ba4214))
* **dpe-server:** Let a page add markup to the head ([d9026b8](https://github.com/dasch-swiss/dsp-repository/commit/d9026b8e1db22d33c631889955e81fba487c6124))
* **shared-fair,dpe-api-oai:** Move the DataCite and Dublin Core mappings out of OAI ([39e902f](https://github.com/dasch-swiss/dsp-repository/commit/39e902f7f9437f72b04606bbdcf4b6f4bf8a8de3))
* **shared-fair:** Read the project writers off ProjectGraph ([8356e61](https://github.com/dasch-swiss/dsp-repository/commit/8356e618ca74504a21d16d7125f3e0e7a10022df))
* **shared-fair:** Read the record writers off RecordGraph ([ac196ac](https://github.com/dasch-swiss/dsp-repository/commit/ac196ac9fcd1487e867be585b77d38664abed0ed))
* **shared-fair:** Resolve the creator fallback once, on the graph ([63d98f8](https://github.com/dasch-swiss/dsp-repository/commit/63d98f83c17d631923ff5c4f5745efd58461a2a5))
* **shared-metadata,dpe-core:** Move three reading rules to the contract crate ([9fdd306](https://github.com/dasch-swiss/dsp-repository/commit/9fdd306734c5f021ef359577427108fe9b0faf72))


### Documentation

* **docs,ci:** Record how decision records are homed and cited across components ([3072576](https://github.com/dasch-swiss/dsp-repository/commit/30725768ec6bd0fafc5f3641e96aaa1988a408bc))
* **docs,dpe-server:** The representation is bounded, and by how much ([ebec39e](https://github.com/dasch-swiss/dsp-repository/commit/ebec39e5662cba9e4050aee6df826a97aef3652a))
* **docs:** A test that reimplements what it tests is not a test ([acb60b0](https://github.com/dasch-swiss/dsp-repository/commit/acb60b0f039fb3e7e00a77732aeddf6cd1b547dc))
* **docs:** Account for all ten points F-UJI does not award ([0a2b9c9](https://github.com/dasch-swiss/dsp-repository/commit/0a2b9c91fd04af8849c29fada378173e42efe299))
* **docs:** Add the dsp-cli migration plan (move into the workspace, release wiring, dsp-api drift CI) ([1d86574](https://github.com/dasch-swiss/dsp-repository/commit/1d86574c778c8ea5c5540e43e60c73c6814c52f3))
* **docs:** Add the dune agent-context layer and the monorepo shape ADRs ([79dc416](https://github.com/dasch-swiss/dsp-repository/commit/79dc4168188107af9c51c65fb7ce48582840f920))
* **docs:** Add the learnings from the dsp-cli migration, and say where learnings live ([a6b0f9e](https://github.com/dasch-swiss/dsp-repository/commit/a6b0f9efc08e5c60f6c9b97bf19e77692a68b258))
* **docs:** Amend the ADR, and score three projects instead of one ([fdd8837](https://github.com/dasch-swiss/dsp-repository/commit/fdd88377cf55dfed1573840cd6c34e3beedca557))
* **docs:** Answer the principles, not the assessor ([29eff20](https://github.com/dasch-swiss/dsp-repository/commit/29eff20ed633b136a29bd53750369098ad643e4b))
* **docs:** Assess 081C, the control that has records and no files ([331b5ef](https://github.com/dasch-swiss/dsp-repository/commit/331b5ef9feb9b76421a6999c474e198042e1ff48))
* **docs:** Describe shared-fair and the moved reading rules ([59998a6](https://github.com/dasch-swiss/dsp-repository/commit/59998a6ad5e9d18b2ba48b5e2af229dab9b58bf6))
* **docs:** Describe the landing page's machine-readable metadata ([44da66f](https://github.com/dasch-swiss/dsp-repository/commit/44da66f3c2a4f5a773071d57792079d98fd445b4))
* **docs:** Describe the representations, and stop deferring the enforcement ([fabeb0c](https://github.com/dasch-swiss/dsp-repository/commit/fabeb0c38c50cc6450ca97245a1b9f785c581155))
* **docs:** Describe the two shapes an assessor reads as RDF ([caebb4c](https://github.com/dasch-swiss/dsp-repository/commit/caebb4ce8b3fcd7eedf4c8eb26654690d488586d))
* **docs:** Draw out why the byte sweep caught what it caught ([203a6f3](https://github.com/dasch-swiss/dsp-repository/commit/203a6f3288766bee78eb92227563f39398cbd6a6))
* **docs:** Group the areas under areas/ and name the shared root shared/ ([70acb52](https://github.com/dasch-swiss/dsp-repository/commit/70acb5250ee8bbe26e4c3a6cec3ff385731e939c))
* **docs:** Hand off the fixes the first live assessment found ([9cf0160](https://github.com/dasch-swiss/dsp-repository/commit/9cf0160a611113a47283ef679b08f034c4e067bb))
* **docs:** Name the projects in the assessment table ([f461257](https://github.com/dasch-swiss/dsp-repository/commit/f46125784258f502f36a20138dcab76f7dea75d0))
* **docs:** Plan machine-readable project metadata for FAIR assessment ([20bf0bd](https://github.com/dasch-swiss/dsp-repository/commit/20bf0bd605ba56efaad380983a29c3543ffa3e9e))
* **docs:** Point at the metrics behind the FAIR Champion names ([e903d02](https://github.com/dasch-swiss/dsp-repository/commit/e903d020be1a461dd735ad8c01c820c8e72bb4f2))
* **docs:** Record Phase 8, and reclassify F1-02D as a defect ([e7ed51e](https://github.com/dasch-swiss/dsp-repository/commit/e7ed51e5b7f3b40ad550f0d1f2458d04a87799fc))
* **docs:** Record Phase 9, and where the substitution belongs ([8c6577d](https://github.com/dasch-swiss/dsp-repository/commit/8c6577de31a4137380353e7cbb9754efeabbfc59))
* **docs:** Record phases 5 to 7 and the four assessed projects ([223e7b5](https://github.com/dasch-swiss/dsp-repository/commit/223e7b580a0899f98798571bb08904469aeb6ab7))
* **docs:** Record the execution of the dsp-cli migration plan ([11520a1](https://github.com/dasch-swiss/dsp-repository/commit/11520a18866a5621a36db7eb3b8f5a9bac9d8088))
* **docs:** Record the FAIR metadata exposure run ([bc86f8f](https://github.com/dasch-swiss/dsp-repository/commit/bc86f8fe8ad85b82a6447cd7737967917da4d6cc))
* **docs:** Record the re-run, and why the missing download is a boundary ([11278ca](https://github.com/dasch-swiss/dsp-repository/commit/11278caf08a91ed7acf5380e35166a5a8b9e7f15))
* **docs:** Record the rebase onto main ([b0136d0](https://github.com/dasch-swiss/dsp-repository/commit/b0136d0fff3dd2c22e3b4b2d10ec9a9880832513))
* **docs:** Say shared/ and shared-* everywhere the rename touched ([5e6c3ce](https://github.com/dasch-swiss/dsp-repository/commit/5e6c3ce6b63ca9646f1472c9d64c07478913e4ef))
* **docs:** Say which project the data-pointer residual is about ([4ecbb10](https://github.com/dasch-swiss/dsp-repository/commit/4ecbb100cd662876fc0606d3a2db1285eec6282b))
* **docs:** Say which resolver an ARK carries ([ac4bc9a](https://github.com/dasch-swiss/dsp-repository/commit/ac4bc9aa1707c1ca0cd972156ed8340df78c01c1))
* **docs:** Set up the docs/specs convention ([73dbd40](https://github.com/dasch-swiss/dsp-repository/commit/73dbd405026128582f74f5ab90f0c6162608666d))
* **dsp-cli:** List the insecure-server override in the book ([416e849](https://github.com/dasch-swiss/dsp-repository/commit/416e849785eafbe8c00f9bf6723a6b0e43be5fb5))
* **mosaic-tiles,mosaic-playground,dpe-core,dpe-server,dpe-web,dpe-api-oai,platform-metadata,platform-telemetry,editor-core,editor-server,ci:** Trim comments to the convention's core (DEV-7123) ([57ce87d](https://github.com/dasch-swiss/dsp-repository/commit/57ce87db2bf12ce91e83461bc2004fd19b6f4d70))
* **shared-metadata,docs:** One application point, named as such ([4e62aaa](https://github.com/dasch-swiss/dsp-repository/commit/4e62aaaa5601d4c8aa80328e96cea47d108ea963))


### Tests

* **dpe-api-oai:** Check the representations agree over the whole corpus ([6dfe98e](https://github.com/dasch-swiss/dsp-repository/commit/6dfe98e4b8e31442facd77c4b68391e2e9a7d20d))
* **dpe-api-oai:** Guard the writers against the committed corpus again ([5727133](https://github.com/dasch-swiss/dsp-repository/commit/572713389a8bd3d5e5c484fea622b2f63ac64f97))
* **dpe-api-oai:** Pin OAI output with a corpus-wide hash baseline ([ed04416](https://github.com/dasch-swiss/dsp-repository/commit/ed04416c666d20a222d97124afde44bb8e622fc0))
* **dpe-api-oai:** Pin the unset ARK host ([98a561b](https://github.com/dasch-swiss/dsp-repository/commit/98a561b51484e96e205fca5f57e150a534abea71))
* **dpe-api-oai:** Remove the OAI hash baseline, its job done ([5f7ae8b](https://github.com/dasch-swiss/dsp-repository/commit/5f7ae8b3a4cccb14018f9c436e6654eff888bdb0))
* **shared-fair,dpe-api-oai:** Check the DataCite JSON against DataCite's schema ([404c622](https://github.com/dasch-swiss/dsp-repository/commit/404c622405ad173f4bcc580a081f3f06292ad063))
* **shared-fair:** Pin the multilingual rule and the url reading rule ([e5fbb5c](https://github.com/dasch-swiss/dsp-repository/commit/e5fbb5c1d92bb923d63ca26fbe19a5d8800ede70))


### Build System

* **ci:** Provide commitlint-rs in the Nix dev shell ([2c63129](https://github.com/dasch-swiss/dsp-repository/commit/2c63129948603e29533b7cbc2b7076a272f672df))
* **dsp-cli,docs:** Register the crate and harmonize it with the workspace ([a667633](https://github.com/dasch-swiss/dsp-repository/commit/a66763352259b2ea5b03e2fdf7b6a5af5487c0ef))


### Miscellaneous Chores

* **ci:** Add `just fair-check` to score a landing page with F-UJI ([837585b](https://github.com/dasch-swiss/dsp-repository/commit/837585b76f30fa2f54d49f76118df0f16d77049f))
* **ci:** Give fair-check time to assess a project that has files ([d5aeb99](https://github.com/dasch-swiss/dsp-repository/commit/d5aeb99ac32ae86bd68516e889b5dff7e1a91ddc))
* **ci:** Keep the justfile formatter-clean ([e02708f](https://github.com/dasch-swiss/dsp-repository/commit/e02708f9260f7176b4a79326b269dc08448c09d8))
* **ci:** Release dsp-cli on its own version line ([b7c3404](https://github.com/dasch-swiss/dsp-repository/commit/b7c34043bfc04eb1914ea23da79462d956fb58a6))
* **ci:** Run dsp-cli live tests against a pinned dsp-api stack ([bd73243](https://github.com/dasch-swiss/dsp-repository/commit/bd7324387590dcf6c76ca3a8b845d47af0559c77))
* **deps:** Update rust-overlay so the dev shell's nightly rustfmt matches CI ([c15ac71](https://github.com/dasch-swiss/dsp-repository/commit/c15ac7106b6b42403cebe32248fcb22e48b3ffaf))
* **dpe-data:** Point 0105 drawings at the migrated server ([18a8f36](https://github.com/dasch-swiss/dsp-repository/commit/18a8f36fc2320d13256de1ac958844e21cb0dc6e))
* **main:** Release dsp-cli 0.3.0 ([6914a9b](https://github.com/dasch-swiss/dsp-repository/commit/6914a9b8c095ac958ee95eec0cccc8b1428fb022))
* **oai:** Update database with mime types ([67c7f89](https://github.com/dasch-swiss/dsp-repository/commit/67c7f8994cdf5bcc7788bfd37f995b33a703c1e9))
* **shared-fair:** Add the crate that will hold the FAIR exposure engine ([32e2234](https://github.com/dasch-swiss/dsp-repository/commit/32e22345084fc5031795ced2889341978b6beaf5))
* **shared-metadata,shared-telemetry:** Move the shared root to shared/ ([0a65104](https://github.com/dasch-swiss/dsp-repository/commit/0a6510485ad88479da05b14af7ba45f1bed7928c))

## [0.8.5](https://github.com/dasch-swiss/dsp-repository/compare/v0.8.4...v0.8.5) (2026-09-17)


### Features

* **editor-core,editor-web,editor-server,platform-metadata,mosaic-tiles:** Finish the project form and submit validation ([8a08f4e](https://github.com/dasch-swiss/dsp-repository/commit/8a08f4eb11ac535682f70c9ab422bb0f7d58c51a))
* **editor-core,editor-web,editor-server:** Derive project status and detect Online at startup ([8c9bde6](https://github.com/dasch-swiss/dsp-repository/commit/8c9bde61fbf27560520a480df811785e77f21521))
* **editor-core,editor-web,editor-server:** Finish a review round ([668cce4](https://github.com/dasch-swiss/dsp-repository/commit/668cce44332f58fdf087bad2ca35ffa0c3bf23f6))
* **editor-core,editor-web,editor-server:** Propose persons and organisations ([62398f0](https://github.com/dasch-swiss/dsp-repository/commit/62398f0f2047e33d87331309b3a7eab46b5b67c7))


### Bug Fixes

* **dpe-core,dpe-web,dpe-server:** Decide the cover image server-side so the fallback works without JavaScript ([625b462](https://github.com/dasch-swiss/dsp-repository/commit/625b462f6919b56fb01504c100a874ccc0dc70ac))
* **dpe-data:** Add external website link for roud-oeuvres (0112) ([7824428](https://github.com/dasch-swiss/dsp-repository/commit/7824428bf295b94fe46220ed8e02d62009d0a1e5))
* **editor-core,editor-web,editor-server:** Act on the review of [#384](https://github.com/dasch-swiss/dsp-repository/issues/384) ([860df18](https://github.com/dasch-swiss/dsp-repository/commit/860df18ea4a536569378a96a7dbd129a91ef0f3b))


### Code Refactoring

* **mosaic-tiles,editor-web:** Give the alert tile its own bottom margin ([76874dd](https://github.com/dasch-swiss/dsp-repository/commit/76874ddefdff005ff0ef32e5f94061feb0cbd405))


### Documentation

* **docs:** Adopt the shared comment convention (DEV-7123) ([5bd28a9](https://github.com/dasch-swiss/dsp-repository/commit/5bd28a922d0d3bef85b75d9ebc7aa96dba07b594))
* **dpe-server:** Correct the JSON API's environment URLs ([3725820](https://github.com/dasch-swiss/dsp-repository/commit/3725820e322131267adc76ec11dbd4dc41041be9))
* **editor-core,editor-web,editor-server:** Trim comments to the convention's core (DEV-7123) ([65470c4](https://github.com/dasch-swiss/dsp-repository/commit/65470c4342944284df2ebfe726ed924405c3ae6e))


### Tests

* **editor-web:** E2E, accessibility and the no-JavaScript path ([c8ae3bb](https://github.com/dasch-swiss/dsp-repository/commit/c8ae3bbdebfce1b92b618081e70c6c0b842ed94f))


### Miscellaneous Chores

* **ci,platform-metadata:** Fail the build when a platform crate hardcodes a path into another module ([48cbd51](https://github.com/dasch-swiss/dsp-repository/commit/48cbd5176b0fd42a09fdff8a3ab0b0b353102216))
* **ci:** Give every mktemp a TMPDIR template ([02d0427](https://github.com/dasch-swiss/dsp-repository/commit/02d04275b3265dc780e705da1a1125120485feff))
* **ci:** Reject Datastar's pre-RC.6 hyphen delimiter ([52c06ca](https://github.com/dasch-swiss/dsp-repository/commit/52c06ca148cc421eacdfd86b9d053636c83f0cab))
* **docs:** Remove pre-Claude-5 ceremony from CLAUDE.md ([944ead9](https://github.com/dasch-swiss/dsp-repository/commit/944ead9dfc424f6f2a47ca1087f3752a929c20f1))
* **dpe-data:** Point 0103 theatre societe at the migrated server ([39d01c9](https://github.com/dasch-swiss/dsp-repository/commit/39d01c9853dbb246be495c6478a3359d10d8b764))
* **dpe-data:** Point 0116 medframes at the migrated server ([1c26038](https://github.com/dasch-swiss/dsp-repository/commit/1c26038d5ef79dd1a0c01a789ba826bac7a17b85))
* **dpe-data:** Update Proto4DigEd handbook publication to Zenodo software citation ([5fc330c](https://github.com/dasch-swiss/dsp-repository/commit/5fc330cb9614947e99445275bb50c576d917ca11))
* **dpe-data:** Update the demo URL for 0854 (Alice in DaSCHland) ([545f8b7](https://github.com/dasch-swiss/dsp-repository/commit/545f8b7272878b109c15fe8af1fda71b65d824c7))
* **dpe-data:** Update the demo URL for 0854 (Alice in DaSCHland) ([97decfb](https://github.com/dasch-swiss/dsp-repository/commit/97decfba8823e8cb6d02dd0c95abc28eeb4ff20e))

## [0.8.4](https://github.com/dasch-swiss/dsp-repository/compare/v0.8.3...v0.8.4) (2026-09-07)


### Features

* **dpe-server:** Add a record file metadata endpoint ([6449a83](https://github.com/dasch-swiss/dsp-repository/commit/6449a839af217f161f043a2ba94d682de36eb464))
* **editor-core,editor-server:** Read the published project set ([712ecf9](https://github.com/dasch-swiss/dsp-repository/commit/712ecf9fd36f5681b06e1b7c7890354ab01884a2))
* **editor-core,editor-web,editor-server:** Add the review queue and the field-by-field diff ([2461c62](https://github.com/dasch-swiss/dsp-repository/commit/2461c628ca4780675bb4a05c6033d73d9b705a33))
* **editor-core,editor-web,editor-server:** Render and save one project form section ([878bbca](https://github.com/dasch-swiss/dsp-repository/commit/878bbcaa94d1847a55450e4f034c143b1d2689ab))
* **editor-core:** Add the canonical project writer ([a73a4e6](https://github.com/dasch-swiss/dsp-repository/commit/a73a4e6ca975c1d30054ba5e3568df3839fae4a2))
* **editor-core:** Add the permissive draft representation ([c6c1e20](https://github.com/dasch-swiss/dsp-repository/commit/c6c1e20f526e98b899477b41898f970ef438a44b))
* **editor-core:** Read a posted form body back into a draft ([f86dcd7](https://github.com/dasch-swiss/dsp-repository/commit/f86dcd7b0ed3966fac7ebedd8eddff1bd55ea3cb))
* **editor-core:** Refuse a submission whose temporalCoverage cannot resolve ([4f4b7fc](https://github.com/dasch-swiss/dsp-repository/commit/4f4b7fc6bb8e9884947482b47e910ba45878b2e9))
* **editor-server:** Seed sample records on a throwaway deployment ([fc2f4c4](https://github.com/dasch-swiss/dsp-repository/commit/fc2f4c41c7856c1e235e38cc12b6b139bf79272d))
* **editor-web:** Add the project field registry and its sections ([46a36e5](https://github.com/dasch-swiss/dsp-repository/commit/46a36e5af6204bb48298dfd36810ed5f6aed1848))
* **editor-web:** Link the review queue from the RDU projects page ([a7ffd0e](https://github.com/dasch-swiss/dsp-repository/commit/a7ffd0ee3b2f45218958e1da041c7ac83477b604))
* **mosaic-tiles:** Add the alert tile ([bb78935](https://github.com/dasch-swiss/dsp-repository/commit/bb78935af5cec6b988db004a59f406cd2cfecb29))
* **mosaic-tiles:** Add the form tiles the project form needs ([c24223f](https://github.com/dasch-swiss/dsp-repository/commit/c24223f6d5efd86bc8056679e92de9aacd4e5ab1))
* **mosaic-tiles:** Add the table tile ([b59fcfe](https://github.com/dasch-swiss/dsp-repository/commit/b59fcfe8fd7d1c82a771a5e2bc8bbeed8321a185))
* **mosaic-tiles:** Add the text field tile ([96041fd](https://github.com/dasch-swiss/dsp-repository/commit/96041fd16d4a8e17e8cbd92bee50a6849203b8fe))
* **mosaic-tiles:** Let a submit button carry its own name and value ([5f9108d](https://github.com/dasch-swiss/dsp-repository/commit/5f9108d6124b64b48ab0c61444de3dc1586ea1fe))


### Bug Fixes

* **dpe-data:** Point 0111 project URL at app.dasch.swiss after server move ([02d8c40](https://github.com/dasch-swiss/dsp-repository/commit/02d8c40fe2b385051abf5729c7c2aaba1dbd0f99))
* **dpe-data:** Restore 0103's canonical key order ([26c8622](https://github.com/dasch-swiss/dsp-repository/commit/26c86222f749d2aa301f56fd3e1c154d39a6c766))
* **dpe-data:** Set the licenses decided for 0119, 082C, 0110 and 082A (DEV-7156) ([d5102dc](https://github.com/dasch-swiss/dsp-repository/commit/d5102dcae888bab267f48a8c9f5ffad0b0878dac))
* **dpe-server:** Answer the record file 404 in JSON, not the HTML shell ([94bcbdf](https://github.com/dasch-swiss/dsp-repository/commit/94bcbdf08f777554927ad07d178bd35f6eeaa153))
* **platform-metadata,dpe-core:** Make mimeType optional and log dump load failures ([eece231](https://github.com/dasch-swiss/dsp-repository/commit/eece2316d1220c14e2103d0b3551389f06f61024))


### Code Refactoring

* **dpe-server:** Rewrite validate over the extracted checker ([b17169e](https://github.com/dasch-swiss/dsp-repository/commit/b17169e46ab1bb35752c725abe395124adf92a1e))
* **editor-server:** Fold the review columns into the initial schema ([4a39691](https://github.com/dasch-swiss/dsp-repository/commit/4a39691a514d8b251575ed8364a85696c841f0f0))
* **editor-web:** Render the login and depositor screens with the new tiles ([0daa25f](https://github.com/dasch-swiss/dsp-repository/commit/0daa25ff33271fc0cd2aab12e24103933a6721db))
* **mosaic-tiles:** Group the form tiles into a form directory ([6c426ef](https://github.com/dasch-swiss/dsp-repository/commit/6c426ef93ff011fa385813efbda91b8cba023c07))
* **platform-metadata:** Make multilingual maps deterministic ([05bdd48](https://github.com/dasch-swiss/dsp-repository/commit/05bdd485d5f94de2fb1e160367bca5d0f4deff6b))


### Documentation

* **editor-core:** Record the project representation and canonical form ([a8e35e2](https://github.com/dasch-swiss/dsp-repository/commit/a8e35e2f4fb2ace5c4ab97cc6d4ed61572f31b9f))
* **editor-server:** State the review surface's rules without citing the PRD ([9d6bf8a](https://github.com/dasch-swiss/dsp-repository/commit/9d6bf8a8a4a71914db10309ad0ab5fd27c88ee8c))
* **mosaic-tiles:** Bring the tile conventions and the add-component skill up to date ([3ca5b1e](https://github.com/dasch-swiss/dsp-repository/commit/3ca5b1ed1eb6cffbe2eb93156a15b851155abc06))


### Tests

* **dpe-server:** Pin validate's error wording before extracting its rules ([b882c2a](https://github.com/dasch-swiss/dsp-repository/commit/b882c2ab8a638b45a21acb5d21805ba422a31b03))
* **editor-core:** Round-trip the canonical writer over all 85 project files ([8e4ca50](https://github.com/dasch-swiss/dsp-repository/commit/8e4ca503a22343c63cd1df8cdf47fb56639e9a4e))
* **editor-web:** Follow a registry id's dotted path into the contract ([315dc9f](https://github.com/dasch-swiss/dsp-repository/commit/315dc9ffd5853f4ad70658c0d5256a139489d4ac))


### Build System

* **deps:** Bump DPE's vendored Datastar client to 1.0.2 (DEV-6907) ([bb30cf4](https://github.com/dasch-swiss/dsp-repository/commit/bb30cf4e57fb57ffb98b3e590e6f3294cc05f2cc))


### Miscellaneous Chores

* **ci:** Reject merge commits on a branch, and say so plainly ([c14ab74](https://github.com/dasch-swiss/dsp-repository/commit/c14ab74057743d06f231d8b7c2d8c1df78345f25))
* **ci:** Verify vendored JS and Tailwind CLI checksums (DEV-7126, DEV-6727) ([7869953](https://github.com/dasch-swiss/dsp-repository/commit/78699535e095e5fa0e4c59529d19072901d95c9e))
* **dpe-data:** Normalise top-level key order in 31 project files ([990206f](https://github.com/dasch-swiss/dsp-repository/commit/990206f60194167154b0d84eee04e59ddf6231d7))
* **dpe-data:** Point 0115 activites-cs at the migrated server ([061f63a](https://github.com/dasch-swiss/dsp-repository/commit/061f63a9e1145f9793eaf5f1bbd439d00f856d9d))
* **dpe-data:** Replace 0103 cover image with high-res version and add image credit ([a6a5ea7](https://github.com/dasch-swiss/dsp-repository/commit/a6a5ea781adceafe19528c5928bee071d769644e))
* **dpe-data:** Update records with the sidecar fields (DEV-7025, DEV-6963) ([60e298c](https://github.com/dasch-swiss/dsp-repository/commit/60e298c4f2956a11456fe2029372d84359d59bdb))
* **dpe-server:** Use pretty, colour-coded logs on a terminal ([14de46c](https://github.com/dasch-swiss/dsp-repository/commit/14de46c071ef18d31386f02550f18319ca74db5a))

## [0.8.3](https://github.com/dasch-swiss/dsp-repository/compare/v0.8.2...v0.8.3) (2026-08-31)


### Features

* **dpe-data:** Add project URL for 0863 (Samaria Ivories) ([afe7ab9](https://github.com/dasch-swiss/dsp-repository/commit/afe7ab98301680f37410f7c9d2dd0bc5f7585932))
* **dpe-server:** Add temporal-coverage resolution check to validate ([4efbbc2](https://github.com/dasch-swiss/dsp-repository/commit/4efbbc26604ac05e74489c1012eb7f223bfcc6f9))
* **editor-server:** Add email one-time-code login and sessions ([b1b2e22](https://github.com/dasch-swiss/dsp-repository/commit/b1b2e224ce1703f62bd3049d7e71b2bd1416fa88))
* **editor-server:** Add RDU depositor account management ([5fe85e5](https://github.com/dasch-swiss/dsp-repository/commit/5fe85e5cf3b02f2d7f3bd836cf4e46d49a74d446))
* **editor-server:** Add the SQLite persistence layer ([caae102](https://github.com/dasch-swiss/dsp-repository/commit/caae102afe309a90914d67ca3ebe42410c776982))
* **editor-server:** Extend the auth persistence for lockout and browser binding ([ff96b32](https://github.com/dasch-swiss/dsp-repository/commit/ff96b3255666394357b4bc8e29e12caabd8b75f5))
* **editor-server:** Require Sec-Fetch-Site same-origin on state-changing requests ([800e86b](https://github.com/dasch-swiss/dsp-repository/commit/800e86b668378599f81d3e8396710387d7a540de))
* **editor-server:** Scaffold the metadata editor service ([c0b7acd](https://github.com/dasch-swiss/dsp-repository/commit/c0b7acd81d3b76e24d1b6eec11077f92f870281c))
* **editor-server:** Scope project access to a depositor's assigned shortcodes ([49c2b88](https://github.com/dasch-swiss/dsp-repository/commit/49c2b88bcf5cb031ee95728196a3bb817e45345f))
* **editor-web:** Add the login and code-entry screens ([0e358c8](https://github.com/dasch-swiss/dsp-repository/commit/0e358c8eb3a34ba8672c1dd7e54746656539ac8c))


### Bug Fixes

* **ci:** Provision the full non-Nix toolchain in the install recipes ([5520b7b](https://github.com/dasch-swiss/dsp-repository/commit/5520b7be1233ef466ab3b73f342b89397173076e))
* **dpe-data:** Correct solec (0868) data link and contact email ([e4a1575](https://github.com/dasch-swiss/dsp-repository/commit/e4a1575b42f851a35eb407e1b5cd117be0edae74))
* **dpe-data:** Point 0101 project URL at app.dasch.swiss after server move ([7bdfcce](https://github.com/dasch-swiss/dsp-repository/commit/7bdfcce63ebdae112873a2f491c5a25c7a0ee5c2))
* **dpe-data:** Point 0114 project URL at app.dasch.swiss after server move ([c696082](https://github.com/dasch-swiss/dsp-repository/commit/c696082b1d82cef995dfe8cce527feace9857533))
* **dpe-data:** Stop advertising external websites as DSP data ([8fbfac3](https://github.com/dasch-swiss/dsp-repository/commit/8fbfac3f9c179d80912b2741416b187361e353ad))
* **dpe-server:** Require delimiter after host in healthcheck loopback guard ([8ea36f1](https://github.com/dasch-swiss/dsp-repository/commit/8ea36f16e5e82d256c069fb6c6a2a9b84979d24d))
* **dpe-web:** Open external links in a new tab ([909a122](https://github.com/dasch-swiss/dsp-repository/commit/909a1221d2136ed7c98a3fac09c930c04ee53d48))
* **editor-server:** Make the login-code send cap countable and per-account ([7571e94](https://github.com/dasch-swiss/dsp-repository/commit/7571e94c4252ff2823931a632b83eb29ccc710c3))
* **editor-server:** Show the login code where no mail can be sent ([322550f](https://github.com/dasch-swiss/dsp-repository/commit/322550fa58f619a15de834461c78219929e71f2c))
* **platform-telemetry:** Normalize page_url per-service instead of one shared route table ([6622573](https://github.com/dasch-swiss/dsp-repository/commit/6622573579736c5ca88fbc7b96c8e74ae6df422e))


### Code Refactoring

* **dpe-telemetry:** Move the browser beacon collector into the crate ([73d667e](https://github.com/dasch-swiss/dsp-repository/commit/73d667e8b25c76567fb25814143f1020d4085d88))
* **editor-server:** Hold the repository ports in AppState ([62ed13f](https://github.com/dasch-swiss/dsp-repository/commit/62ed13f2967652f44f772795b426253682a0a018))
* **editor-server:** Make the data directory an explicit, unset-by-default seam ([104b1b8](https://github.com/dasch-swiss/dsp-repository/commit/104b1b89b96de3b33caddd8b459b238208c241fd))
* **mosaic-tiles:** Extract a component-CSS barrel ([f35de37](https://github.com/dasch-swiss/dsp-repository/commit/f35de376bafbccd0e3a09e446687de7831ec0774))
* **platform-metadata,dpe-core:** Extract the shared metadata contract ([c78459a](https://github.com/dasch-swiss/dsp-repository/commit/c78459a9d27427241523cc5b770f996617be7f75))
* **platform-telemetry:** Move the shared telemetry crate under modules/platform ([5b09cbd](https://github.com/dasch-swiss/dsp-repository/commit/5b09cbdd68cc82d2f72b9c9cfcfe7a1ab2a70dc6))


### Documentation

* **docs:** Trim derivable content from the CLAUDE.md guidance files ([74f53a3](https://github.com/dasch-swiss/dsp-repository/commit/74f53a3949bfcbe277440bfdb613d12b8877fe4c))
* **dpe-core:** Name the editor's copy of is_valid_shortcode ([857412f](https://github.com/dasch-swiss/dsp-repository/commit/857412f448c1b16a5822ff424171c08130bca301))
* **dpe-server:** Correct the distroless uid in the operations guide ([3eae258](https://github.com/dasch-swiss/dsp-repository/commit/3eae2583e5a4a12094c87d723249dae15126d452))
* **editor-server:** Record the email-authentication deviation ([518fe21](https://github.com/dasch-swiss/dsp-repository/commit/518fe212d82b791e6799d1e6a1a64d0224fe5d66))


### Miscellaneous Chores

* **ci:** Add just recipe for updating record dumps (DEV-6838) ([b28d161](https://github.com/dasch-swiss/dsp-repository/commit/b28d1617547651611fbccc9b0b3724e46268a3f7))
* **ci:** Bound job runtime and harden network fetches ([aaf3d24](https://github.com/dasch-swiss/dsp-repository/commit/aaf3d240c74f24377e2ec39bf6de43900909b26b))
* **ci:** Update the path-keyed automation for the new module layout ([ad83a72](https://github.com/dasch-swiss/dsp-repository/commit/ad83a725cc755f8ae06c00f327f7912a27303f13))
* **dpe-api-oai,dpe-server:** Wrap over-long comments to satisfy rustfmt ([6d17ca8](https://github.com/dasch-swiss/dsp-repository/commit/6d17ca81c8bde4b2df41f99236209bf1d81fa1df))
* **dpe-data:** Normalise language-key order in project files ([acfb409](https://github.com/dasch-swiss/dsp-repository/commit/acfb409b27b3c15a86d52bd4a75b8a868ea2c870))
* **dpe-data:** Point 0107 stardom at the migrated server ([43e5c5a](https://github.com/dasch-swiss/dsp-repository/commit/43e5c5a447a79522e7e637b3967eb16487a32fd2))
* **dpe-data:** Point 0110 h-steiner at the migrated server ([3cfe78d](https://github.com/dasch-swiss/dsp-repository/commit/3cfe78d7aa00b846466303e979ccc85fc99f4a08))
* **dpe-data:** Update record dumps (DEV-6838) ([b3f9be7](https://github.com/dasch-swiss/dsp-repository/commit/b3f9be7ea9fcb9427fa54b77a43c87b6f0e4d5a0))
* **dpe-data:** Update the demo URL for 0854 (Alice in DaSCHland) ([f95ab4c](https://github.com/dasch-swiss/dsp-repository/commit/f95ab4cd4b5ae1671c8772ec6c4d21208455bd0e))
* **dpe-data:** Update the demo URL for 0854 (Alice in DaSCHland) ([aea9023](https://github.com/dasch-swiss/dsp-repository/commit/aea9023962b61a5aaa50187ecca58b0a1a688775))
* **mosaic:** Rename add-mosaic-component skill file to SKILL.md ([ab27d14](https://github.com/dasch-swiss/dsp-repository/commit/ab27d1483105ec0712041b0dd3eacea9edf47c6e))

## [0.8.2](https://github.com/dasch-swiss/dsp-repository/compare/v0.8.1...v0.8.2) (2026-07-29)


### Features

* **dpe-data:** Add imageCredit for 0113 (Women Martyrs in Action) ([3931d4f](https://github.com/dasch-swiss/dsp-repository/commit/3931d4f72d0900c0f6ed9c1d1827ad2f73d9d45d))
* **dpe-data:** Backfill imageCredit for 16 projects from dsp-app (DEV-6870) ([7c228ed](https://github.com/dasch-swiss/dsp-repository/commit/7c228edf0ae670910cae50c34573074b5e61fb51))
* **dpe-web:** Add optional imageCredit field + cover-image caption (DEV-6860) ([f169e2f](https://github.com/dasch-swiss/dsp-repository/commit/f169e2f9bc5e5a194b20a0a415ec99a2ceba855d))
* **dpe-web:** Show project image credit on grid teaser cards ([8b89d99](https://github.com/dasch-swiss/dsp-repository/commit/8b89d990dabbc5abcfb5b9fb54c87f5e35f92b1f))

## [0.8.1](https://github.com/dasch-swiss/dsp-repository/compare/v0.8.0...v0.8.1) (2026-07-28)


### Features

* Add VARMA (8055) project cover image to the DPE ([1adf7e2](https://github.com/dasch-swiss/dsp-repository/commit/1adf7e25403c7e7d2687ccbff5888e40b41a7bf3))
* **dpe-api-oai:** Add resumption-token paging to ListRecords and ListIdentifiers (DEV-6684) ([625142d](https://github.com/dasch-swiss/dsp-repository/commit/625142dbd93c1f3aa979054382013d79b81b1730))
* **dpe-api-oai:** Expose record file MIME type and download link in OAI output (DEV-6684) ([b281376](https://github.com/dasch-swiss/dsp-repository/commit/b2813760ce37c4333b9d1d94ab81b1178d4333e9))
* **dpe-data:** Onboard project 0113 (WoMartyrAct) ([c870215](https://github.com/dasch-swiss/dsp-repository/commit/c87021534560b46c303326311ac995e2aeff9218))
* **dpe,mosaic:** Browser live-reload in the dev loops (DEV-6728) ([8f1283a](https://github.com/dasch-swiss/dsp-repository/commit/8f1283a91adda697aaeaf262fb0ec4ea461e04ce))
* Move project abstract from Publications to Overview ([0448a6a](https://github.com/dasch-swiss/dsp-repository/commit/0448a6a25f2f9ca0459424534b3b875ed3a69ab2))
* OAI-PMH endpoint rate-limiting (DEV-6724) ([9c38251](https://github.com/dasch-swiss/dsp-repository/commit/9c38251b345c0e002ce3d1963585a6dc7d4bdb16))
* Onboard project 8055 (VARMA) and link contributor profiles ([dfb0478](https://github.com/dasch-swiss/dsp-repository/commit/dfb04780e24d01f7fdd08167b95cc9ac92e9a17f))
* Show project permalink as the bare ARK identifier ([95233d3](https://github.com/dasch-swiss/dsp-repository/commit/95233d3e898bde09f03651ffcc3d65df090bb249))


### Bug Fixes

* Restore default blue link colour ([ccca2c2](https://github.com/dasch-swiss/dsp-repository/commit/ccca2c2f4452f32f0bc583882d2fd1fefe660020))


### Code Refactoring

* **dpe,mosaic:** Remove Leptos — migrate to Maud + Axum + Datastar (DEV-6642) ([b89eaf4](https://github.com/dasch-swiss/dsp-repository/commit/b89eaf4fc1f031d463b6385ef0859c1a7f41ead8))


### Documentation

* Clarify that PRs should default to a single commit ([73c3c17](https://github.com/dasch-swiss/dsp-repository/commit/73c3c172e602177cc1006836543bc7cc1a4563f1))


### Build System

* Add cargo-machete unused-dependency check to just check ([29bfa59](https://github.com/dasch-swiss/dsp-repository/commit/29bfa5913d67c65217cf63062372629ec477a6fb))


### Miscellaneous Chores

* **ci:** Enforce commit type, scope, and one commit per PR ([461094c](https://github.com/dasch-swiss/dsp-repository/commit/461094c502f37a0342c8c7aa0e5fc15cd799996b))
* Docker Scout & Dependabot housekeeping; distroless Mosaic runtime ([314d0c9](https://github.com/dasch-swiss/dsp-repository/commit/314d0c9387c27b1f8fd2ab0f4ab7b8df995a1631))

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
