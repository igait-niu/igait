---
description: Materialise iGait dev environment (.env) from Vaultwarden. Run once per machine; re-run to resync after a secret rotation.
---

You are setting up — or resyncing — the iGait development environment for the user in this repo. Work through the steps below. Be concise: one short status line per step is enough. Only get chatty if something breaks and the user needs to intervene.

## Goal

Produce one artefact at repo root:

1. `./.env` — a plain `KEY=value` file that `docker compose` auto-loads (and that the user can `set -a; source .env; set +a` into a shell).

Content comes from the Vaultwarden item `igait/dev-env` (org: `igait-niu`), whose `.fields[]` array holds every required value.

> The Firebase SDK's GCP key file (`/credentials/gcp-key.json`) used to live here too — it's now baked as a stub into every runtime image by `.docker/workspace/Dockerfile` so local dev and CI no longer need a real key. Prod continues to mount the real key from a K8s Secret. If the Vaultwarden item still has a `GCP_KEY_JSON` field, skip it.

## Steps

### 1. Sanity-check tooling

Run `bw --version` and `jq --version` in one Bash call. If either is missing, tell the user to enter the Nix dev shell (`nix develop` or hook direnv — but note we no longer *rely* on direnv; it's just a convenience) and stop.

### 2. Walk the Bitwarden auth state machine

Run `bw status | jq -r '.status'`. Branch on the output:

- **`unauthenticated`** → Stop and tell the user to run, using the `!` prefix so the command runs in this shell:
  ```
  !bw config server https://vault.igaitapp.com
  !bw login
  ```
  Then re-invoke `/igait-environment`.

- **`locked`** → Stop and tell the user to:
  - Close Claude
  - Run:
  ```
  export BW_SESSION=$(bw unlock --raw)
  ```
  - Reload Claude and invoke `/igait-environment`

- **`unlocked`** → Continue.

- **anything else** → Show the raw `bw status` output and ask the user to diagnose.

### 3. Fetch the item

```
bw sync --quiet
bw get item igait/dev-env
```

If the fetch fails, the user probably lacks read access to the `igait-niu` collection — tell them to ask @hiibolt.

### 4. Write `.env`

Parse the returned JSON and write every `fields[]` entry as `NAME=value` to `./.env`, **except** `GCP_KEY_JSON` (skip it — legacy field, no longer needed; the runtime image bakes a stub). Use `jq` to emit the lines; quote values that contain whitespace or special characters so shell sourcing stays correct.

Idempotent: overwrite `.env` wholesale. Set it to mode 600.

### 5. Report

One line: how many env vars were written. Done.

## Notes

- `.env` is gitignored; never commit it.
- `docker compose` picks up `.env` automatically from the project root — no extra step.
- The tracked `.envrc` sources the same `.env` via `dotenv_if_exists`, so shells that have direnv hooked also get these vars after a `direnv reload`. `.envrc` itself does not touch Vaultwarden — this command is the only auth-bearing path.
- Re-running this command is the supported resync path after a Vaultwarden rotation. There is no watcher and no automatic refresh.
