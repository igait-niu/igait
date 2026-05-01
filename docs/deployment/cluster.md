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

### Why SSH'ing the kubeconfig out is OK

This is the one time SSH is the *correct* tool: kubectl can't bootstrap
its own credentials. We pull `/etc/rancher/k3s/k3s.yaml` (k3s's generated
admin kubeconfig) over SSH, rewrite the loopback server URL to the
Tailscale hostname, and merge it into `~/.kube/config` locally.

Honest tradeoff: `k3s.yaml` embeds a client cert in the `system:masters`
group — that's full cluster-admin per dev. Acceptable for our small NIU
team behind Tailscale; if the team grows, swap this for k3s's
`--kube-apiserver-arg` + RBAC-scoped per-user kubeconfigs.

### Bootstrap recipe (works on clean *or* multi-cluster machines)

The flow imports `k3s.yaml` into a *temp* file, renames the context to
`igait-prod`, then merges into `~/.kube/config`. Existing contexts
(docker-desktop, minikube, a personal k3d, whatever) stay untouched and
remain switchable via `kubectl config use-context`.

```bash
# 1. Pull k3s's admin kubeconfig from the control plane.
#    k3s writes `server: https://127.0.0.1:6443` by default (the file
#    is meant to be read *on* the node), so we rewrite to the Tailscale
#    hostname. The api-server's TLS cert already has `DNS:ai-leads` in
#    its SAN list, so no cert regen needed.
ssh root@ai-leads 'cat /etc/rancher/k3s/k3s.yaml' \
  | sed 's|server: https://127.0.0.1:6443|server: https://ai-leads:6443|' \
  > /tmp/igait-prod.yaml

# 2. Rename the context inside the temp file to `igait-prod`
#    (k3s names it `default`).
KUBECONFIG=/tmp/igait-prod.yaml kubectl config rename-context default igait-prod

# 3. Merge into ~/.kube/config without clobbering existing entries.
mkdir -p ~/.kube
KUBECONFIG="$HOME/.kube/config:/tmp/igait-prod.yaml" \
  kubectl config view --flatten > "$HOME/.kube/config.new"
mv "$HOME/.kube/config.new" "$HOME/.kube/config"
chmod 600 "$HOME/.kube/config"
rm /tmp/igait-prod.yaml

# 4. Activate it. Toggle back any time with `kubectl config use-context <name>`.
kubectl config use-context igait-prod
kubectl get nodes   # should show `ai-leads  Ready  control-plane`
```

### What `.envrc` does

The repo's `.envrc` runs two soft probes on `direnv reload`:

1. TCP connect to `ai-leads:6443` — warns if unreachable (likely Tailscale).
2. Compares `kubectl config current-context` to `igait-prod` — nudges if
   you're on a different context (docker-desktop, minikube, etc.) so you
   don't misfire a kubectl against the wrong cluster.

Neither mutates `~/.kube/config`. Context switching stays your call.

## What lives on the cluster

- **`igait` namespace** — backend Deployment + five long-lived stage
  Deployments under `infra/k8s/stages/` (one per pipeline stage), each
  polling Firebase RTDB queues directly. The K8s-Job orchestrator that
  used to spawn ephemeral per-job pods is gone.
- **`igait-secrets`** (K8s Secret, `igait` ns) — shared runtime config
  consumed by `secretKeyRef` in the backend Deployment and `envFrom` in
  every stage Deployment. Materialized by External Secrets Operator from AWS SSM
  Parameter Store (`/igait/prod/env/*`); do not edit in-cluster directly
  — edit the SSM param and ESO picks it up within ~5m. See
  `docs/deployment/external-secrets.md` for the full flow and rotation
  recipe.
- **`gcp-key`** (K8s Secret, `igait` ns) — the GCP service-account JSON,
  mounted as a file volume. Also ESO-managed from
  `/igait/prod/gcp-key/key.json`.
- **`operator-oauth`** (K8s Secret, `tailscale` ns) — Tailscale Operator's
  OAuth client creds, ESO-managed from `/igait/prod/tailscale/*`.
- **ArgoCD** — deploys the `infra/k8s/` folder from the monorepo. Pushing
  to `main` triggers a sync; you generally don't `kubectl apply` YAML
  directly unless doing an emergency override.
- **cloudflared** — ingress. External traffic to `igaitapp.com` tunnels
  through.

## Common ops recipes (all kubectl, all from your laptop)

### Watch a rollout

```bash
kubectl rollout restart deployment/backend -n igait
kubectl rollout status deployment/backend -n igait --timeout=3m
kubectl logs -n igait -l app=backend --tail=50 --prefix=true
```

### Tail a stage's logs

Stages run as long-lived Deployments (one per stage). To watch what a stage
is doing right now:

```bash
kubectl -n igait get deploy
kubectl -n igait logs deployment/<stage> --tail=200 --follow
# <stage> ∈ {media-conversion, pose-estimation, cycle-detection, prediction, finalize}
```

If a stage looks wedged on a specific job, inspect its claim in the Firebase
RTDB Console under `queues/<stage>/<job_id>` — `claimed_at` older than 5
minutes means the heartbeat died and the next worker will re-claim shortly.
Per-job timeouts are enforced in code at
`apps/shared/src/microservice/worker.rs::process_one_job` (matching the old
`activeDeadlineSeconds` budgets the K8s-Job spec used to set).

### One-shot post-cutover cleanup

When the K8s-Job orchestrator was retired, the manual purge of any orphaned
`Job` objects was:

```bash
kubectl -n igait delete jobs -l app=igait-pipeline
```

Safe to re-run; if the label has nothing matching it, the command is a no-op.

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
