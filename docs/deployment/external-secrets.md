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
        "ssm:GetParametersByPath"
      ],
      "Resource": "arn:aws:ssm:us-east-2:*:parameter/igait/prod/*"
    },
    {
      "Sid": "DescribeAllParametersMetadata",
      "Effect": "Allow",
      "Action": "ssm:DescribeParameters",
      "Resource": "*"
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

Three things to note about this policy:

1. **`ssm:DescribeParameters` must be unscoped (`Resource: "*"`).** It is a
   region-wide listing API; AWS does not support resource-level IAM for it.
   Used by ESO's `find.name.regexp` matcher to enumerate params. The only
   leakage is param *names* in the account — values stay locked to
   `/igait/prod/*` by the first statement.
2. **`kms:Decrypt` `Resource: "*"` is safe** because the `kms:ViaService`
   condition scopes decryption to keys invoked via SSM only — a leaked key
   cannot decrypt arbitrary KMS blobs.
3. **`GetParametersByPath` is resource-scopeable** (unlike `DescribeParameters`),
   so if you later migrate the ExternalSecret from `name.regexp` to `path`-based
   discovery, you can drop the `DescribeAllParametersMetadata` statement
   entirely.

## Adoption of pre-existing Secrets

ESO v0.10+ with `creationPolicy: Owner` will **adopt** an existing K8s Secret
if it has no conflicting `ownerReferences` — no manual cutover needed. When
the initial migration happened, `igait-secrets` and `gcp-key` already existed
as manually-applied Secrets; ESO simply took ownership on its first successful
reconcile and began keeping them in sync with SSM. No `kubectl delete`, no
pod restart. Worth knowing for future clusters — you do not need to pre-clean
the target namespace.

If ESO ever reports `SecretAlreadyExists` / `NotManaged` instead of adopting,
it means the existing Secret has an `ownerReferences` entry pointing at some
*other* controller. In that case, remove the stale owner reference (or delete
the Secret outright) and ESO will reconcile into the vacated slot on the next
cycle.

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

### OAuth client scopes

Tailscale OAuth clients attach tags *to individual scopes* — tags are not a
separate top-level list. Per Tailscale's Kubernetes Operator docs, the client
needs these three scopes, all **write**:

- **Devices Core**
- **Auth Keys**
- **Services**

Attach `tag:k8s-igait-operator` to each of the three scopes. The workload tag
`tag:k8s-igait` is **not** needed here — it is minted by the Operator device
itself via the ACL's `tagOwners` delegation (`tag:k8s-igait-operator` owns
`tag:k8s-igait`), not by the OAuth client.

**Common failure mode:** if the scopes do not include `tag:k8s-igait-operator`,
auth-key minting fails with `Status: 400, Message: "requested tags
[tag:k8s-igait-operator] are invalid or not permitted"` and the Operator
crashloops on `creating operator authkey`. Fix is UI-only — edit the OAuth
client, add the tag to each scope, save. Next pod start succeeds.

### Rotating Tailscale OAuth creds

Same pattern as any other SSM param — edit the value under
`/igait/prod/tailscale/*`, wait ≤5m, restart the Operator:

```bash
kubectl -n tailscale rollout restart deployment/operator
```
