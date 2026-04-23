# frontend Dockerfile

Bun + SvelteKit build for `igait-frontend/`. Produces the static preview
server that runs on port 4173.

## Why this is separate from `.docker/workspace/`

Nothing to share. The workspace Dockerfile's whole reason for existing is
the shared `cargo-chef` cache across a six-crate Cargo workspace — the
frontend has no Cargo.toml, no `target/`, and nothing to cache against
that builder. Merging the two would just add a parallel `FROM oven/bun`
stage that shares zero layers with the chef path.

The split is also useful for CI: frontend and workspace builds are
triggered by disjoint `paths:` filters (`igait-frontend/**` vs.
`igait-{backend,lib,stages}/**`), so pushes that only touch one side
don't rebuild the other.

## Build context

Context is `./igait-frontend` (scoped to the frontend source tree), and
`.dockerignore` stays at `igait-frontend/.dockerignore` because
`.dockerignore` is positional to the build context. The Dockerfile is
addressed from outside the context via the Docker `-f` flag.

```sh
# from repo root
docker build -f .docker/frontend/Dockerfile -t igait-frontend ./igait-frontend
```

## Build-time vars

All `VITE_*` values are `ARG`s because Vite inlines them into the bundle
at build time. The `docker-compose.yml` service definition supplies the
local-emulator-flavoured values; `build-web.yml` supplies prod secrets.
