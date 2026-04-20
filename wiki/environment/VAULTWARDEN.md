# Vaultwarden — iGait Secrets

## Purpose

Vaultwarden is iGait's self-hosted, Bitwarden-compatible secret store. It replaced
an older OneDrive-based `.env` distribution flow. One item — `igait/dev-env` —
holds every environment variable required to run the dev stack locally. Vaultwarden
is the source of truth; nobody emails, Slacks, or commits secrets.

- **Instance:** <https://vault.igaitapp.com>
- **Org / collection:** `igait-niu`
- **Dev-env item:** `igait/dev-env` (type: Secure Note; values live in `fields[]`)
- **Access is granted per-engineer** by @hiibolt via the Vaultwarden admin UI.

## First-Time Setup

The `bw` CLI and `jq` are provided by `flake.nix` — you get them automatically
inside the Nix dev shell.

```bash
bw config server https://vault.igaitapp.com
bw login                                  # interactive: email + password + 2FA
export BW_SESSION=$(bw unlock --raw)      # add to your shell rc for persistence
direnv allow                              # if not already allowed
```

From then on, every new shell runs `.envrc`, which fetches the current
`igait/dev-env` from Vaultwarden and exports every field as an env var.

## How Environment Loading Works

`.envrc` runs on `cd` into the repo (via direnv). The flow:

1. Verify `bw` and `jq` are on `PATH` (from the Nix dev shell).
2. Check `bw status`:
   - `unauthenticated` → prompt first-time login
   - `locked` → prompt `bw unlock` + `BW_SESSION` export
   - `unlocked` → continue
3. `bw sync --quiet` to pull the freshest Vaultwarden state.
4. `bw get item igait/dev-env` → JSON blob.
5. Iterate `.fields[]` and `export name=value` for each entry.
6. Print `iGait env loaded from Vaultwarden.`

Rotation: when another engineer updates a field in Vaultwarden, you pick up the
new value on the next shell reload (`direnv reload`).

### What belongs in `igait/dev-env`

**Yes** — shared secrets and shared environment-scoped config that every engineer
needs identical. Examples: `FIREBASE_ACCESS_KEY`, `FIREBASE_RTDB_URL`,
`FIREBASE_PROJECT_ID`, `AWS_ACCESS_KEY_ID`, `IGAIT_S3_BUCKET`,
`SES_FROM_ADDRESS`.

**No** — per-developer preferences or per-service runtime config:

- `PORT` — each service picks its own; set in `docker-compose.yml` (Phase 2) or K8s
  Deployment `env:` blocks, not the shared bundle.
- `RUST_LOG` — tracing-subscriber convention, belongs in your shell rc or
  `.envrc.local`.
- Anything personal (editor settings, aliases).

## Adding or Editing Items via CLI

The CLI flow avoids the web UI round-trip and is idempotent.

### Inspect the dev-env item

```bash
bw sync
ITEM_ID=$(bw get item igait/dev-env | jq -r .id)
bw get item "$ITEM_ID" | jq '.fields[] | {name, value}'
```

### Add or update a field

```bash
bw get item "$ITEM_ID" \
  | jq '.fields = (
      (.fields // [])
      | map(select(.name != "NEW_VAR_NAME"))       # drop existing if present
      + [{"name":"NEW_VAR_NAME","value":"the-value","type":0}]
    )' \
  | bw encode \
  | bw edit item "$ITEM_ID"

bw sync                                              # push to server
direnv reload                                        # re-export into current shell
```

`type: 0` is a plain (visible) field; `type: 1` is hidden (masked in UI). Prefer
`1` for secrets.

### Remove a field

```bash
bw get item "$ITEM_ID" \
  | jq '.fields |= map(select(.name != "VAR_TO_DROP"))' \
  | bw encode \
  | bw edit item "$ITEM_ID"

bw sync
```

### Rename a field

```bash
bw get item "$ITEM_ID" \
  | jq '.fields |= map(if .name == "OLD_NAME" then .name = "NEW_NAME" else . end)' \
  | bw encode \
  | bw edit item "$ITEM_ID"

bw sync
```

### Create a brand-new item

Rare for dev-env, but useful for per-service secrets:

