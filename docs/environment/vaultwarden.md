# Vaultwarden — iGait

> **No longer on the prod runtime-secret path.** As of #105 (2026-04-24),
> prod secrets live in **AWS SSM Parameter Store** and flow into the
> cluster via External Secrets Operator. See
> `docs/deployment/external-secrets.md` for the authoritative reference.
>
> Vaultwarden still runs in-cluster at `https://vault.igaitapp.com` for
> team password sharing and historical archival, but editing a field
> there no longer affects anything running in K8s.

## What a model needs to know

- **For prod runtime secrets:** use AWS SSM Parameter Store under
  `/igait/prod/*`. Edits propagate to the `igait-secrets` / `gcp-key` /
  `operator-oauth` K8s Secrets within ~5m via ESO. Full procedure in
  `docs/deployment/external-secrets.md`.
- **For personal / team credentials** (not consumed by any cluster
  workload): Vaultwarden is fine. Treat it like a shared password manager.
- **The `igait/dev-env` item still exists** in Vaultwarden as a historical
  snapshot of the values that were copied into SSM during migration. Do
  not edit it expecting cluster effects — the cluster no longer reads
  from Vaultwarden.

## Local dev

Local dev does not touch Vaultwarden at all. The hermetic `docker compose`
stack is env-var-free and uses hardcoded local surrogates for every cloud
dep. See `docs/local-dev/hermetic-stack.md`.

## Adding or editing a field via CLI

For non-prod-runtime items (team passwords etc.), the CLI flow is
idempotent and avoids the web UI. `type: 0` is plain, `type: 1` is hidden
— prefer `1` for secrets.

```bash
ITEM_ID=$(bw get item <item-name> | jq -r .id)

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

## Retirement status

Retiring the in-cluster Vaultwarden deployment itself is a separate
tracked task — not urgent. While it runs, the `igait/dev-env` item is a
useful backup source if anyone ever needs to cross-check what a prod
secret's value *was* at migration time. Treat the Vaultwarden data as
append-only archival until the deployment is explicitly retired.
