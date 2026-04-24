# Vaultwarden — iGait

> **Deprecated at the runtime-config layer.** Prod secrets now live in
> AWS SSM Parameter Store and flow into the cluster via External Secrets
> Operator. See **[`docs/deployment/external-secrets.md`](../deployment/external-secrets.md)**
> for the authoritative reference.

Vaultwarden still runs at `https://vault.igaitapp.com` for team password
sharing and as a historical archive (the `igait/dev-env` item is a
pre-migration snapshot of values that were copied into SSM). Editing a
field there no longer affects anything running in K8s.

This file persists so old links still resolve.
