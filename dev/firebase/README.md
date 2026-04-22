# Firebase emulator config

Used by the `firebase-emulator` service in `docker-compose.yml` to simulate
Firebase RTDB locally. Project ID is `igait-local`; RTDB listens on port
9000 inside the container.

`database.rules.json` at the repo root is bind-mounted in — edit it there,
not here.
