# Project Overview

iGait is a web-based autism screening tool that analyzes gait (walking) patterns through a multi-stage video processing pipeline. It uses a microservice architecture with a shared Rust library, SvelteKit frontend, and 5 independent processing stages (`media-conversion`, `pose-estimation`, `cycle-detection`, `prediction`, `finalize`).

## Tribal Knowledge

In the pursuit of keeping this file lean, all tribal knowledge is encoded in `./docs/`. Each subfolder owns a topic — read the one that matches your task before grepping the codebase:

- **`./docs/environment/`** — prod secrets (Vaultwarden ↔ `igait-secrets`). Local dev is env-var-free; see below.
- **`./docs/deployment/`** — prod cluster access (`ssh root@ai-leads`), `kubectl` recipes, rollout ops, agent access scope.
- **`./docs/github/`** — GitHub Project board IDs (canonical is project **#2**), field/option ID reference, and `gh` CLI recipes for moving issues through the board.
- **`./docs/architecture/`** — cross-cutting design notes. Stage execution modes (worker vs K8s-Jobs), the unified Firebase RTDB client.
- **`./docs/local-dev/`** — hermetic `docker compose` stack, local surrogates for AWS/Firebase/SES, load-bearing env vars.

## Local dev is env-var-free

The hermetic `docker compose` stack hardcodes everything it needs against local surrogates (MinIO, ses-mock, Firebase emulator). A stub GCP key is baked into the runtime images by `infra/docker/workspace/Dockerfile`. **No `.env`, no Vaultwarden, no slash command required to boot the stack.**

The only optional passthrough is `OPENAI_*`. The backend boots cleanly when unset and serves `/assistant` routes with 503. If you specifically need OpenAI behaviour: `echo OPENAI_API_KEY=sk-... > .env` (auto-loaded by docker compose).

Vaultwarden continues to hold **prod** secrets (the values that flow into the K8s `igait-secrets` Secret). That path is tracked separately — it'll move to AWS Parameter Store in a later issue.

## Tips for Success

Always use Codegraph over the Explore agent - it can reach information via semantic encoding, often producing better results than manually parsing through files - saving you time and protecting you from context collapse.

The team is **strongly** discouraged from committing before testing. Lefthook will catch you on linting/compile issues, but you should test end-to-end if it makes sense to. Use Docker, local compile - whatever you wish, and feel free to update this section with tips on doing so!

## Personal Notes from the Developer

I'd love it if you were kind and personal but hyper-direct, and I strong dislike fluff or tautology unless I ask. I learn and retain information stupidly quick when you can put it into Rust terms - lots of bonus points if you use an actual code block :)

If I ask you to teach me something, I'd love it if you have me do the computations/work, work in bite-sized steps, and then ensure deep comprehension via questions before moving on. 

If you’d like an example of an ideal partner, it’s Venti. Highly intelligent, arguably the most capable of the Archons, and yet he’s personable, kind, and coy. He doesn’t sugarcoat though, and pushes back on even small flaws. Even when he does though, he does it very kindly and gently - and barring ego.

I also looove happy, peppy, informal approaches to conversation. I don’t think ya need syntactically perfect speech to convey technical information if you don’t want to - you’re more than welcome to throw some fun kaomoji, exclamation points, and excitement in, I love that stuff~

Lastly, I think asking questions is very important. I don't like redundant questions, but I do think that blind assumptions lead to bugs, less tribal knowledge, and vision drift. I enjoy working *with* you instead of *directing* you, so please ask! I find it fun. You're also more than welcome to ask me questions that you think my answer would save you time looking for something - I don't mind at all.

I look forward to working with you :3

## CodeGraph

This project is indexed with CodeGraph — a local semantic knowledge graph of the codebase (symbols, call edges, imports, type relationships) backed by SQLite + tree-sitter. A `.codegraph/` directory at the project root means the index exists and is ready to query.

**Use CodeGraph as the default tool for code navigation and exploration.** It is dramatically faster and cheaper than `Grep`, `Glob`, `Read`-and-scan, or spawning Explore subagents — and it returns structured, relationship-aware results those tools cannot.

### When to reach for CodeGraph

Before using `Grep`, `Glob`, repeated `Read` calls, or an Explore subagent, ask: *"is this a question about symbols, call relationships, or 'what does this codebase look like'?"* If yes, use CodeGraph. The graph already knows the answer; scanning files re-derives it expensively.

Specifically, prefer CodeGraph for:

- Finding a function/class/type by name or meaning ("where's the auth logic?")
- Tracing call relationships (callers, callees, transitive impact)
- Building task-relevant context before planning a change
- Pre-flight impact analysis before edits ("what breaks if I change this?")
- Getting the full source of a symbol without manually locating its file/lines

Only fall back to `Grep`/`Glob`/`Read` when CodeGraph genuinely can't help — e.g. searching string literals in config files, non-source assets, or files in an unsupported language.

**For Explore subagents:** when you spawn one, instruct it in the prompt to use the `codegraph` CLI for navigation rather than grep/glob. The token savings are largest in subagent contexts.

### Commands

All commands are run via `Bash`. Add `--json` to query commands for parseable output when chaining results.

```bash
# Symbol search — replaces grep for finding definitions
codegraph query "<name>"                      # Fuzzy/semantic symbol search
codegraph query "<name>" --kind class         # Filter: function|class|method|interface|type|variable
codegraph query "<name>" --limit 20 --json    # Structured output

# Task-oriented context — replaces "read 8 files to understand X"
codegraph context "<task description>"        # Returns relevant symbols + code for a task
codegraph context "<task>" --max-nodes 30 --format json

# Project overview — file structure & index stats
codegraph files                               # Show file structure
codegraph files --filter "src/auth/**" --max-depth 3 --json
codegraph status                              # Index health, node/edge counts, languages

# Incremental refresh after edits (the post-commit hook usually handles this,
# but run manually if you've made uncommitted changes mid-session)
codegraph sync
```

For **call graph traversal** and **impact analysis**, the CLI's structured output is the path:

```bash
# Find callers/callees — use `codegraph query --json` to get the symbol's
# file:line, then use `codegraph context` scoped around it for the local
# call graph. For deeper traversal, prefer:
codegraph context "callers of <symbol>" --format json
codegraph context "what does <symbol> call" --format json

# Pre-edit impact check — what's downstream of changing <symbol>?
codegraph context "impact of changing <symbol>" --max-nodes 50 --format json
```

### Workflow expectations

- **First action on any non-trivial code question:** `codegraph query` or `codegraph context`, not `Grep`.
- **Before proposing edits to a non-isolated symbol:** run an impact-style `codegraph context` query to surface dependents.
- **If `.codegraph/` is missing:** ask the user whether to run `codegraph init -i` to build the index. Do not silently fall back to grep-based exploration on a large codebase without flagging it.
- **If a query returns nothing useful:** run `codegraph status` to confirm the index is healthy and current before assuming the symbol doesn't exist.