```bash
bw get template item \
  | jq '.name = "my-new-item" | .notes = "purpose: ..." | .fields = [
      {"name":"FOO","value":"bar","type":1}
    ]' \
  | bw encode \
  | bw create item
```

## Production Secret Flow

Vaultwarden is the **dev** source of truth. Production secrets live in the
`igait-secrets` Kubernetes Secret in the `igait` namespace — populated out-of-band
by whoever owns the cluster. The backend Deployment and every orchestrator-spawned
stage Job consume it via `envFrom: secretRef: igait-secrets`.

When `igait/dev-env` gains a new required key (any new `.context("... must be
set")?` read added to the Rust code), you **must** also update the cluster
Secret before the next deploy, or pods crashloop.

```bash
# Add a single key
kubectl create secret generic igait-secrets \
  --namespace igait \
  --from-literal=NEW_VAR=the-value \
  --dry-run=client -o yaml \
  | kubectl apply -f -

# Or patch multiple via JSON Patch (values must be base64)
kubectl patch secret igait-secrets -n igait --type=json -p='[
  {"op":"add","path":"/data/NEW_VAR","value":"'"$(echo -n the-value | base64 -w0)"'"},
  {"op":"remove","path":"/data/OLD_VAR"}
]'
```

## Migrating Keys in Prod (Add→Restart→Verify→Delete)

When Phase-1-style work adds, renames, or removes keys in `igait-secrets`, apply
the changes in a specific order to avoid crashloops. The principle: **the
Secret must be a superset of what every running pod needs at every moment.**

1. **Add** all new keys first. If you're renaming, copy the old value into the
   new key — don't move it.
2. **Rollout-restart** `igait-backend` (and any other consumers) so pods pick
   up the new keys.
3. **Verify** pods reach `Running 1/1` and logs show `Starting iGait backend
   initialization...` with no `Missing required env vars` error.
4. **Delete** the obsolete keys only after step 3 succeeds.

If you delete first, the window between "delete" and "next rollout" is a
crashloop waiting to happen — pods restarting for any reason (node drain, OOM,
ArgoCD sync) will fail `check_env`.

```bash
# ssh root@ai-leads first

# Step 1 — add new keys. printf %s avoids trailing newline in base64.
kubectl patch secret igait-secrets -n igait --type=json -p='[
  {"op":"add","path":"/data/NEW_VAR","value":"'"$(printf %s "$NEW_VALUE" | base64 -w0)"'"}
]'

# For renames — copy the existing base64 value server-side (no re-encoding risk):
OLD_B64=$(kubectl -n igait get secret igait-secrets -o jsonpath='{.data.OLD_NAME}')
kubectl patch secret igait-secrets -n igait --type=json -p="[
  {\"op\":\"add\",\"path\":\"/data/NEW_NAME\",\"value\":\"$OLD_B64\"}
]"

# Step 2 — rollout
kubectl rollout restart deployment/igait-backend -n igait
kubectl rollout status deployment/igait-backend -n igait --timeout=3m

# Step 3 — verify
kubectl logs -n igait -l app=igait-backend --tail=30 | grep -E 'Missing|Starting'

# Step 4 — drop obsolete
kubectl patch secret igait-secrets -n igait --type=json -p='[
  {"op":"remove","path":"/data/OLD_NAME"}
]'
```

Stage Jobs don't need an explicit restart — they re-read the Secret on every
new Job spawn via `envFrom`. Only long-lived Deployments (backend) need the
rollout step.

## Troubleshooting

| Symptom | Fix |
|---------|-----|
| `ERROR: Not logged in to Vaultwarden` | `bw config server ...` then `bw login` |
| `ERROR: Vaultwarden is locked` | `export BW_SESSION=$(bw unlock --raw)` |
| `ERROR: Could not fetch 'igait/dev-env'` | Ask @hiibolt to grant access to the `igait-niu` collection |
| Fresh key in Vaultwarden not in shell | `direnv reload` (or `bw sync && direnv reload` if stale) |
| Pod crashloops after deploy with "VAR must be set" | The cluster `igait-secrets` is missing a key `igait/dev-env` has — mirror it in |
