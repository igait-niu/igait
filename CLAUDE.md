# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

iGait is a web-based autism screening tool that analyzes gait (walking) patterns through a multi-stage video processing pipeline. It uses a microservice architecture with a shared Rust library, SvelteKit frontend, and 7 independent processing stages.

## Architecture

**Queue-driven pipeline**: Backend receives video upload → enqueues to Firebase RTDB → each stage polls its queue independently → processes via S3 I/O → enqueues next stage. Stages are stateless: fetch from S3, process, upload results, queue next.

- **igait-backend/** — Rust/Axum API server (port 3000). Handles auth, uploads, job management, OpenAI assistant integration.
- **igait-frontend/** — Bun/SvelteKit/TypeScript/Tailwind. SvelteKit route-based layout with `(authenticated)/` group for protected routes.
- **igait-lib/** — Shared Rust library used by backend and all stages. Key modules: `microservice/registry.rs` (the `STAGES` const — single source of truth for stage identity, ordering, UI metadata, and declared inter-stage I/O), `microservice/worker.rs` (StageWorker trait), `microservice/queue.rs` (Firebase RTDB ops), `microservice/storage.rs` (S3 client). Has feature flags: `microservice`, `email`.
- **igait-stages/** — Rust microservice stages, each following the StageWorker pattern from igait-lib. Each stage lives in its own directory named for what it does:
  - `media-conversion/` — FFmpeg (entry point)
  - `validity-check/` — Python submodule
  - `reframing/` — FFmpeg
  - `pose-estimation/` — MediaPipe/Python submodule
  - `cycle-detection/` — Python submodule
  - `prediction/` — TensorFlow/Python submodule
  - `finalize/` — AWS SESv2 (terminal stage, sends email)

**Cloud services**: Firebase (RTDB queues, Auth, Firestore), AWS S3 (file storage), AWS SESv2 (email), OpenAI API (assistant).

## Build & Run Commands

### Prerequisites
- Nix with flakes enabled + direnv (auto-loads dev environment with Rust, Bun, Python, FFmpeg)
- Docker for full stack
- Credentials: `.envrc` and `credentials/gcp-key.json` from team OneDrive

### Development (individual services)
```bash
# Backend or any stage (from its directory)
export PORT=<port> && cargo run --release

# Frontend
cd igait-frontend && bun install && bun run dev

# Frontend lint/format
bun run lint        # prettier + eslint check
bun run format      # auto-format
```

### Integration Test
```bash
./test.sh  # Uploads a test video to a running backend on :3000
```

> The top-level `docker-compose.yml` has been removed pending a rewrite.
> Individual stages still have Dockerfiles under `igait-stages/{key}/`
> that build via `docker build -f igait-stages/{key}/Dockerfile .`
> from the repo root; K8s deployments pull each stage's image from
> `ghcr.io/igait-niu/igait/{key}:sha-<short>`.

## CI/CD

Path-filtered GitHub Actions: each service has its own workflow triggered by changes to its directory. All Rust services also rebuild when `igait-lib/` changes. Images push to `ghcr.io/igait-niu/`. Kubernetes configs are in a git submodule (`igait-kubernetes-configs/`).

## Key Patterns

- **`STAGES` is the source of truth.** The ordered array in `igait-lib/src/microservice/registry.rs` drives stage identity, `next_stage()`, terminal-queue routing in `worker.rs`, rerun input-keys resolution in `backend/routes/rerun.rs`, and the frontend's tab list. No hardcoded stage names should appear outside the registry.
- The backend **publishes `STAGES` to RTDB at `/registry/stages`** on startup (`backend/helper/registry_publish.rs`). The frontend reads it via `igait-frontend/src/lib/stores/registry.svelte.ts` (`registryStore`) — both stage-rendering pages (`/job/[id]` and `/admin/queues`) iterate that store.
- **Custom UI panels** are enum-dispatched via `StagePanel` (`Default` | `VideoEdit` | `GaitCycles`). Adding a new panel variant is a deliberate cross-language change — a Rust enum variant plus a Svelte component keyed by the variant.
- All stages implement the `StageWorker` trait from igait-lib — check `igait-lib/src/microservice/worker.rs` for the interface.
- Several stages use **git submodules** for Python ML code — clone with `--recurse-submodules`.
- Firebase RTDB queue entries follow a strict schema (see `database.rules.json`): `queues/stage_1..stage_6` require `job_id, user_id, enqueued_at, input_keys, metadata`; `queues/finalize` requires `job_id, user_id, enqueued_at, success, metadata`. The `/registry` path allows authenticated read and admin write. Legacy `stage_N` segment names are retained for backward compatibility with in-flight jobs; the registry bridges them via `queuePathSegment` / `stageLogsKey` helpers.
- Frontend uses Firebase Auth; API client in `igait-frontend/src/lib/api/client.ts` attaches auth tokens.

## Adding a new stage

The refactor in `refactor/stage-unordering` is designed so that inserting a stage between two existing ones is a **single-file edit** plus a new stage crate.

1. **Register the stage.** Add a `StageSpec { key, display_name, description, terminal, panel, inputs }` entry at the desired position in the `STAGES` const in `igait-lib/src/microservice/registry.rs`. Declare the stage's input artifacts via `StageInput { name, ext, from }` entries — `from` is either another stage's key or the sentinel `"upload"` for raw uploads.
2. **Update the successor's inputs.** If the new stage's outputs should feed the following stage, change that stage's `inputs[*].from` to point at your new key.
3. **Create the stage crate.** Copy any existing stage's directory under `igait-stages/` (e.g. `cp -r igait-stages/reframing igait-stages/<your-key>`), then edit its `Cargo.toml`, `Dockerfile`, and `src/main.rs` to match — the `StageWorker::stage()` method must return the matching `StageNumber` variant and `service_name()` the key.
4. **Add orchestration glue.** A new service block in `docker-compose.yml` and a workflow file at `.github/workflows/build-<key>.yml` (copy an existing one and globally replace the key). For K8s deployments, add a matching `STAGE{N}_IMAGE` env var in `igait-kubernetes-configs/igait-backend/deployment.yaml`.

After a backend redeploy, the frontend automatically picks up the new stage from the RTDB registry — no frontend code change is required unless you're introducing a new `StagePanel` variant.
