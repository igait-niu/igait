# iGait

<div align="center">

📖 **Working on the codebase?** Head to [**`docs/README.md`**](docs/README.md).

<sub>Role-aware onboarding (frontend / backend / pipeline / infra), architecture deep-dives, deploy flow, secrets, and kubectl recipes all live there.</sub>

</div>

---

iGAIT is an innovative, objective, equitable, widely accessible, easy-to-use, free web-based tool for early autism screening.

## Architecture at a glance

iGait is a microservice-based, multi-stage pipeline:

1. **Frontend** receives the user's video upload.
2. **Backend** handles user/job creation and the initial submission email, then drops the job onto a Firebase RTDB queue.
3. **Pipeline stages** (5 of them) each pull from their own queue, transform files via S3, and enqueue for the next stage.
4. The **finalize stage** accumulates the result or failure and emails the user.

I/O is atomic through Firebase RTDB (queues + job state) and AWS S3 (media files). All services share `apps/shared` (`igait-lib`) for domain types and clients. Each stage scales independently — a slow stage just gets more replicas. Deeper architecture notes (stage execution modes, the Firebase client invariant, the queue contract) live under [`docs/architecture/`](docs/architecture/).

## Quick start

Linux or [WSL2](https://learn.microsoft.com/en-us/windows/wsl/install) only — Windows is **strongly** discouraged. Install [Docker](https://www.docker.com/), [Nix](https://nixos.org/download/) (with [flakes](https://nixos.wiki/wiki/flakes) enabled), and [`direnv`](https://direnv.net/) (auto-enters the Nix dev shell on `cd`).

```bash
git clone --recurse-submodules https://github.com/igait-niu/igait.git
cd igait
docker compose up   # zero-config; first boot ~5–10min
```

Then open the frontend at <http://localhost:4173>. The hermetic stack is **env-var-free** — no AWS, no Firebase project, no `.env` required to boot. The only optional passthrough is `OPENAI_*` in a repo-root `.env` if you're working on `/assistant` routes.

For the full topology, port map, troubleshooting, and the *why* behind every load-bearing config flag, see [`docs/local-dev/hermetic-stack.md`](docs/local-dev/hermetic-stack.md). For everything else — role-aware dev loops, deploy flow, secrets, kubectl recipes — head to [`docs/README.md`](docs/README.md).
