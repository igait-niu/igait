FROM node:20-alpine

RUN apk add --no-cache openjdk21-jre-headless wget \
 && npm install -g firebase-tools \
 && firebase setup:emulators:database
# Note: the auth emulator ships bundled inside firebase-tools itself — no
# separate `setup:emulators:auth` subcommand exists. Only jar-backed
# emulators (database, firestore, pubsub, storage) need downloading.

WORKDIR /app

# Bake .firebaserc — it's a static project-id pin, no need to bind-mount
# and no reason it should differ per environment.
COPY .firebaserc /app/.firebaserc

CMD ["firebase", "emulators:start", "--only", "auth,database", "--project", "igait-local"]
