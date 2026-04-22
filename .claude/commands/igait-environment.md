---
description: Materialise iGait dev environment (.env + credentials/gcp-key.json) from Vaultwarden. Run once per machine; re-run to resync after a secret rotation.
---

You are setting up — or resyncing — the iGait development environment for the user in this repo. Work through the steps below. Be concise: one short status line per step is enough. Only get chatty if something breaks and the user needs to intervene.

## Goal

Produce two artefacts at repo root:

1. `./.env` — a plain `KEY=value` file that `docker compose` auto-loads (and that the user can `set -a; source .env; set +a` into a shell).
2. `./credentials/gcp-key.json` (mode 600) — the GCP service-account JSON the Firebase SDK insists must exist on disk, even when the emulator ignores it.

Both must come from the Vaultwarden item `igait/dev-env` (org: `igait-niu`). The item stores every required field in its `.fields[]` array; one field named `GCP_KEY_JSON` holds the JSON blob that lands in `credentials/gcp-key.json`.

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

Parse the returned JSON and write every `fields[]` entry as `NAME=value` to `./.env`, **except** `GCP_KEY_JSON` (skip it — it's a file payload, not an env var). Use `jq` to emit the lines; quote values that contain whitespace or special characters so shell sourcing stays correct.

Idempotent: overwrite `.env` wholesale. Set it to mode 600.

### 5. Materialise the GCP key

Extract `GCP_KEY_JSON` from the same JSON, `mkdir -p credentials`, write it to `credentials/gcp-key.json`, and `chmod 600`.

If the field is missing or empty, fall back to any existing file on disk; if neither exists, warn the user and explain that they need to add a `GCP_KEY_JSON` custom field to the `igait/dev-env` item (full service-account JSON as the value).

### 6. Report

One line: how many env vars were written, whether the GCP key was materialised. Done.

## Notes

- `.env` and `credentials/` are gitignored; never commit either.
- `docker compose` picks up `.env` automatically from the project root — no extra step.
- The tracked `.envrc` sources the same `.env` via `dotenv_if_exists`, so shells that have direnv hooked also get these vars after a `direnv reload`. `.envrc` itself does not touch Vaultwarden — this command is the only auth-bearing path.
- Re-running this command is the supported resync path after a Vaultwarden rotation. There is no watcher and no automatic refresh.
