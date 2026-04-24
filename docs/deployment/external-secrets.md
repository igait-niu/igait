# External Secrets Operator + AWS SSM Parameter Store

> Replaces the manual Vaultwarden ↔ `igait-secrets` dual-update flow described
> in `docs/environment/vaultwarden.md`. Prod secrets now live in AWS SSM
> Parameter Store under `/igait/prod/*`; ESO materializes them into K8s Secrets.

## Architecture

```
AWS SSM Parameter Store          Kubernetes cluster
─────────────────────────        ───────────────────────────────────────────
/igait/prod/env/*         ──┐    ┌── Secret/igait-secrets  (ns: igait)
                            ├──▶ │
/igait/prod/gcp-key/*     ──┘    └── Secret/gcp-key        (ns: igait)
                                        ▲
                                        │ ExternalSecret CRs + ClusterSecretStore
                                        │ reconciled by external-secrets-operator
                                        │ every 5m
                             bootstrap: Secret/aws-ssm-creds  (ns: external-secrets)
                                        │ single static IAM user, read-only on
                                        │ arn:aws:ssm:us-east-2:*:parameter/igait/prod/*
```

## Bootstrap (one-time, per cluster)

The `aws-ssm-creds` Secret is the one credential **not** managed by ESO — it's
what ESO uses to talk to AWS. Create it manually:

```bash
# 1. In AWS IAM, create user `igait-eso-ssm-reader` with programmatic access
#    and the policy in `Minimum IAM policy` below. Save the access key.

# 2. Create the bootstrap Secret in-cluster:
kubectl create namespace external-secrets || true
kubectl -n external-secrets create secret generic aws-ssm-creds \
  --from-literal=access-key-id=AKIA... \
  --from-literal=secret-access-key=...
```

Rotate every 90 days (track in team calendar; automation pending).

## Minimum IAM policy

Attach to `igait-eso-ssm-reader`:

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Sid": "ReadIgaitProdParameters",
      "Effect": "Allow",
      "Action": [
        "ssm:GetParameter",
        "ssm:GetParameters",
        "ssm:GetParametersByPath",
        "ssm:DescribeParameters"
      ],
      "Resource": "arn:aws:ssm:us-east-2:*:parameter/igait/prod/*"
    },
    {
      "Sid": "DecryptSecureStrings",
      "Effect": "Allow",
      "Action": "kms:Decrypt",
      "Resource": "*",
      "Condition": {
        "StringEquals": {
          "kms:ViaService": "ssm.us-east-2.amazonaws.com"
        }
      }
    }
  ]
}
```

The `kms:Decrypt` condition scopes decryption to keys invoked via SSM — even
though the `Resource: "*"` looks loose, the condition makes it safe.

## Cutover playbook (initial migration from manually-applied Secrets)

Run **after** the PR is merged and ArgoCD has synced, and **after** all 14 SSM
params are populated:

```bash
# 1. Verify ESO pods are Running.
kubectl -n external-secrets get pods

# 2. Verify ClusterSecretStore is Ready.
kubectl get clustersecretstore aws-ssm \
  -o jsonpath='{.status.conditions[0].type}={.status.conditions[0].status}'
# Expect: Ready=True

# 3. Check ExternalSecret status — will say SecretSyncedError because the
#    existing manually-applied Secret blocks `creationPolicy: Owner` takeover.
kubectl -n igait get externalsecret

# 4. Back up current Secrets (paranoia — 10 seconds of insurance):
kubectl -n igait get secret igait-secrets -o yaml > /tmp/igait-secrets.bak.yaml
kubectl -n igait get secret gcp-key       -o yaml > /tmp/gcp-key.bak.yaml

# 5. Delete the manually-applied Secrets. ESO recreates from SSM within seconds.
kubectl -n igait delete secret igait-secrets gcp-key

# 6. Confirm ESO created fresh Secrets (look for owner reference to ExternalSecret):
kubectl -n igait get secret igait-secrets -o yaml | grep -A2 ownerReferences

# 7. Restart backend to pick up the new Secret values (envFrom would hot-reload,
#    but we use individual `secretKeyRef` which only re-read on pod start):
kubectl -n igait rollout restart deployment/backend
kubectl -n igait rollout status deployment/backend
```

Rollback (if step 5 goes sideways):

```bash
kubectl -n igait apply -f /tmp/igait-secrets.bak.yaml
kubectl -n igait apply -f /tmp/gcp-key.bak.yaml
```

## Rotation verification

Exercise the full loop end-to-end — should be done once after initial cutover:

```bash
# 1. In AWS console, edit /igait/prod/env/OPENAI_ASSISTANT_ID to a dummy value.
# 2. Wait up to 5m (refreshInterval).
# 3. Confirm the in-cluster Secret reflects the new value:
kubectl -n igait get secret igait-secrets \
  -o jsonpath='{.data.OPENAI_ASSISTANT_ID}' | base64 -d ; echo
