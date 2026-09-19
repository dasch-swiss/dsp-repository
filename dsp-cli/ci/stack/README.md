# dsp-cli CI stack

A docker-compose stack pinning dsp-api and its Fuseki triplestore, so
dsp-cli's live tests can run against a real dsp-api without credentials for
a shared, long-lived server. It exists to catch drift between dsp-api's
actual API and dsp-cli's mocks, which a mock-only test suite cannot see.

## What's in it

- `db`: `daschswiss/apache-jena-fuseki`, holding the `dsp-repo` repository.
- `api`: `daschswiss/knora-api`, pointed at `db`.

No sipi, no ingest, no alloy, no app — dsp-cli does not need them, and
dsp-api's healthcheck does not require sipi to report healthy.

## The version pin

`stack.env` holds two independent pins, both currently `v38.1.0`. This is not
a recommendation from the dsp-api team and is not kept in lockstep with
dsp-api's own compose file, which uses `latest` for both images.

- `API` pins the `knora-api` image tag, and also which tag `load-fixtures.sh`
  sparse-checks-out from `dasch-swiss/dsp-api` for the fixtures and
  ontologies it loads. Bumping `API` moves both.
- `DB` pins the Apache Jena Fuseki image's own version line (upstream
  currently `v39.0.0-N-gSHA`), unrelated to any dsp-api ref. Bumping `DB`
  moves only the Fuseki image.

To bump `API`: change it in `stack.env`, run the stack locally (below) to
confirm the fixtures still load and dsp-cli still works against the new
dsp-api version, then commit. To bump `DB`: change it in `stack.env`, run
the stack locally to confirm Fuseki still starts and fixtures still load
into it, then commit. The two can be bumped independently or together.

## Running it by hand

```sh
# from the repo root

# 1. Bring up the triplestore and wait for its healthcheck
docker compose --env-file dsp-cli/ci/stack/stack.env \
  -f dsp-cli/ci/stack/docker-compose.yml up -d --wait db

# 2. Load the dsp-api test fixtures (ontologies + project data) into it
bash dsp-cli/ci/stack/load-fixtures.sh

# 3. Bring up dsp-api and wait for its healthcheck
docker compose --env-file dsp-cli/ci/stack/stack.env \
  -f dsp-cli/ci/stack/docker-compose.yml up -d --wait api

# 4. Explicit, debuggable wait on top of the healthcheck
bash dsp-cli/ci/stack/wait-for-api.sh

# 5. Log in and export tokens for the live tests
eval "$(bash dsp-cli/ci/stack/token.sh)"

# 6. Tear down
docker compose --env-file dsp-cli/ci/stack/stack.env \
  -f dsp-cli/ci/stack/docker-compose.yml down -v
```

`token.sh` logs in as the fixture system admin (`root@example.com`) for
`DSP_TOKEN`, and as `anything.user01@example.org`, a fixture member of
project `0001` with no admin rights, for `DSP_TEST_NON_ADMIN_TOKEN`. Both
passwords are `test`, per the dsp-api test fixtures. When `GITHUB_ENV` is
set, the same two variables are also appended there.
