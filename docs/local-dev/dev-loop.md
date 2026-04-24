# Dev loop

Quality gates, CI parity, and observability — what runs when you code,
what runs when you push, and how to debug what ran in prod.

## Quality gates

iGait runs two tiers of local gates via Lefthook, each mirrored in CI:

| When | What | Why |
|---|---|---|
| **pre-commit** | `cargo fmt --check`, `cargo clippy --workspace -- -D warnings`, `bun run lint`, `bun run check` | Fast. Blocks a commit that would fail CI's style/lint gates. |
| **pre-push** | `cargo test --workspace` | Slower. Blocks pushing a commit that would fail CI's test matrix. Runs only on push because running tests every commit kills iteration speed. |

Install once per clone: `lefthook install`. Skip for a single commit:
`LEFTHOOK=0 git commit …` — reserved for emergency bypass.

### Scope nuance worth knowing

- **Pre-commit's clippy is `--workspace`; CI's clippy is per-crate (matrix).**
  Same lint ruleset, different dispatch. The per-crate shape exists only
  to parallelize across GH runners — don't try to "fix" the local side to
  match.
- **Dep bumps retrigger Rust jobs even with no `.rs` diff.** Pre-commit's
  Rust globs include `Cargo.toml`/`Cargo.lock`, because a bad dep bump
  can break clippy/tests without touching any source file.
- **Frontend jobs auto-install `node_modules`** on first run (`bun
  install --frozen-lockfile`). Triggers on `*.{ts,js,svelte,css,html,json,lock}`
  under `apps/frontend/`.

## CI parity

`.github/workflows/ci.yml` runs the same four jobs (fmt / clippy / test
/ frontend lint+check) plus one more:

- **`integration`** — brings up `infra/compose.yml` with `--wait
  --timeout 300`, exercises upload → S3 → 5-stage pipeline → result
  email, tears down on finish. Uploads compose logs on failure.

CI's integration job passes explicit env vars (service-name hostnames
for the compose network) rather than relying on `docker compose`'s
auto-loading. The hermetic stack itself remains env-var-free for local
dev — CI just parameterizes it for its ephemeral runner environment.

CI's `paths-ignore: infra/k8s/**` sits out the bot's deploy commits
(see [`../deployment/deploy-flow.md`](../deployment/deploy-flow.md)).

## Observability

**Stance: logs-first by choice, until scale demands otherwise.** No
Prometheus, no Grafana, no Loki, no OpenTelemetry collector. This is a
deliberate decision, not a gap — the current scale doesn't justify the
operator-burden of a metrics stack. When it does, revisit.

### Tools in use today

- **`kubectl logs`** — primary for every prod-level debug question.
  `--all-containers`, `--follow`, `--prefix=true`, label selectors
  (`-l app=backend`). Recipes in [`../deployment/cluster.md`](../deployment/cluster.md).
- **ArgoCD UI** — deploy state, sync status, manifest diffs, rollout
  health. Read-only most of the time; manual patches are rare.
- **Firebase emulator UI** (local only, `:4000`) — inspect RTDB state,
  queues, job records. The Firebase console is the prod equivalent.
- **`tracing-subscriber`** in every Rust service — structured logs to
  stdout, picked up by kubelet and surfaced via `kubectl logs`. Set
  `RUST_LOG` to raise or lower the level (configured via `igait-secrets`
  in prod, ambient env in dev).
- **`docker compose logs -f <svc>`** (local) — live tailing for a
  specific service.

### When to escalate

If you find yourself reading logs across multiple stages to debug a
distributed behavior and it genuinely feels impossible — that's the
signal to propose adding a metrics/trace stack. Until then, Firebase
RTDB is the source of truth for pipeline state, `kubectl logs` is the
source of truth for "what did the code do," and the two together cover
today's debugging load.

## Related reading

- [`hermetic-stack.md`](./hermetic-stack.md) — local dev topology, load-bearing env vars
- [`../architecture/stage-execution-modes.md`](../architecture/stage-execution-modes.md) — why stages are dual-mode
- [`../deployment/deploy-flow.md`](../deployment/deploy-flow.md) — how code reaches prod
- [`../deployment/cluster.md`](../deployment/cluster.md) — kubectl recipes
