<!-- Keep this stub minimal. Anything procedural belongs in docs/. -->

# backend

The iGait HTTP API — accepts uploads, enqueues pipeline work, and (in prod) orchestrates K8s Jobs across the 5 stages.

**Stack:** axum + tokio, Firebase admin SDK for auth, `aws-sdk-s3` + `aws-sdk-sesv2`, `kube` for the orchestrator.

For the dev loop, route surface, and orchestrator gating, see **[`docs/README.md#backend`](../../docs/README.md#backend)**.
