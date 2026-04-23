# Production Cluster — DGX Spark

The iGait prod cluster is a k3s instance on the `ai-leads` DGX Spark box,
reachable over Tailscale.

## Access model — kubectl first, SSH is a cardinal sin

Everything you can do via `kubectl` — reads, restarts, secrets, logs — you
**must** do via `kubectl`. SSH is reserved for the narrow set of operations
that modify cluster state at a layer kubectl can't touch (cluster lifecycle,
node-level configuration). Treat `ssh root@ai-leads` like `rm -rf`: if your
reflex is to reach for it, stop and check whether kubectl solves it first.

- **Allowed via kubectl** (from your laptop): get/describe/logs/exec,
  rollout restarts, secret edits, ArgoCD patches, port-forwards, deletes
  targeted at application resources (pods, jobs, deployments).
- **Requires SSH** (rare): `k3s` service restarts, node-level filesystem
  cleanup (e.g., stale `/root/.docker/config.json`), installing/upgrading
  k3s, touching `/etc/rancher/k3s/*`, `systemctl` anything, inspecting
  containerd state directly via `crictl`.

If you're unsure which category a task falls into, it's kubectl.

## Bootstrap — wiring up local kubectl

Prerequisite: you're on Tailscale and can reach `ai-leads`. `tailscale status`
should list it with a `100.x.x.x` IP and no `offline` tag. If not, fix
Tailscale first — `/onboard` has the setup steps.

Pick the recipe that matches your machine's existing kubeconfig state.

### You have no existing `~/.kube/config` (clean machine)

```bash
mkdir -p ~/.kube
ssh root@ai-leads 'cat /etc/rancher/k3s/k3s.yaml' \
  | sed 's|server: https://127.0.0.1:6443|server: https://ai-leads:6443|' \
  > ~/.kube/config
chmod 600 ~/.kube/config
kubectl config rename-context default igait-prod
kubectl config use-context igait-prod
kubectl get nodes   # should show `ai-leads  Ready  control-plane`
```

The sed rewrite matters: k3s writes `server: https://127.0.0.1:6443` by
default (the file is meant to be read *on* the node). The api-server's TLS
cert already has `DNS:ai-leads` in its SAN list, so no cert regen needed.

### You already manage other clusters (merge into existing config)

Don't overwrite `~/.kube/config` — merge. Kubeconfig merging is first-class:

```bash
ssh root@ai-leads 'cat /etc/rancher/k3s/k3s.yaml' \
  | sed 's|server: https://127.0.0.1:6443|server: https://ai-leads:6443|' \
  > /tmp/igait-prod.yaml
# Give the context a unique name so it can't collide with anything you have
KUBECONFIG=/tmp/igait-prod.yaml kubectl config rename-context default igait-prod

# Merge, then atomically replace
KUBECONFIG="$HOME/.kube/config:/tmp/igait-prod.yaml" \
  kubectl config view --flatten > "$HOME/.kube/config.new"
mv "$HOME/.kube/config.new" "$HOME/.kube/config"
chmod 600 "$HOME/.kube/config"
rm /tmp/igait-prod.yaml

kubectl config use-context igait-prod
kubectl get nodes
```

Both recipes land you at the same state: `igait-prod` is a named context
in `~/.kube/config`, and `kubectl` with no flags talks to the prod cluster.

### What `.envrc` does

The repo's `.envrc` runs two soft probes on `direnv reload`:

1. TCP connect to `ai-leads:6443` — warns if unreachable (likely Tailscale).
2. Compares `kubectl config current-context` to `igait-prod` — nudges if
   you're on a different context (docker-desktop, minikube, etc.) so you
   don't misfire a kubectl against the wrong cluster.

Neither mutates `~/.kube/config`. Context switching stays your call.

## What lives on the cluster

- **`igait` namespace** — backend Deployment + per-job stage Jobs (spawned
  dynamically by the orchestrator). Stage Jobs appear on-demand; don't
  expect to see long-lived stage pods.
