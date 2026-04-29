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
| OpenAI | (optional) | Backend boots without `OPENAI_API_KEY`; `/assistant` routes return 503. Set `OPENAI_*` in a repo-root `.env` to enable. |

## Mode flags

The stack is defined by *what it does not set*:

- `ENABLE_ORCHESTRATOR` unset → backend skips K8s Jobs orchestrator.
- `IGAIT_JOB_PAYLOAD` unset per stage → stages run as workers, not one-shot jobs.
- `IGAIT_FINALIZE_PAYLOAD` unset → finalize runs as a worker too (see `stage-execution-modes.md`).

See `docs/architecture/stage-execution-modes.md` for the dual-mode story.

## Non-obvious configuration (load-bearing)

Each of these was a blocker during #102. If any is removed, the stack breaks in a way whose error message does *not* point at the root cause — so leave them in place unless you're consciously changing the corresponding piece.

### `FIREBASE_AUTH_EMULATOR_HOST=firebase-emulator:9099`

Emulator ID tokens are unsigned JWTs (`alg: none`). The `firebase-auth` crate on the backend verifies against Google's real JWKS by default and rejects them as `InvalidSignature`. Setting this env var switches the crate into "extract claims without verifying" mode — see `verify_id_token_with_project_id` in the crate. Required on the backend; unused by stages (they don't verify tokens).

### `AWS_S3_FORCE_PATH_STYLE=true`

`aws-sdk-s3` defaults to virtual-host addressing (`<bucket>.<host>`), which resolves to `igait-storage.minio` inside the compose network and fails DNS lookup. `StorageClient::with_config` in `apps/shared` reads this env var and flips the client to path-style (`<host>/<bucket>`). Keep on for MinIO; harmless on prod where AWS supports both.

### `FIREBASE_RTDB_URL=http://firebase-emulator:9000/?ns=igait-local`

The emulator requires a `?ns=<project>` query param on every request. `FirebaseRtdb::url()` in `apps/shared` handles a pre-existing query in the base URL by splitting it off, inserting the path, and re-appending — so prod URLs (no query) and emulator URLs (with `?ns=`) both work through the same code path.

### `CORS_ALLOW_ORIGIN=http://localhost:4173`

The backend adds `tower-http::CorsLayer` only if this env var is set. Without it, browser preflight for cross-origin `Authorization`-bearing requests fails silently as a generic "Network error" in the frontend. Prod doesn't set it (same-origin deploy), which is why the backend's default is no CORS layer.

### Firebase emulator is a prebuilt image

The RTDB emulator is a JVM process, and `firebase-tools` dropped support for Java <21. Rather than installing openjdk + firebase-tools on every boot (which historically took several minutes and periodically blew past the healthcheck window, cascading failures through every `service_healthy` dependent), the emulator is now built from `infra/docker/firebase-emulator/Dockerfile` — it bakes in JRE 21, `firebase-tools`, the pre-downloaded emulator jars (`firebase setup:emulators:{auth,database}`), and `.firebaserc`. First `docker compose build` takes ~3–5 min once; every boot after that is JVM startup (~5–10s), so `start_period: 30s` is plenty.

If you bump the `firebase-tools` version or swap JREs, rebuild with `docker compose build firebase-emulator`.

### `ses-mock` has no healthcheck

`aws-ses-v2-local` has no `/health` route and ships without `curl`/`wget`. There is nothing to probe. Its dependents use `condition: service_started` rather than `service_healthy`. It boots in seconds and the backend only touches it at the end of a pipeline run, so the weaker gate is fine.

## Seeded admin account

The `firebase-bootstrap` service runs on every `docker compose up` and seeds a deterministic admin into both emulators so the admin panel is reachable without manual setup.

| Field | Value |
|---|---|
| Email | `admin@igait.local` |
| Password | `admin123` |
| RTDB record | `users/<uid>/administrator = true` |

Sign in via the **email/password** form on the login page — not Google OAuth. Rationale: the Google popup against the emulator opens a fake provider-picker page that's awkward for iterative dev, while `signInWithEmailAndPassword` (in `auth.svelte.ts`) talks directly to `:9099` and returns a usable emulator ID token the backend accepts (because `FIREBASE_AUTH_EMULATOR_HOST` is set — see above).

Emulator state is ephemeral (no volume mount on `firebase-emulator`), so this seed has to run every boot. The bootstrap is idempotent: on restart `accounts:signUp` returns `EMAIL_EXISTS`, and the script falls back to `accounts:lookup` to recover the uid before the RTDB PUT. Want another admin or a non-admin account? Register normally through the frontend — the second account won't have `administrator: true` unless you flip the flag in the emulator UI at `:4000`.

## Browser vs container URLs

Frontend `VITE_*` env vars are baked into the Vite bundle at **build time** and served to the user's browser — so they must resolve from the host, not the compose network. `VITE_API_BASE_URL=http://localhost:3000`, `VITE_FIREBASE_DATABASE_URL=http://localhost:9000/?ns=igait-local`. Inside the compose network, everything uses service names (`backend`, `firebase-emulator`, …).

The frontend's `API_ENDPOINTS` (in `apps/frontend/src/lib/api/config.ts`) owns the `/api/v1` version prefix — `VITE_API_BASE_URL` is origin-only. Do not fold the prefix back into the env var "to make things work": it moves the mismatch from code into config and hides future version coexistence.

## Low-RAM machines (<16GB)

BuildKit parallelises stage image builds by default, which can OOM-kill on a constrained host (5 stages × Rust compiles is a lot of concurrent memory). Serialise the build, then bring the stack up:

```bash
COMPOSE_BAKE=true docker compose build --parallel 1
docker compose up -d
```

## Port 9000 collision

MinIO's S3 API and the Firebase RTDB emulator both bind `:9000` inside their containers. The compose file publishes MinIO to host port **9002** to avoid the host-side clash; inside the compose network each service keeps its own 9000 (no conflict — different services, different DNS names).

## What's not tested here

Worker mode does not exercise `orchestrator.rs` (K8s Jobs spawning, RBAC, Pod lifecycle). If you touch orchestrator code, a `kind`-based harness is the right tool — not yet built.
