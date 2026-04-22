# Firebase RTDB client

## The only Firebase client in this repo is `igait_lib::microservice::FirebaseRtdb`

Backend **and** all five stages talk to Firebase RTDB through the same client — a thin reqwest-based wrapper in `igait-lib/src/microservice/worker.rs`. It supports `get / set / update / delete / multi_update` plus CAS primitives (`get_with_etag`, `put_if_match`, `transaction`) used by the queue-claim logic.

## Do not re-introduce `firebase-rs` (or any other client)

The backend formerly used the `firebase-rs` crate while the stages used `FirebaseRtdb`. That split caused a P0 for the hermetic local stack: `firebase-rs` enforces an HTTPS scheme and rejects `http://firebase-emulator:9000/` with `NotHttps`, crashing the backend on boot. The fix (issue #102) was to consolidate on `FirebaseRtdb`. The historical `firebase-rs` dep is gone from `igait-backend/Cargo.toml` and should not come back.

If a future task wants "admin-ish" features `FirebaseRtdb` doesn't yet have, extend `FirebaseRtdb` — don't add a second client.

## Emulator vs prod URL shape

`FirebaseRtdb::url()` handles the case where `FIREBASE_RTDB_URL` already contains a query string. That matters because the emulator requires `?ns=<project>` on every request:

- Prod: `FIREBASE_RTDB_URL=https://igait-prod.firebaseio.com` → requests go to `.../path.json?auth=<token>`
- Emu:  `FIREBASE_RTDB_URL=http://firebase-emulator:9000/?ns=igait-local` → requests go to `.../path.json?auth=<token>&ns=igait-local`

Both shapes pass through the same builder. If you replace the builder or switch to a different HTTP layer, preserve this — or the emulator will return `403: Invalid Firebase database name`.

## Authentication

`FirebaseRtdb` uses an **access-token-style** auth: the token (from `FIREBASE_ACCESS_KEY`) is appended as `?auth=<token>`. The emulator accepts any non-empty value here; prod expects a database secret or a minted ID token with RTDB scope.

ID-token verification on incoming requests (the backend's `Authorization: Bearer` path) is a separate concern handled by the `firebase-auth` crate — see the Hermetic Stack wiki's note on `FIREBASE_AUTH_EMULATOR_HOST`.
