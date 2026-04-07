# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

iGait is a web-based autism screening tool that analyzes gait (walking) patterns through a multi-stage video processing pipeline. It uses a microservice architecture with a shared Rust library, SvelteKit frontend, and 7 independent processing stages.

## Architecture

**Queue-driven pipeline**: Backend receives video upload → enqueues to Firebase RTDB → each stage polls its queue independently → processes via S3 I/O → enqueues next stage. Stages are stateless: fetch from S3, process, upload results, queue next.

- **igait-backend/** — Rust/Axum API server (port 3000). Handles auth, uploads, job management, OpenAI assistant integration.
- **igait-frontend/** — Bun/SvelteKit/TypeScript/Tailwind. SvelteKit route-based layout with `(authenticated)/` group for protected routes.
- **igait-lib/** — Shared Rust library used by backend and all stages. Key modules: `microservice/worker.rs` (StageWorker trait), `microservice/queue.rs` (Firebase RTDB ops), `microservice/storage.rs` (S3 client). Has feature flags: `microservice`, `email`.
- **igait-stages/** — 7 Rust microservices, each following the StageWorker pattern from igait-lib:
  - Stage 1: Media conversion (FFmpeg)
  - Stage 2: Validity check (Python submodule)
  - Stage 3: Reframing (FFmpeg)
  - Stage 4: Pose estimation (MediaPipe/Python submodule)
  - Stage 5: Cycle detection (Python submodule)
  - Stage 6: ML prediction (TensorFlow/Python submodule)
  - Stage 7: Finalize + email (AWS SESv2)

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

### Full Stack (Docker)
```bash
# First-time: build services individually (parallel build may exhaust RAM)
docker compose build web
docker compose build backend
docker compose build stage1  # ...through stage7

# Run everything
docker compose up --build

# Frontend at https://localhost:4173
```

### Integration Test
```bash
./test.sh  # Uploads test video, monitors stage 1 processing
```

## CI/CD

Path-filtered GitHub Actions: each service has its own workflow triggered by changes to its directory. All Rust services also rebuild when `igait-lib/` changes. Images push to `ghcr.io/igait-niu/`. Kubernetes configs are in a git submodule (`igait-kubernetes-configs/`).

## Key Patterns

- All stages implement the `StageWorker` trait from igait-lib — check `igait-lib/src/microservice/worker.rs` for the interface.
- Several stages use **git submodules** for Python ML code — clone with `--recurse-submodules`.
- Firebase RTDB queue entries follow a strict schema (see `database.rules.json`): stages 1-6 require `job_id, user_id, enqueued_at, input_keys, metadata`; finalize requires `job_id, user_id, enqueued_at, success, metadata`.
- Frontend uses Firebase Auth; API client in `igait-frontend/src/lib/api/client.ts` attaches auth tokens.
