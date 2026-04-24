# Deploy flow

How code reaches the prod cluster — start to finish.

## The pipeline

```
┌──────────────┐
│ push to main │
└──────┬───────┘
       │ (path filter matches apps/<name>/** or apps/shared/** etc.)
       ▼
┌──────────────────────────────┐
│ build-<app>.yml              │  build multi-arch image →
│ (one per app/stage)          │  ghcr.io/igait-niu/igait/<app>:sha-<7>
│                              │  + :latest
└──────┬───────────────────────┘
       │ (GH Actions bot commits with DEPLOY_SSH_KEY)
       ▼
┌──────────────────────────────────────────┐
│ infra/k8s/<app>/deployment.yaml          │
│ image: ...:sha-<7>  ← sed-replaced       │
│ committed: "deploy: <app>:sha-<7>"       │
│ pushed to main (up to 5 retries on race) │
└──────┬───────────────────────────────────┘
       │ (ArgoCD watches infra/k8s/)
       ▼
┌──────────────────────────────────────────┐
│ ArgoCD syncs within seconds              │
│ selfHeal: true  •  prune: true           │
└──────────────────────────────────────────┘
```

## Path-filter matrix

Each workflow watches only what affects its image:

| Workflow | Trigger paths |
|---|---|
| `build-backend.yml` | `apps/backend/**`, `apps/shared/**`, Cargo files, workspace Dockerfile |
| `build-web.yml` | `apps/frontend/**` |
| `build-media-conversion.yml` | `apps/stages/media-conversion/**`, `apps/shared/**`, Cargo files, workspace Dockerfile |
| `build-pose-estimation.yml` | (same shape, for its stage) |
| `build-cycle-detection.yml` | same |
| `build-prediction.yml` | same |
| `build-finalize.yml` | same |

Every Rust-app workflow path-filters on `apps/shared/**` because the
shared lib is a workspace dep — a change there must rebuild everything
that links it.

## Why CI doesn't retrigger on bot commits

`.github/workflows/ci.yml` has `paths-ignore: infra/k8s/**`. The bot's
`deploy:` commits only touch `infra/k8s/<app>/deployment.yaml`, so CI
sits them out — no redundant test runs, no feedback loop. Real code
changes always come in through PRs, where CI runs once before merge.

## Rollback

Image tags are immutable in ghcr once pushed, so rollback is a git op:
revert the `deploy:` commit on `main`. ArgoCD picks up the reverted
image tag on its next sync.

For a rollback to a specific earlier sha without reverting git state,
bypass ArgoCD temporarily by editing the live Deployment via `kubectl`,
then undo the edit before ArgoCD's next reconcile (`selfHeal` will
revert to the git-sourced tag anyway). Recipes in
[`cluster.md`](./cluster.md).

## Gotchas

- **Path restructures break ArgoCD.** Moving files under `infra/k8s/`
  requires patching live Application CRs — `selfHeal` can't fix a
  `path does not exist` error because the Application itself can't
  read its own manifest. Every restructure PR must include the
  corresponding `kubectl patch` commands. See
  [`cluster.md`](./cluster.md) § "Re-point ArgoCD apps after a path
  restructure."
- **The `DEPLOY_SSH_KEY` secret is load-bearing.** It's how the
  build workflow pushes back to `main`. Rotating it without updating
  the repo secret silently breaks deploys.
- **5-retry push loop on race.** Multiple apps can finish building
  simultaneously; only one `git push` wins per cycle. The workflow
  re-fetches and re-seds up to 5 times. If all 5 fail, the image is
  built but not deployed — re-run the workflow to retry the commit.

## Related reading

- [`cluster.md`](./cluster.md) — kubectl recipes, ArgoCD patch ops
- [`external-secrets.md`](./external-secrets.md) — AWS SSM → ESO → K8s
- [`../local-dev/dev-loop.md`](../local-dev/dev-loop.md) — quality gates that run before this pipeline
