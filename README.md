# iGait
iGAIT is an innovative, objective, equitable, widely accessible, easy-to-use, free web-based tool for early autism screening.

*If you are a developer looking for onboarding, skip to [Development](#development)*

## Architecture
iGait uses a microservice-based architecture due to its multi-stage pipeline:
- **1.** The process begins on the frontend, where the users submits their input. 
- **2.** The central backend then receives it, handling user/job creation and initial submission emails
- **3-9.** The pipeline then steps in, executing each stage. The final stage accumulates the final result or failure, and decides how to convey this information to the user.

All microservices and the backend share a common library (`igait-lib`) to facilitate the common grounds each stage and the backend have in common.

The process of I/O is done atomically through Google Firebase RTDB and AWS S3. 

### Google Firebase RTDB and the Queue System
RTDB holds a queue for each stage, which is how each stage knows what to work on. 

Each stage deployment works on one queue entry at a time - so to scale a specific deployment that slows the rest, simply increase the number of deployments. They can work independently! 

The backend is the first point of entry - it adds the entry to the first queue for the first stage to pick up.

### AWS S3, Persistant Storage, and Stage I/O
Files are first uploaded by the backend to S3, and the backend never sees them again. In general, both the pipeline and the backend never hold onto their inputs! 

Each stage then pulls the files from S3, performs some modification or check, and then uploads the modified files for the next stage to work on. After doing so, it adds the job to the next stage's queue.

It's a shockingly simple approach to an otherwise incredibly complex process, and allows a ton of visibility into data as it flows from step to step.

## Development
### Dependencies
Please use Linux or [WSL2](https://learn.microsoft.com/en-us/windows/wsl/install) to work on this repository. It's **strongly** encouraged not to use Windows.

You'll want to have the following installed on your machine:
- [Docker](https://www.docker.com/)
- [Nix](https://nixos.org/download/). Enable [Nix Flakes](https://nixos.wiki/wiki/flakes)
- [`direnv`](https://direnv.net/) — auto-enters the Nix dev shell and sources `.env` on `cd` into the repo. Hook your shell per the direnv docs.
- [Claude Code](https://claude.com/claude-code) — the `/igait-environment` slash command materialises `.env` from Vaultwarden

First, download this repository (note the submodules!):
```bash
git clone --recurse-submodules https://github.com/igait-niu/igait.git
cd igait
```

Then, set up access to secrets. We use [Vaultwarden](https://vault.igaitapp.com) — no more shared dotfiles on OneDrive. Ask @hiibolt for an invite to the `igait-niu` organization if you don't already have one.

```bash
# One-time per machine:
bw config server https://vault.igaitapp.com
bw login                           # interactive: email + master password + 2FA

# One-time per shell (or put in your rc — the session token lives here):
export BW_SESSION=$(bw unlock --raw)
```

Then, inside this repo, run the Claude Code slash command **`/igait-environment`**. It reads the `igait/dev-env` item and writes `./.env` — every custom field, auto-loaded by `docker compose`. (The legacy `GCP_KEY_JSON` field is now ignored; `.docker/workspace/Dockerfile` bakes a stub key into every runtime image for local/CI, and prod mounts the real key as a K8s Secret.)

Re-run `/igait-environment` whenever a secret is rotated in Vaultwarden, then `direnv reload` in any open shell. The tracked `.envrc` only does two things: `use flake` (Nix dev shell) and `dotenv_if_exists .env` (source the file the slash command wrote). It does **not** touch Vaultwarden — auth is a conscious action, not per-`cd` churn. See `wiki/environment/VAULTWARDEN.md` for the field-editing workflow.

**Optional**:
I strongly recommend using [Visual Studio Code](https://code.visualstudio.com/) with the Svelte and `rust-analyzer` extensions! 

Additionally, if you choose to use GitHub Copilot, repository context and MCPs are tracked in this repostitory.

### Starting iGait

The fastest path to a working stack is the **hermetic local stack** — a single
`docker compose up` brings the backend, frontend, all 5 pipeline stages, and
three local cloud-service surrogates online. No AWS, no Firebase project, no
SES identity required; you can develop on airplane wifi.

```bash
# (first time, or after a Vaultwarden rotation)
/igait-environment  # Claude Code slash command — writes .env from Vaultwarden

docker compose up   # first boot: ~5-10min for Rust/Python image builds
```

When it's up, open:

| Endpoint | URL | Notes |
|----------|-----|-------|
| Frontend | http://localhost:4173 | SvelteKit preview build |
| Backend | http://localhost:3000 | API, direct uploads |
| Firebase emulator UI | http://localhost:4000 | RTDB tree, queues, job state |
| MinIO console | http://localhost:9001 | login: `minioadmin` / `minioadmin` |
| SES mock UI | http://localhost:8005 | captured result emails |

**What's tested end-to-end:** upload → S3 → 5 stage pipeline → result email.
**What's not tested:** the K8s Jobs orchestrator code path (stages run in
worker mode here; see `wiki/architecture/stage-execution-modes.md`).

**Low-RAM machines (<16GB):** BuildKit parallelises stage image builds by
default, which can be rough. Serialise with:

```bash
COMPOSE_BAKE=true docker compose build --parallel 1
docker compose up -d
```

**Cold-start note:** the Firebase emulator's first boot installs
`firebase-tools` via npm and can take 30-60s — its healthcheck has 24
retries to accommodate. Subsequent boots are instant.

Troubleshooting and deeper design notes live in
`wiki/local-dev/hermetic-stack.md`.

### Working on iGait
**Backend/Pipeline**:
Each pipeline stage and the backend can (and should!) be worked on entirely independently.

Starting a stage/the backend is as simple as navigating to its respective folder and running the following, where `<port>` is the port you'd like to start it on:
```bash
export PORT=<port> && cargo run --release
```

**Frontend**:
Working on the frontend is a roughly the same, navigate to its folder:
```bash
bun install
bun run dev
```