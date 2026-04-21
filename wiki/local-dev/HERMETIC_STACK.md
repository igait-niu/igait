# Hermetic local stack

`docker compose up` brings up the full iGait pipeline against local
cloud-service surrogates — no AWS account, no Firebase project, no verified
SES identity required. The stack is self-contained: on airplane wifi, it
still works.

## Topology

```
┌─────────────┐     ┌─────────────┐
│  frontend   │────▶│   backend   │─── upload → MinIO
│   :4173     │     │    :3000    │─── enqueue → Firebase emu
└─────────────┘     └─────────────┘
                           │
                           │ (no K8s orchestrator — stages self-poll)
                           ▼
    ┌──── Firebase RTDB (emu) :9000 / UI :4000 ────┐
    │  queues/media-conversion                     │
    │  queues/pose-estimation                      │
    │  queues/cycle-detection                      │
    │  queues/prediction                           │
    │  queues/finalize                             │
    └────┬────┬────┬────┬────┬─────────────────────┘
         │    │    │    │    │    (each stage is a long-running
         ▼    ▼    ▼    ▼    ▼     container polling its queue)
      5 stage worker services
         │
         │ (finalize stage) send email
         ▼
   ┌──────────────┐         ┌──────────────┐
   │   ses-mock   │         │    minio     │
   │    :8005     │         │  :9002 (S3)  │
   └──────────────┘         │  :9001 (UI)  │
                            └──────────────┘
```

## Surrogate map

| Prod dependency | Local surrogate | Container | Notes |
|---|---|---|---|
| AWS S3 | MinIO | `minio` + `minio-bootstrap` | Bucket `igait-storage` is auto-created on boot |
| AWS SES v2 | aws-ses-v2-local | `ses-mock` | UI at `:8005` shows captured emails |
| Firebase RTDB | Firebase emulators | `firebase-emulator` | Project `igait-local`; rules sourced from repo-root `database.rules.json` |
| Firebase Auth | (none needed) | — | Emulator ignores token verification |
| K8s Jobs orchestrator | (disabled) | — | Stages run in worker mode instead |
| OpenAI API | (none — uses prod) | — | Pass-through from host via `OPENAI_*` |

## Mode flags

The compose stack is defined by *what it does not set*:

- `ENABLE_ORCHESTRATOR` unset → backend skips K8s Jobs orchestrator
- `IGAIT_JOB_PAYLOAD` unset per stage → stages run as long-running workers

See `wiki/architecture/STAGE_EXECUTION_MODES.md` for the full dual-mode story.

## AWS SDK endpoint overrides

`aws-sdk-rust` (`BehaviorVersion::latest()`) reads the standard
`AWS_ENDPOINT_URL_S3` and `AWS_ENDPOINT_URL_SESV2` env vars automatically.
No code changes are needed in `igait-lib`; the compose stack sets these to
point at MinIO and `ses-mock` respectively.

If that behaviour ever regresses in a future SDK release, the fallback is
explicit `.endpoint_url()` calls in `igait-lib/src/microservice/storage.rs`
and `email.rs`.

## Browser vs container URLs

Frontend `VITE_*` env vars are baked into the Vite bundle at **build time**
and served to the user's browser. That means they must be reachable from
the user's machine, not the compose network — so `VITE_API_BASE_URL` is
`http://localhost:3000` and `VITE_FIREBASE_DATABASE_URL` is
`http://localhost:9000/?ns=igait-local`. Inside the compose network
everything uses service names (`backend`, `firebase-emulator`, etc).

## Port conflicts

MinIO's S3 API runs on 9000 inside the container, which collides with the
Firebase emulator's RTDB port. The compose file publishes MinIO's S3 API to
host port **9002** to avoid this. Inside the compose network both services
listen on their own 9000 — no conflict there.

## What's not tested here

Worker mode does not exercise `orchestrator.rs` (K8s Jobs spawning, RBAC
wiring, Pod lifecycle). Those paths are validated by unit tests and the
real prod cluster. If you need to debug orchestrator code locally, a
`kind`-based harness is the right tool — that's a separate, future effort.

## Gotchas

- **Firebase emulator cold-start is slow** (~30-60s first boot for
  `npm install -g firebase-tools`). The healthcheck has 24 retries to cover
  it. Subsequent boots are instant.
- **Low-RAM machines**: first-time image builds can OOM with parallel
  BuildKit. Use `COMPOSE_BAKE=true docker compose build --parallel 1`
  before `docker compose up`.
- **`credentials/gcp-key.json` is required** even though the emulator
  ignores auth — the Firebase SDK still insists on the file existing.
  `.envrc` materialises it from Vaultwarden.
