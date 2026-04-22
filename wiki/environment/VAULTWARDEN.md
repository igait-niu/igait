# Vaultwarden — iGait Secrets

## What a model needs to know

- **Source of truth for dev secrets**: Vaultwarden item `igait/dev-env` (org `igait-niu`) at <https://vault.igaitapp.com>. Fields live in `.fields[]`.
- **Materialisation is a one-shot command**: `/igait-environment` writes `.env` (at repo root, auto-loaded by `docker compose`) and `credentials/gcp-key.json` (mode 600). There is no direnv, no watcher — the command is idempotent and re-run after rotations.
- **`GCP_KEY_JSON` is not an env var.** It's a field on the item whose value is the full GCP service-account JSON. The slash command writes it to `credentials/gcp-key.json` and skips it in `.env`.
- **The emulator ignores the GCP key** but the Firebase SDK init still opens the file. Do not remove the bind-mount from `docker-compose.yml` under the assumption that "the emulator doesn't need auth."

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

Only store things that every engineer needs identical. Per-developer preferences (editor settings, personal `RUST_LOG` overrides) and per-service runtime ports belong in `docker-compose.yml` / K8s manifests, not the shared bundle.
