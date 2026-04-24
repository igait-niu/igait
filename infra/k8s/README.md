# infra/k8s

Kustomize manifests for iGait's prod K8s cluster. Deployed continuously
by ArgoCD from this folder — no manual `kubectl apply -k` under normal
conditions.

## Layout

Flat — no `base/` + `overlays/` split, one prod target.

```
infra/k8s/
├── kustomization.yaml         # root: bundles namespace + backend + frontend
├── igait-namespace/           # the `igait` namespace itself
├── backend/                   # Deployment, Service, RBAC, ExternalSecret
├── frontend/                  # Deployment, Service
├── argocd/apps/               # ArgoCD Application CRs (app-of-apps pattern)
├── cloudflared/               # in-cluster ingress via Cloudflare tunnel
├── external-secrets/          # External Secrets Operator install
├── tailscale-operator/        # Tailscale Operator install
└── vaultwarden/               # team password vault (off the runtime-secret path)
```

## App-of-apps

ArgoCD bootstraps everything from a single `app-of-apps` Application at
`argocd/apps/app-of-apps.yaml`, which in turn creates one Application
per child — `igait` (the root kustomization above), `cloudflared`,
`vaultwarden`, `tailscale-operator`, `external-secrets`, and
`nvidia-device-plugin`.

Two pairs run in waves because their manifests depend on secrets ESO
materializes:

- `external-secrets-config` (wave `-1`) → `external-secrets` (wave `0`)
- `tailscale-operator-config` (wave `-1`) → `tailscale-operator` (wave `0`)

All Applications have `selfHeal: true` and `prune: true` — drift is
reverted on the next reconcile, removed resources are deleted.

## How code reaches this folder

Not by hand. Each `build-*.yml` workflow on push to `main` sed-replaces
the image tag in the matching `*/deployment.yaml` and commits with a
`deploy:` prefix; ArgoCD picks it up within seconds. See
[`docs/deployment/deploy-flow.md`](../../docs/deployment/deploy-flow.md)
for the full pipeline.

CI has `paths-ignore: infra/k8s/**` so those bot commits don't retrigger
CI — deliberate.

## Related reading

- **Cluster access + kubectl recipes:** [`docs/deployment/cluster.md`](../../docs/deployment/cluster.md)
- **Secrets (SSM → ESO → K8s):** [`docs/deployment/external-secrets.md`](../../docs/deployment/external-secrets.md)
- **Deploy flow end-to-end:** [`docs/deployment/deploy-flow.md`](../../docs/deployment/deploy-flow.md)

## Editing

Change manifests in a PR, wait for ArgoCD to sync on merge. Out-of-band
`kubectl apply` is reserved for emergencies — `selfHeal` will revert
drift on the next reconcile anyway.
