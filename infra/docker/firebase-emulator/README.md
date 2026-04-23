# firebase-emulator Dockerfile

Custom image for the `firebase-emulator` service in `infra/compose.yml`.
Simulates Firebase RTDB + Auth locally against project id `igait-local`.

## Why a custom image (vs. installing firebase-tools at boot)

The RTDB emulator is a JVM process, and `firebase-tools` dropped support
for Java <21. Installing openjdk + `firebase-tools` + pre-downloading
emulator jars at container start historically took several minutes and
periodically blew past the healthcheck window, cascading failures through
every `service_healthy` dependent.

This Dockerfile bakes all of that in — JRE 21, `firebase-tools`, the
pre-downloaded emulator jars (`firebase setup:emulators:{auth,database}`),
and `.firebaserc`. First `docker compose build` takes ~3–5 min once;
every boot after that is JVM startup (~5–10s), so `start_period: 30s` in
the compose healthcheck is plenty.

## Files

- `Dockerfile` — the image recipe.
- `.firebaserc` — baked in. Static project-id pin; no reason it should
  differ per environment.
- `firebase.json` — bind-mounted at runtime (not baked) so port / emulator
  config can be edited without a rebuild.
- `database.rules.json` lives at the repo root (shared with prod deploys)
  and is bind-mounted in by `infra/compose.yml`.
