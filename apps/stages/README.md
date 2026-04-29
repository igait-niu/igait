# apps/stages

The five pipeline stages. Each is a thin Rust binary that consumes a
`QueueItem` from Firebase RTDB, runs one transformation, and enqueues
the next stage's work.

## The stage pattern

Every stage implements `StageWorker::process(&QueueItem)` from
`igait_lib::microservice` and wires it up with a single env-var gate in
its `main.rs`:

```rust
if std::env::var("IGAIT_JOB_PAYLOAD").is_ok() {
    run_stage_job(Worker).await      // one-shot, K8s Jobs (prod)
} else {
    run_stage_worker(Worker).await   // long-running, polling (local)
}
```

Both paths share the same `process()` — only the surrounding lifecycle
differs. **Finalize uses `IGAIT_FINALIZE_PAYLOAD`** because its queue
item type is `FinalizeQueueItem`, not `QueueItem`. Otherwise identical.

See [`docs/architecture/stage-execution-modes.md`](../../docs/architecture/stage-execution-modes.md)
for the full dual-mode story. Do not remove either mode — worker mode
is what the hermetic local stack depends on, job mode is what prod
depends on.

## The 5 stages

| # | Stage | Does | Internal docs |
|---|---|---|---|
| 1 | [`media-conversion/`](./media-conversion/) | ffmpeg normalization (fixed resolution + frame rate, H.264/AAC) | — |
| 2 | [`pose-estimation/`](./pose-estimation/) | MediaPipe skeleton extraction | [`igait-mediapipe`](./pose-estimation/igait-mediapipe/README.md) |
| 3 | [`cycle-detection/`](./cycle-detection/) | Gait cycle boundary detection | [`igait-gait-cycle-detection`](./cycle-detection/igait-gait-cycle-detection/README.md) |
| 4 | [`prediction/`](./prediction/) | ML model inference (autism likelihood) | [`iGAIT_MODEL_IO`](./prediction/iGAIT_MODEL_IO/README.md) |
| 5 | [`finalize/`](./finalize/) | Collate results, email the user, clean up S3 | — |

## Working on stages

For the dev loop (docker compose local, restart-on-edit, how to trigger
a test job), see [`docs/README.md#pipeline`](../../docs/README.md#pipeline).
