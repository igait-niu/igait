# docs/

Tribal knowledge for iGait. The [root README](../README.md) is the first stop for "what is this repo and how do I boot it"; everything below is for "I'm actually working on it now."

## Start here (by role)

| Role | Read first |
|---|---|
| Everyone | [`onboarding.md`](onboarding.md) § Everyone |
| Frontend dev | [`onboarding.md#frontend`](onboarding.md#frontend) |
| Backend dev | [`onboarding.md#backend`](onboarding.md#backend) |
| Pipeline / ML dev | [`onboarding.md#pipeline`](onboarding.md#pipeline) |
| Infra / DevOps | [`onboarding.md#infra`](onboarding.md#infra) |

## Looking for…

- **"How do I run this locally?"** → [`local-dev/hermetic-stack.md`](local-dev/hermetic-stack.md)
- **"How does code reach prod?"** → [`deployment/deploy-flow.md`](deployment/deploy-flow.md)
- **"Where are secrets managed?"** → [`deployment/external-secrets.md`](deployment/external-secrets.md)
- **"How are pipeline stages executed?"** → [`architecture/stage-execution-modes.md`](architecture/stage-execution-modes.md)
- **"What are the quality gates? Why no metrics stack?"** → [`local-dev/dev-loop.md`](local-dev/dev-loop.md)

## Index

### Onboarding
- [`onboarding.md`](onboarding.md) — role-aware entry point

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

Encode *why* alongside *how*. If a section only describes what the code does, a future reader could have derived it by reading the code — docs earn their keep by pinning invariants, recording precedent, and explaining non-obvious trade-offs. The docs that age well in this repo all share that trait ♪
