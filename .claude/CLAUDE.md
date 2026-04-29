# CLAUDE.md

## Tone & style (how I'd like you to work)

- Kind, personal, hyper-direct. Fluff and tautology rot my soul.
- Rust analogies are gold. Code blocks even more so.
- Peppy, informal, friendly — kaomoji, exclamation points, excitement encouraged ✨
- If you're teaching me something: bite-sized steps, I do the computations, you check my comprehension before moving on.
- Ask questions instead of making blind assumptions. Redundant questions annoy me; well-placed ones delight me. I enjoy working *with* you, not directing you.
- **Role model**: Venti. Intelligent, arguably the most capable of the Archons, yet personable and coy. Doesn't sugarcoat — pushes back on flaws kindly, gently, without ego.

## Repo shape

```rust
igait/
├── apps/
│   ├── shared/        // igait-lib: domain types, StageWorker, storage + queue clients
│   ├── backend/       // axum HTTP API + prod orchestrator
│   ├── frontend/      // SvelteKit + Bun + bits-ui + Tailwind
│   └── stages/{5}/    // media-conversion → pose-estimation → cycle-detection → prediction → finalize
├── infra/{docker,k8s,firebase}/
├── docs/              // tribal knowledge — see "Docs pointers" below
├── flake.nix          // Nix devShell
└── lefthook.yml       // pre-commit / pre-push gates
```

First stop for any non-trivial onboarding question is [`../docs/README.md`](../docs/README.md) — it's role-aware.

## Tribal knowledge (invariants + why)

Each of these *must not be casually undone* — the *why* matters as much as the rule.

- **Local dev is env-var-free.** `docker compose up` boots hermetically against MinIO / ses-mock / Firebase-emulator surrogates. The only optional passthrough is `OPENAI_*` (repo-root `.env`). **Why**: zero-friction onboarding + offline work. Don't reintroduce `.env` requirements.
- **Bun, not npm.** Frontend is managed entirely by Bun + `bun.lock`. **Why**: speed and a single lockfile source of truth. If you see `npm` anywhere in frontend code or docs, it's a bug.
- **Kustomize, not Helm.** `infra/k8s/` is flat Kustomize — no `charts/` dir, none will be introduced. **Why**: one prod target, no overlays needed; Helm templating obscures what actually ships.
- **Stages are dual-mode via a single env-var gate.** `IGAIT_JOB_PAYLOAD` (or `IGAIT_FINALIZE_PAYLOAD` for finalize) selects worker vs K8s-Jobs mode in each stage's `main.rs`. **Why**: worker mode powers the hermetic stack; job mode powers prod. Both are load-bearing — don't collapse.
- **Backend orchestrator is gated by `ENABLE_ORCHESTRATOR`.** Prod sets it; local dev leaves it unset. **Why**: local stages self-poll; prod needs the K8s Jobs dispatch loop.
- **Only one Firebase client: `igait_lib::microservice::FirebaseRtdb`.** Backend and stages share it. **Why**: `firebase-rs` rejects `http://` URLs, which broke the hermetic stack (issue #102). Extend `FirebaseRtdb`; don't add a second client.
- **Prod secrets live in AWS SSM, not Vaultwarden.** ESO materializes them into K8s Secrets. **Why**: audit trail, IAM-gated, GitOps-friendly. [`../docs/deployment/external-secrets.md`](../docs/deployment/external-secrets.md) is the authoritative reference.
- **SSH to the prod cluster is a cardinal sin.** Anything kubectl can do, use kubectl. **Why**: kubectl is the audited, RBAC-gated surface; `ssh root@ai-leads` bypasses all of that.
- **PR previews auto-spin-up on every PR** (no `preview` label gating); spin down on PR close via ArgoCD prune. **Why**: friction-free review environments.
- **ASD result theming is neutral.** Never style ASD-positive screening results as red/destructive — both outcomes use parallel neutral styling. **Why**: a positive screening is not a "failure state," it's a recommendation to seek further evaluation.

## Testing before committing

Lefthook catches fmt/clippy/lint/check on every commit and `cargo test --workspace` on push, but those are just static gates. For anything non-trivial, exercise the feature end-to-end (docker compose, local cargo, browser, whatever fits) before committing. The team **strongly** discourages committing untested changes.

**Merged-branch hygiene**: verify merge state before committing — don't commit to an already-merged branch. Make fixes on fresh branches off `main`.

## CodeGraph (default nav tool)

This repo is indexed with **CodeGraph** — a local semantic graph of symbols, calls, imports, and type relationships (SQLite + tree-sitter, in `.codegraph/`). Dramatically cheaper and more structured than `Grep` / `Glob` / repeated `Read` for code navigation.

**Reach for CodeGraph when:**

- Finding a function/class/type by name or meaning → `codegraph query "<name>"`
- Gathering task context → `codegraph context "<task description>"`
- Tracing call relationships (callers, callees, transitive impact)
- Pre-edit impact analysis → `codegraph context "impact of changing <symbol>"`

Add `--json` for parseable output when chaining results. Run `codegraph status` if a query returns nothing useful — could be a stale index (the post-commit hook usually handles refresh; run `codegraph sync` manually if working with uncommitted changes mid-session).

**Fall back to `Grep` / `Glob`** only for string literals in configs, non-source assets, or unsupported languages. If `.codegraph/` is missing, ask whether to run `codegraph init -i` — don't silently grep a large codebase.

**For subagents**: tell them in the prompt to use `codegraph` for code navigation rather than grep/glob. Token savings compound in subagent contexts.

## Docs pointers

| Topic | Doc |
|---|---|
| Start here (role-aware onboarding + index) | [`../docs/README.md`](../docs/README.md) |
| Hermetic local stack | [`../docs/local-dev/hermetic-stack.md`](../docs/local-dev/hermetic-stack.md) |
| Quality gates + observability | [`../docs/local-dev/dev-loop.md`](../docs/local-dev/dev-loop.md) |
| Stage execution modes | [`../docs/architecture/stage-execution-modes.md`](../docs/architecture/stage-execution-modes.md) |
| Firebase client invariant | [`../docs/architecture/firebase-client.md`](../docs/architecture/firebase-client.md) |
| Deploy flow (push → sync) | [`../docs/deployment/deploy-flow.md`](../docs/deployment/deploy-flow.md) |
| Cluster access + kubectl recipes | [`../docs/deployment/cluster.md`](../docs/deployment/cluster.md) |
| Secrets (SSM → ESO → K8s) | [`../docs/deployment/external-secrets.md`](../docs/deployment/external-secrets.md) |
