# build environment
FROM oven/bun:1 AS builder
WORKDIR /usr/src/app
ENV PATH /usr/src/app/node_modules/.bin:$PATH

# Firebase & API config — required at build time for Vite to inline them
ARG VITE_FIREBASE_API_KEY
ARG VITE_FIREBASE_AUTH_DOMAIN
ARG VITE_FIREBASE_PROJECT_ID
ARG VITE_FIREBASE_STORAGE_BUCKET
ARG VITE_FIREBASE_MESSAGING_SENDER_ID
ARG VITE_FIREBASE_APP_ID
ARG VITE_FIREBASE_DATABASE_URL
ARG VITE_FIREBASE_MEASUREMENT_ID
ARG VITE_API_BASE_URL
ARG VITE_FIREBASE_USE_EMULATOR

COPY package.json bun.lock /usr/src/app/
RUN bun install
COPY . /usr/src/app
RUN bun run build
EXPOSE 4173
CMD ["bun", "run", "preview", "--host", "0.0.0.0"]