- **`igait-secrets`** (K8s Secret, `igait` ns) — shared runtime config
  consumed by `envFrom: secretRef` in both the backend Deployment and every
  stage Job. Dev mirror lives in Vaultwarden as `igait/dev-env`. Keep them
  in sync — see `docs/environment/vaultwarden.md`.
- **ArgoCD** — deploys the `infra/k8s/` folder from the monorepo. Pushing
  to `master` triggers a sync; you generally don't `kubectl apply` YAML
  directly unless doing an emergency override.
- **cloudflared** — ingress. External traffic to `igaitapp.com` tunnels
  through.
- **GPU** — available to stage Jobs that request it (pose estimation,
  prediction).

## Common ops recipes (all kubectl, all from your laptop)

### Watch a rollout

```bash
kubectl rollout restart deployment/backend -n igait
kubectl rollout status deployment/backend -n igait --timeout=3m
kubectl logs -n igait -l app=backend --tail=50 --prefix=true
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

ArgoCD `Application` resources store `spec.source.path` in the cluster,
not git. When a monorepo restructure moves `infra/k8s/**` (or the ArgoCD
apps folder itself), the deployed Applications still point at the *old*
path even after git is updated — you'll see `ComparisonError: app path
does not exist` in the UI.

The `app-of-apps` pattern *cannot* self-heal this: if its current `path`
points nowhere, it can't read the new manifest for itself.

**Fix:** patch every affected Application once via kubectl, then let
selfHeal take over again.

```bash
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
```

Survey afterwards — every Git-sourced app should be `Synced`:

```bash
kubectl get application -n argocd -o jsonpath='{range .items[*]}{.metadata.name}{"\t"}{.spec.source.path}{"\t"}{.status.sync.status}{"\t"}{.status.health.status}{"\n"}{end}'
```

Any future path restructure PR should include these patch commands in its
merge checklist — git alone won't update the live cluster.

## SSH-only recipes (the rare, cluster-modifying ones)

### Don't `docker login` on the cluster node

Kubelet's credential resolution for image pulls reads, in order: the pod's
`imagePullSecrets`, the service account's `imagePullSecrets`, then
`/root/.docker/config.json` on the node (because kubelet runs as root).

If someone `docker login ghcr.io`s on `ai-leads` for a one-off pull, the
resulting `~/.docker/config.json` becomes a silent node-wide credential
source for *every* subsequent image pull. When its token expires (GitHub
`ghs_*` session tokens last hours; even PATs rotate), kubelet keeps
sending the expired credential — and ghcr returns **403** to invalid
creds instead of falling back to anonymous. Public packages suddenly fail
to pull with `failed to fetch oauth token: 403 Forbidden`.

**Don't `docker login` on the node.** If you need to pull a private image:

1. Prefer making the ghcr package public (we do this for every app/stage
   image — they're binaries, not source).
2. Otherwise, add an `imagePullSecrets` entry to the Deployment or its
   ServiceAccount. There's already an unused `ghcr-pull-secret` in the
   `igait` namespace if you need it.

If someone already did `docker login`, clean it up (this is one of the
few legitimate SSH uses — it's a node-level filesystem mutation, no
kubectl equivalent):

```bash
ssh root@ai-leads rm /root/.docker/config.json
kubectl -n igait delete pods -l app=<affected>   # kubectl, not ssh
```

## Access scope for Claude / agents

kubectl access (from an agent's shell, now that the local kubeconfig is
wired) is authorized for:
- Read-only inspection of the `igait` namespace (get/describe/logs).
- Rollout-restarting `backend` and reading pod logs.
- Editing `igait-secrets` when explicitly asked.
- ArgoCD `Application` patches when explicitly asked.

SSH access — because of the cardinal-sin framing — requires explicit
confirmation from @hiibolt for every invocation, even when the task falls
under "SSH-only recipes" above. The agent should propose the command and
wait for green-light, not execute blindly.

Anything beyond that scope — cluster-wide changes, destructive deletions,
node-level commands, other namespaces — should be explicitly confirmed by
@hiibolt first regardless of which tool (kubectl or ssh) would be used.
