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
- [`direnv`](https://direnv.net/), for fast dev environment loading. Be sure to hook your shell!

First, download this repository (note the submodules!):
```bash
git clone --recurse-submodules https://github.com/igait-niu/igait.git
cd igait
```

Then, set up access to secrets. We use [Vaultwarden](https://vault.igaitapp.com) — no more shared dotfiles on OneDrive. Ask @hiibolt for an invite to the `igait-niu` organization if you don't already have one.

```bash
# One-time setup on this machine:
bw config server https://vault.igaitapp.com
bw login                           # interactive: Vaultwarden email + master password + 2FA

# Every shell (or add to your shell rc for persistence):
export BW_SESSION=$(bw unlock --raw)
```

The GCP service-account key is now stored in the same `igait/dev-env` Vaultwarden
item as a custom field named `GCP_KEY_JSON`. `.envrc` writes it to
`credentials/gcp-key.json` (mode 600) on every shell load — no more manual
OneDrive fetches. If you're setting this up for the first time and the field
is missing, `.envrc` will warn you; ask @hiibolt for the payload.

If `direnv` is hooked into your shell, entering the repo prints:
```bash
direnv: error /home/you/igait/.envrc is blocked. Run `direnv allow` to approve its content
```
...if not, go back and ensure you installed/hooked correctly.

Run `direnv allow`. The tracked `.envrc` calls `bw get item igait/dev-env` and exports every custom field on that item as an env var in your shell. When another engineer rotates a secret in Vaultwarden, you pick it up on the next shell reload (or explicitly with `bw sync && direnv reload`).

**Optional**:
I strongly recommend using [Visual Studio Code](https://code.visualstudio.com/) with the Svelte and `rust-analyzer` extensions! 

Additionally, if you choose to use GitHub Copilot, repository context and MCPs are tracked in this repostitory.

### Starting iGait

The fastest path to a working stack is the **hermetic local stack** — a single
`docker compose up` brings the backend, frontend, all 5 pipeline stages, and
three local cloud-service surrogates online. No AWS, no Firebase project, no
SES identity required; you can develop on airplane wifi.

```bash
direnv allow        # loads env + materialises credentials/gcp-key.json
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