# Hermetic local stack

`docker compose up` brings up the full iGait pipeline against local cloud-service surrogates — no AWS, no Firebase project, no verified SES identity. Works offline.

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
    │  queues/<stage-id>                           │
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

| Prod dep | Local surrogate | Notes |
|---|---|---|
| AWS S3 | MinIO | Bucket `igait-storage` auto-created by `minio-bootstrap` |
| AWS SES v2 | aws-ses-v2-local | UI at `:8005`; no `/health` route → no healthcheck |
| Firebase RTDB | Firebase emulator | Project `igait-local`; rules from repo-root `database.rules.json` |
| Firebase Auth | Firebase emulator | Tokens are unsigned JWTs — see *Auth verification* below |
| K8s orchestrator | (disabled) | `ENABLE_ORCHESTRATOR` unset; stages run in worker mode |
| OpenAI | (pass-through) | No local mock; `OPENAI_*` from host `.env` |

## Mode flags

The stack is defined by *what it does not set*:

- `ENABLE_ORCHESTRATOR` unset → backend skips K8s Jobs orchestrator.
- `IGAIT_JOB_PAYLOAD` unset per stage → stages run as workers, not one-shot jobs.
- `IGAIT_FINALIZE_PAYLOAD` unset → finalize runs as a worker too (see `STAGE_EXECUTION_MODES.md`).

See `wiki/architecture/STAGE_EXECUTION_MODES.md` for the dual-mode story.

## Non-obvious configuration (load-bearing)

Each of these was a blocker during #102. If any is removed, the stack breaks in a way whose error message does *not* point at the root cause — so leave them in place unless you're consciously changing the corresponding piece.

### `FIREBASE_AUTH_EMULATOR_HOST=firebase-emulator:9099`

Emulator ID tokens are unsigned JWTs (`alg: none`). The `firebase-auth` crate on the backend verifies against Google's real JWKS by default and rejects them as `InvalidSignature`. Setting this env var switches the crate into "extract claims without verifying" mode — see `verify_id_token_with_project_id` in the crate. Required on the backend; unused by stages (they don't verify tokens).

### `AWS_S3_FORCE_PATH_STYLE=true`

`aws-sdk-s3` defaults to virtual-host addressing (`<bucket>.<host>`), which resolves to `igait-storage.minio` inside the compose network and fails DNS lookup. `StorageClient::with_config` in `igait-lib` reads this env var and flips the client to path-style (`<host>/<bucket>`). Keep on for MinIO; harmless on prod where AWS supports both.

### `FIREBASE_RTDB_URL=http://firebase-emulator:9000/?ns=igait-local`

The emulator requires a `?ns=<project>` query param on every request. `FirebaseRtdb::url()` in `igait-lib` handles a pre-existing query in the base URL by splitting it off, inserting the path, and re-appending — so prod URLs (no query) and emulator URLs (with `?ns=`) both work through the same code path.

### `CORS_ALLOW_ORIGIN=http://localhost:4173`

The backend adds `tower-http::CorsLayer` only if this env var is set. Without it, browser preflight for cross-origin `Authorization`-bearing requests fails silently as a generic "Network error" in the frontend. Prod doesn't set it (same-origin deploy), which is why the backend's default is no CORS layer.

### Firebase emulator needs JRE 21+

The RTDB emulator is a JVM process. `firebase-tools` dropped support for Java <21, so the compose image installs `openjdk21-jre-headless`, not 17 or earlier. First cold boot is 60–90s (JDK install + `npm i -g firebase-tools`); the `start_period: 120s` on the healthcheck covers it.

### `ses-mock` has no healthcheck

`aws-ses-v2-local` has no `/health` route and ships without `curl`/`wget`. There is nothing to probe. Its dependents use `condition: service_started` rather than `service_healthy`. It boots in seconds and the backend only touches it at the end of a pipeline run, so the weaker gate is fine.

## Browser vs container URLs

Frontend `VITE_*` env vars are baked into the Vite bundle at **build time** and served to the user's browser — so they must resolve from the host, not the compose network. `VITE_API_BASE_URL=http://localhost:3000`, `VITE_FIREBASE_DATABASE_URL=http://localhost:9000/?ns=igait-local`. Inside the compose network, everything uses service names (`backend`, `firebase-emulator`, …).

The frontend's `API_ENDPOINTS` (in `igait-frontend/src/lib/api/config.ts`) owns the `/api/v1` version prefix — `VITE_API_BASE_URL` is origin-only. Do not fold the prefix back into the env var "to make things work": it moves the mismatch from code into config and hides future version coexistence.

## Port 9000 collision

MinIO's S3 API and the Firebase RTDB emulator both bind `:9000` inside their containers. The compose file publishes MinIO to host port **9002** to avoid the host-side clash; inside the compose network each service keeps its own 9000 (no conflict — different services, different DNS names).

## What's not tested here

Worker mode does not exercise `orchestrator.rs` (K8s Jobs spawning, RBAC, Pod lifecycle). If you touch orchestrator code, a `kind`-based harness is the right tool — not yet built.
