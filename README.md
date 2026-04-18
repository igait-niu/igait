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

Then, download the `.envrc` and `gcp-key.json` files from "iGait Credentials/Monorepo" in Onedrive. Place `.envrc` in the project root, and `gcp-key.json` at `credentials/gcp-key.json`.

If you've correctly hooked your shell with `direnv`, you should see the following:
```bash
direnv: error /home/hiibolt/igait/.envrc is blocked. Run `direnv allow` to approve its content
```
...if not, go back and ensure you installed/hooked correctly.

If you're using WSL2, you may need to normalize `.envrc` from CRLF to LF format. This can be done by selecting the file in Visual Studio Code, clicking the "CRLF" button in the footer panel, and selecting LF instead.

Next, run `direnv allow` as it suggests, and it should automatically download all dependencies, and load all environment variables. Neat, right?

**Optional**:
I strongly recommend using [Visual Studio Code](https://code.visualstudio.com/) with the Svelte and `rust-analyzer` extensions! 

Additionally, if you choose to use GitHub Copilot, repository context and MCPs are tracked in this repostitory.

### Starting iGait
Please save any work open on your machine before starting the application - Docker will try to build the entire application in parallel by default, which even beefy machines will struggle with for a first-time build. My laptop usually bricks itself when I try.

If your machine has under 64gb of RAM, first run the following commands to build a cache - it's normal to take quite a while the first time:
```bash
docker compose build web
docker compose build backend
docker compose build media-conversion
docker compose build validity-check
docker compose build reframing
docker compose build pose-estimation
docker compose build cycle-detection
docker compose build prediction
docker compose build finalize
```

Once you've built this initial cache, you can build and run iGait with the following:
```bash
docker compose up --build
```

This will bring the entire stack online - frontend, backend, and pipeline stages! You can view the frontend now at https://localhost:4173.

Other useful commands:
- `docker compose down` - Takes iGait offline
- `docker compose logs <container>` - View the logs

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