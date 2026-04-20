# Production Cluster — DGX Spark

## Access

The iGait prod cluster is a k3s instance on the `ai-leads` DGX Spark box.

```bash
ssh root@ai-leads
```

`kubectl` is pre-configured on the host — no kubeconfig juggling needed. From
there, every namespace is directly reachable.

```bash
kubectl version -o json | jq -r .serverVersion.gitVersion    # e.g. v1.34.6+k3s1
kubectl get ns
kubectl -n igait get pods
```

## What lives on the cluster

- **`igait` namespace** — backend Deployment + per-job stage Jobs (spawned
  dynamically by the orchestrator). Stage Jobs appear on-demand; don't expect
  to see long-lived stage pods.
- **`igait-secrets`** (K8s Secret, `igait` ns) — shared runtime config consumed
  by `envFrom: secretRef` in both the backend Deployment and every stage Job.
  Dev mirror lives in Vaultwarden as `igait/dev-env`. Keep them in sync — see
  `wiki/environment/VAULTWARDEN.md`.
- **ArgoCD** — deploys the `igait-kubernetes-configs/` folder from the monorepo.
  Pushing to `master` triggers a sync; you generally don't `kubectl apply`
  YAML directly unless doing an emergency override.
- **cloudflared** — ingress. External traffic to `igaitapp.com` tunnels through.
- **GPU** — available to stage Jobs that request it (pose estimation, prediction).

## Common ops recipes

### Watch a rollout

```bash
kubectl rollout restart deployment/igait-backend -n igait
kubectl rollout status deployment/igait-backend -n igait --timeout=3m
kubectl logs -n igait -l app=igait-backend --tail=50 --prefix=true
```

### Tail live stage Jobs for a running pipeline

```bash
kubectl -n igait get jobs --sort-by=.metadata.creationTimestamp | tail -10
kubectl -n igait logs job/<job-name> --all-containers --follow
```

### Inspect the shared Secret

```bash
kubectl -n igait get secret igait-secrets -o jsonpath='{.data}' | jq 'keys'
```

(Values are base64 — decode on purpose, never casually.)

## Access scope for Claude / agents

SSH access is authorized for:
- Inspecting/editing `igait-secrets` in the `igait` namespace
- Rollout-restarting `igait-backend` and reading pod logs
- Running non-destructive `kubectl get` / `describe` / `logs` against the
  `igait` namespace

Anything beyond that scope — node-level commands, other namespaces, destructive
deletions, cluster-wide changes — should be explicitly confirmed by @hiibolt
first.
