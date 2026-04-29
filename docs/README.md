# docs/

Tribal knowledge for iGait. The [root README](../README.md) is for "what is this repo and how do I boot it"; this doc is the role-aware entry point for "I'm actually working on it now." Start with **Everyone** below, then jump to your role section ♪

iGait is a web-based autism screening tool. Users upload a video of someone walking; a 5-stage ML pipeline extracts pose data, detects gait cycles, runs an ML model, and emails a screening result. The codebase is a Rust workspace plus a SvelteKit frontend, coordinated through Firebase RTDB and deployed to a K8s cluster via ArgoCD.

## Monorepo shape

```rust
igait/
├── apps/
│   ├── shared/        // igait-lib: domain types, StageWorker, storage + queue clients
│   ├── backend/       // axum HTTP API + prod orchestrator
│   ├── frontend/      // SvelteKit + Bun
│   └── stages/{5}/    // media-conversion → pose-estimation → cycle-detection → prediction → finalize
├── infra/{docker,k8s,firebase}/
├── docs/              // you are here
├── flake.nix          // Nix devShell (Rust, Bun, ffmpeg, kubectl, aws-cli, …)
└── lefthook.yml       // pre-commit / pre-push gates
```

## Looking for…

- **"How do I run this locally?"** → [`local-dev/hermetic-stack.md`](local-dev/hermetic-stack.md)
- **"How does code reach prod?"** → [`deployment/deploy-flow.md`](deployment/deploy-flow.md)
- **"Where are secrets managed?"** → [`deployment/external-secrets.md`](deployment/external-secrets.md)
- **"How are pipeline stages executed?"** → [`architecture/stage-execution-modes.md`](architecture/stage-execution-modes.md)
- **"What are the quality gates? Why no metrics stack?"** → [`local-dev/dev-loop.md`](local-dev/dev-loop.md)

## Everyone

