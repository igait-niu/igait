# Vaultwarden — iGait Secrets

> **Prod-only.** Local dev does not touch Vaultwarden at all; the hermetic
> `docker compose` stack is env-var-free and uses hardcoded local surrogates
> for every cloud dep. See `docs/local-dev/hermetic-stack.md`.
>
> **Planned migration:** Vaultwarden is slated to be replaced by AWS Parameter
> Store for prod-secret storage. Tracked separately; this page documents the
> current state.

## What a model needs to know

- **Source of truth for prod secrets**: Vaultwarden item `igait/dev-env` (org `igait-niu`) at <https://vault.igaitapp.com>. Fields live in `.fields[]`. Despite the historical name (`dev-env`), the values now flow into the K8s `igait-secrets` Secret that prod pods read from — they are production secrets, not dev ones.
- **There is no slash command or automation.** The Vaultwarden ↔ `igait-secrets` dual-update is manual and deliberate. After editing a field here, update the matching K8s Secret on the cluster (see `docs/deployment/`).
- **`GCP_KEY_JSON` is a legacy field.** It used to be written to disk for local dev. It's no longer needed there — `infra/docker/workspace/Dockerfile` bakes a stub into the runtime images and prod mounts the real key via a K8s Secret volume. Leave it in Vaultwarden as the source for prod rotations.

## Adding or editing a field via CLI

The CLI flow is idempotent and avoids the web UI. `type: 0` is plain, `type: 1` is hidden — prefer `1` for secrets.

```bash
ITEM_ID=$(bw get item igait/dev-env | jq -r .id)

bw get item "$ITEM_ID" \
  | jq '.fields = (
      (.fields // [])
      | map(select(.name != "NEW_VAR_NAME"))
      + [{"name":"NEW_VAR_NAME","value":"the-value","type":1}]
    )' \
  | bw encode \
  | bw edit item "$ITEM_ID"

bw sync
```

Removal: same pattern with `.fields |= map(select(.name != "VAR_TO_DROP"))`.

## Classification guidance

Only store things that prod needs and that every operator should see identical. Per-developer preferences and per-service runtime ports do not belong here — they belong in `infra/compose.yml` / K8s manifests.