# 4. Revert the SSM param to the real value, repeat step 3 to confirm.
```

## Param reference

14 parameters, all `SecureString` + Intelligent-Tiering + AWS-managed KMS:

| SSM path | Target K8s Secret key |
|---|---|
| `/igait/prod/env/GOOGLE_APPLICATION_CREDENTIALS` | `igait-secrets.GOOGLE_APPLICATION_CREDENTIALS` |
| `/igait/prod/env/FIREBASE_PROJECT_ID` | `igait-secrets.FIREBASE_PROJECT_ID` |
| `/igait/prod/env/FIREBASE_ACCESS_KEY` | `igait-secrets.FIREBASE_ACCESS_KEY` |
| `/igait/prod/env/FIREBASE_RTDB_URL` | `igait-secrets.FIREBASE_RTDB_URL` |
| `/igait/prod/env/IGAIT_S3_BUCKET` | `igait-secrets.IGAIT_S3_BUCKET` |
| `/igait/prod/env/SES_FROM_ADDRESS` | `igait-secrets.SES_FROM_ADDRESS` |
| `/igait/prod/env/SES_FROM_IDENTITY_ARN` | `igait-secrets.SES_FROM_IDENTITY_ARN` |
| `/igait/prod/env/AWS_REGION` | `igait-secrets.AWS_REGION` |
| `/igait/prod/env/AWS_ACCESS_KEY_ID` | `igait-secrets.AWS_ACCESS_KEY_ID` |
| `/igait/prod/env/AWS_SECRET_ACCESS_KEY` | `igait-secrets.AWS_SECRET_ACCESS_KEY` |
| `/igait/prod/env/OPENAI_API_KEY` | `igait-secrets.OPENAI_API_KEY` |
| `/igait/prod/env/OPENAI_ASSISTANT_ID` | `igait-secrets.OPENAI_ASSISTANT_ID` |
| `/igait/prod/env/OPENAI_VECTOR_STORE_ID` | `igait-secrets.OPENAI_VECTOR_STORE_ID` |
| `/igait/prod/gcp-key/key.json` | `gcp-key.gcp-key.json` |
| `/igait/prod/tailscale/client_id` | `operator-oauth.client_id` (ns: `tailscale`) |
| `/igait/prod/tailscale/client_secret` | `operator-oauth.client_secret` (ns: `tailscale`) |

To add a new secret: put it in SSM under `/igait/prod/env/` and it'll appear
in `igait-secrets` on the next refresh — the `dataFrom: find:` selector in
`infra/k8s/backend/external-secrets.yaml` auto-discovers everything under
that path. Then add a `secretKeyRef` entry in the backend deployment to
surface it as an env var.

## Tailscale Operator

The Tailscale Operator is installed via Helm (ArgoCD App:
`tailscale-operator`) and its OAuth credentials are supplied by ESO, not
by the chart's values. The flow:

1. `tailscale-operator-config` App (sync-wave `-1`) creates the
   `tailscale` namespace and the `ExternalSecret` targeting
   `operator-oauth`.
2. ESO materializes `operator-oauth` in the `tailscale` namespace with
   keys `client_id` / `client_secret` from SSM.
3. `tailscale-operator` App (sync-wave `0`) installs the Helm chart.
   Chart values leave `oauth.clientId` / `oauth.clientSecret` empty —
   the chart's conditional Secret template skips creation, and the
   Operator Deployment picks up our ESO-managed `operator-oauth` via
   `secretKeyRef`.

### Tag hierarchy

Two tags, owned correctly:

- `tag:k8s-igait-operator` — the Operator pod registers itself with
  this tag. Self-owned so only devices already bearing it can mint new
  auth keys for it (bootstrap capability).
- `tag:k8s-igait` — assigned by the Operator to the workload devices
  it spawns (exposed Services, Ingresses). Owned by
  `tag:k8s-igait-operator`, so *only the Operator* can register a
  device with this tag.

### Tailnet ACL snippet

Required in `https://login.tailscale.com/admin/acls`:

```hujson
{
  "tagOwners": {
    // admins mint the Operator's own auth keys -- empty list means
    // nobody can mint it (Tailscale returns 400 "not permitted"):
    "tag:k8s-igait-operator": ["autogroup:admin"],
    // only the Operator spawns workload devices:
    "tag:k8s-igait": ["tag:k8s-igait-operator"],
    // ...existing tags unchanged
  },
  "acls": [
    // Let tailnet members reach workload devices:
    { "action": "accept", "src": ["autogroup:member"], "dst": ["tag:k8s-igait:*"] },
    // ...existing rules unchanged
  ],
}
```

### OAuth client scope footgun

The OAuth client used by the Operator **must** have `tag:k8s-igait-operator`
AND `tag:k8s-igait` in its allowed tags list (Tailscale admin →
OAuth clients → edit → Tags). Without this, auth-key minting fails
with an opaque 401 and the Operator logs "failed to create auth key"
with no clue why. Scope at the OAuth client should be:

- Devices: Core Read+Write
- Auth Keys: Write
- Tags: `tag:k8s-igait-operator`, `tag:k8s-igait`

### Rotating Tailscale OAuth creds

Same pattern as any other SSM param — edit the value under
`/igait/prod/tailscale/*`, wait ≤5m, restart the Operator:

```bash
kubectl -n tailscale rollout restart deployment/operator
```