1. **Prereqs**: Linux or WSL2, Docker, Nix (with flakes), `direnv`. Install links in the [root README](../README.md).
2. **Clone with submodules**: the repo uses git submodules — pass `--recurse-submodules` on clone (the Nix shellHook also initialises them on entry).
3. **First boot**: `docker compose up` brings up the full hermetic stack — frontend, backend, all 5 stages, Firebase emulator, MinIO (S3 surrogate), ses-mock. Zero env vars required. First build is ~5–10 min; subsequent ones are cached.
4. **Seeded admin**: `admin@igait.local` / `admin123` on the login page. Use email/password, **not** Google — see [`local-dev/hermetic-stack.md`](local-dev/hermetic-stack.md#seeded-admin-account).
5. **Before you push**: Lefthook runs `fmt` + `clippy` + `lint` + `check` on every commit and `cargo test --workspace` on every push. Install once per clone: `lefthook install`. Full rationale in [`local-dev/dev-loop.md`](local-dev/dev-loop.md).

### Where to go next

| You are a… | Jump to |
|---|---|
| Frontend dev (SvelteKit, UI) | [`#frontend`](#frontend) |
| Backend dev (Rust HTTP API, orchestrator) | [`#backend`](#backend) |
| Pipeline / ML dev (the 5 stages, shared lib) | [`#pipeline`](#pipeline) |
| Infra / DevOps (K8s, secrets, deploy) | [`#infra`](#infra) |

---

## Frontend

SvelteKit app at `apps/frontend/`.

- **Stack**: SvelteKit (file-based routing) + Bun (package manager + runtime) + bits-ui (Shadcn-style primitives) + Tailwind. Firebase client SDK for auth.
- **State**: no state library. Props + SvelteKit stores. Auth state is owned by the Firebase client SDK (see `auth.svelte.ts`).
- **Route shape**: `(public)/` (home, login, signup, legal pages) and `(authenticated)/` (dashboard, submit, job detail, assistant, admin). The `(authenticated)/` layout enforces the auth guard.
- **Tests**: none yet. Adding a test harness is a design conversation — raise it before writing.
- **API client**: `apps/frontend/src/lib/api/config.ts` owns the `/api/v1` prefix. `VITE_API_BASE_URL` is origin-only — do **not** fold the prefix into the env var (see [`local-dev/hermetic-stack.md`](local-dev/hermetic-stack.md#browser-vs-container-urls)).

**Dev loop**: either let `docker compose up` serve the frontend preview build at `:4173`, or run Vite dev mode standalone for faster HMR against the backend at `:3000`:

```bash
cd apps/frontend
bun install
bun run dev
```

Keep `docker compose up backend firebase-emulator minio …` running alongside so the dev server has something to talk to.

---

## Backend

Rust axum HTTP server at `apps/backend/`.

- **Stack**: axum + tokio, Firebase admin SDK (`firebase-auth` crate) for ID-token verification, `aws-sdk-s3` + `aws-sdk-sesv2`, `kube` for the orchestrator.
- **Route namespaces**:
  - `/api/v1/*` — public, auth-gated endpoints (upload, contribute, assistant, files, cycles, video-edit)
  - `/api/internal/*` — stage callbacks; reachable only from the cluster network
  - `/healthz` — liveness probe
- **Auth**: `firebase-auth` crate verifies `Authorization: Bearer <id-token>`. Local dev short-circuits signature verification via `FIREBASE_AUTH_EMULATOR_HOST` — see [`local-dev/hermetic-stack.md`](local-dev/hermetic-stack.md#firebase_auth_emulator_hostfirebase-emulator9099).
- **Orchestrator gate**: `ENABLE_ORCHESTRATOR=true` spawns the K8s Jobs orchestrator loop in `helper/orchestrator.rs`. **Local dev leaves it unset** (stages self-poll in worker mode). **Prod sets it** (stages run as one-shot K8s Jobs).
- **Firebase client**: `igait_lib::microservice::FirebaseRtdb` is the only Firebase client in this repo. Don't re-introduce `firebase-rs` — see [`architecture/firebase-client.md`](architecture/firebase-client.md) for the backstory (it broke the hermetic stack).

**Dev loop**: `docker compose up backend` is the default. For a tighter Rust rebuild loop:

```bash
cd apps/backend
cargo run --release
```

…but the surrogates (`firebase-emulator`, `minio`, `ses-mock`, plus the two `-bootstrap` one-shots) still need to be up for anything useful to happen.

---

## Pipeline

The 5 stages live at `apps/stages/`. The shared lib — `StageWorker` trait, queue + storage clients — lives at `apps/shared/` as `igait_lib`.

**The stage pattern** (full tour in [`apps/stages/README.md`](../apps/stages/README.md)):

```rust
if std::env::var("IGAIT_JOB_PAYLOAD").is_ok() {
    run_stage_job(Worker).await      // one-shot (prod, K8s Jobs)
} else {
    run_stage_worker(Worker).await   // polling (local, long-running)
}
```

Finalize uses `IGAIT_FINALIZE_PAYLOAD` because its queue item type is `FinalizeQueueItem`, not `QueueItem`. Otherwise identical.

- **Coordination**: Firebase RTDB. A `QueueItem` at `queues/<stage>/<job>` is the unit of work; whoever claims it first (K8s pod or long-running worker) wins. Deep dive in [`architecture/stage-execution-modes.md`](architecture/stage-execution-modes.md).
- **I/O**: stages read/write media via MinIO (local) or S3 (prod). `StorageClient` in `igait_lib` handles both. `AWS_S3_FORCE_PATH_STYLE=true` is load-bearing for MinIO — don't drop it.
- **Tests**: inline `#[cfg(test)]` modules in `apps/shared/src/microservice/`. Integration coverage comes from the hermetic compose stack (CI runs the full upload → pipeline → email path every PR).

**Dev loop**: stages auto-boot under `docker compose up`. After editing a stage's source:

```bash
docker compose restart <stage-name>
```

No hot reload — the container rebuilds its Rust binary on restart. First build is slow; subsequent ones are cached aggressively via cargo-chef (see [`infra/docker/workspace/`](../infra/docker/workspace/README.md)).

---

## Infra

K8s manifests at `infra/k8s/`, Dockerfiles at `infra/docker/`, Firebase rules at `infra/firebase/`.

- **Kustomize, not Helm.** Flat structure — no `base/` + `overlays/`, one prod target. Layout + app-of-apps shape: [`infra/k8s/README.md`](../infra/k8s/README.md).
- **Deploy flow**: push `main` → path-filtered `build-<app>.yml` builds + pushes image → bot auto-commits the new image tag into `infra/k8s/<app>/deployment.yaml` → ArgoCD syncs within seconds. Full diagram and gotchas: [`deployment/deploy-flow.md`](deployment/deploy-flow.md).
- **Secrets**: AWS SSM Parameter Store (`/igait/prod/*`, us-east-2) → External Secrets Operator → K8s Secrets (`igait-secrets`, `gcp-key`, `operator-oauth`). IAM policy, rotation, Tailscale ACL setup: [`deployment/external-secrets.md`](deployment/external-secrets.md).
- **Cluster access**: kubectl from your laptop via Tailscale. **SSH is a cardinal sin** — anything kubectl can do should go through kubectl. Bootstrap recipes: [`deployment/cluster.md`](deployment/cluster.md).
- **No cert-manager, no metrics stack.** Cloudflared tunnel handles external TLS. Logs are the observability surface — rationale in [`local-dev/dev-loop.md`](local-dev/dev-loop.md#observability).

**Dev loop**: there isn't a true iterative loop for infra — changes land through PRs and ArgoCD syncs on merge. For local validation of K8s manifests, `kubectl apply --dry-run=server -k infra/k8s/` catches most syntax/schema issues before pushing.

---

## Doc index

### Local dev
- [`local-dev/hermetic-stack.md`](local-dev/hermetic-stack.md) — docker compose topology, mode flags, load-bearing config, seeded admin
- [`local-dev/dev-loop.md`](local-dev/dev-loop.md) — quality gates (Lefthook + CI), observability stance

### Architecture
- [`architecture/stage-execution-modes.md`](architecture/stage-execution-modes.md) — worker vs K8s Jobs; the single-env-var gate
- [`architecture/firebase-client.md`](architecture/firebase-client.md) — why there's only one Firebase client

### Deployment
- [`deployment/deploy-flow.md`](deployment/deploy-flow.md) — push → build → ArgoCD sync pipeline
- [`deployment/cluster.md`](deployment/cluster.md) — cluster access, kubectl recipes, ArgoCD patch ops
- [`deployment/external-secrets.md`](deployment/external-secrets.md) — AWS SSM → ESO → K8s Secrets

### Environment
- [`environment/vaultwarden.md`](environment/vaultwarden.md) — deprecated at the runtime layer; team password sharing only

## Contributing to docs

Encode *why* alongside *how* so a future refactor can't silently collapse a load-bearing invariant. If a section only describes what the code does, a future reader could have derived it by reading the code — docs earn their keep by pinning invariants, recording precedent, and explaining non-obvious trade-offs. The docs that age well in this repo all share that trait ʕ·ᴥ·ʔ
