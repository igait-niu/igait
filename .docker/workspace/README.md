# workspace Dockerfile

Unified multi-stage build for the **Rust side** of iGait: the backend
(`igait-backend`) plus all five pipeline stages (`media-conversion`,
`pose-estimation`, `cycle-detection`, `prediction`, `finalize`).

## Why this is separate from `.docker/frontend/`

The backend and stages live in a single Cargo workspace, so `cargo-chef` can
plan every crate's dependency graph *once*, cook it into a cached layer
*once*, and then `cargo build --release --workspace` produces every binary in
a single pass with a shared `target/` directory. All six service images are
`--target` stages below that shared builder, so incremental builds after a
one-line change in `igait-lib` re-cook zero dependencies.

The frontend is a Bun/SvelteKit project with no Cargo.toml, no
`target/`, and no shared cache with any of the above — folding it into this
Dockerfile would add a parallel build stage that shares nothing with the
chef cache. Two distinct toolchains, two distinct Dockerfiles.

## Build context

Context is the **repo root** (`.`) because the builder needs `Cargo.toml`,
`Cargo.lock`, `igait-lib/`, `igait-backend/`, and `igait-stages/`. The root
`.dockerignore` governs what gets uploaded — `.dockerignore` is positional to
the build context so it stays at the repo root, not here.

```sh
# from repo root
docker build -f .docker/workspace/Dockerfile --target backend -t igait-backend .
```

## Runtime base & the stub GCP key

Every service image descends from the `runtime-base` stage, which bakes a
dummy `/credentials/gcp-key.json` into the image. The Firebase SDK insists
the file at `GOOGLE_APPLICATION_CREDENTIALS` exists on disk even when it's
talking to the emulator (which ignores auth). The stub satisfies that check
locally and in CI without anyone having to materialise a real key.

In prod, the K8s Deployment mounts the real service-account key as a
Secret volume at `/credentials`, which replaces the baked stub. Production
behaviour is unchanged.
