# Production Cluster — DGX Spark

## Access

The iGait prod cluster is a k3s instance on the `ai-leads` DGX Spark box.

```bash
ssh root@ai-leads
```

In order for this to work, you may need to ask the user to run that command interactively in a *separate* shell. `! ssh root@ai-leads` won't work, since auth via Tailscale in the browser is required.

If users don't have access to Tailscale, then they need to prioritize that first. Feel free to refer to the `/onboard` command for more information on how the user should set this up!

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
  `docs/environment/vaultwarden.md`.
- **ArgoCD** — deploys the `infra/k8s/` folder from the monorepo.
  Pushing to `master` triggers a sync; you generally don't `kubectl apply`
  YAML directly unless doing an emergency override.
- **cloudflared** — ingress. External traffic to `igaitapp.com` tunnels through.
- **GPU** — available to stage Jobs that request it (pose estimation, prediction).

## Common ops recipes

### Watch a rollout

```bash
kubectl rollout restart deployment/apps/backend -n igait
kubectl rollout status deployment/apps/backend -n igait --timeout=3m
kubectl logs -n igait -l app=apps/backend --tail=50 --prefix=true
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

### Re-point ArgoCD apps after a path restructure

ArgoCD `Application` resources store `spec.source.path` in the cluster, not
git. When a monorepo restructure moves `infra/k8s/**` (or the ArgoCD apps
folder itself), the deployed Applications still point at the *old* path even
after git is updated — you'll see `ComparisonError: app path does not exist`
in the UI.

The `app-of-apps` pattern *cannot* self-heal this: if its current `path`
points nowhere, it can't read the new manifest for itself.

**Fix:** patch every affected Application once via `kubectl`, then let
selfHeal take over again.

```bash
ssh root@ai-leads 'bash -s' <<'REMOTE'
patch() {
  kubectl -n argocd patch application "$1" --type=merge \
    -p "{\"spec\":{\"source\":{\"path\":\"$2\"}}}"
  kubectl -n argocd annotate application "$1" \
    argocd.argoproj.io/refresh=hard --overwrite
}
patch app-of-apps  infra/k8s/argocd/apps
patch cloudflared  infra/k8s/cloudflared
patch igait        infra/k8s
patch vaultwarden  infra/k8s/vaultwarden
REMOTE
```

Survey afterwards — every Git-sourced app should be `Synced`:

```bash
kubectl get application -n argocd -o jsonpath='{range .items[*]}{.metadata.name}{"\t"}{.spec.source.path}{"\t"}{.status.sync.status}{"\t"}{.status.health.status}{"\n"}{end}'
```

Any future path restructure PR should include these patch commands in its
merge checklist — git alone won't update the live cluster.

### Don't `docker login` on the cluster node

Kubelet's credential resolution for image pulls reads, in order: the pod's
`imagePullSecrets`, the service account's `imagePullSecrets`, then
`/root/.docker/config.json` on the node (because kubelet runs as root).

If someone `docker login ghcr.io`s on `ai-leads` for a one-off pull, the
resulting `~/.docker/config.json` becomes a silent node-wide credential
source for *every* subsequent image pull. When its token expires (GitHub
`ghs_*` session tokens last hours; even PATs rotate), kubelet keeps sending
the expired credential — and ghcr returns **403** to invalid creds instead
of falling back to anonymous. Public packages suddenly fail to pull with
`failed to fetch oauth token: 403 Forbidden`.

**Don't `docker login` on the node.** If you need to pull a private image:

1. Prefer making the ghcr package public (we do this for every app/stage
   image — they're binaries, not source).
2. Otherwise, add an `imagePullSecrets` entry to the Deployment or its
   ServiceAccount. There's already an unused `ghcr-pull-secret` in the
   `igait` namespace if you need it.

If someone already did `docker login`, clean it up:

```bash
ssh root@ai-leads rm /root/.docker/config.json
ssh root@ai-leads kubectl -n igait delete pods -l app=<affected>
```

## Access scope for Claude / agents

SSH access is authorized for:
- Inspecting/editing `igait-secrets` in the `igait` namespace
- Rollout-restarting `apps/backend` and reading pod logs
- Running non-destructive `kubectl get` / `describe` / `logs` against the
  `igait` namespace

Anything beyond that scope — node-level commands, other namespaces, destructive
deletions, cluster-wide changes — should be explicitly confirmed by @hiibolt
first.
