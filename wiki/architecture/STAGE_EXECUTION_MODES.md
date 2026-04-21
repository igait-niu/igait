# Stage execution modes

Every iGait pipeline stage is a single Rust binary that can run under one of
two lifecycles. The choice is made at stage-process startup by a single env
var check in each stage's `main.rs`:

```rust
if std::env::var("IGAIT_JOB_PAYLOAD").is_ok() {
    run_stage_job(Worker).await      // one-shot, K8s-Jobs mode
} else {
    run_stage_worker(Worker).await   // long-running, worker mode
}
```

Both lifecycles call the same `StageWorker::process(&QueueItem)` business
logic. Only the *surrounding* lifecycle differs.

## Worker mode (long-running)

Used in: **docker-compose (local dev), CI integration tests, pre-K8s-Jobs prod**.

The stage container boots, enters an infinite polling loop against the
Firebase RTDB queue for its stage, and processes one job at a time. The
container never exits until killed. Backed by `run_stage_worker()` in
`igait-lib/src/microservice/worker.rs`.

```
container starts → loop { poll queues/<stage> → process → write queues/<next> → sleep 3s }
```

## Job mode (one-shot)

Used in: **production (K8s Jobs)**.

A Kubernetes `Job` is created by the backend orchestrator with
`IGAIT_JOB_PAYLOAD` set in the pod's env to a serialised `QueueItem`. The
stage container reads the payload, processes exactly one job, writes its
result to RTDB, and exits. The `Job` is garbage-collected by K8s. Backed by
`run_stage_job()` in the same file.

```
orchestrator sees work → kubectl apply Job → pod boots → process once → exit
```

## Backend coordination

The backend has a matching gate: `ENABLE_ORCHESTRATOR=true` enables the K8s
Jobs orchestrator loop in `igait-backend/src/helper/orchestrator.rs`; anything
else disables it. For worker-mode deployments, leave `ENABLE_ORCHESTRATOR`
unset — the backend still handles uploads and enqueues work to RTDB, but
stages pick it up themselves.

## The queue is the sync point

Both lifecycles converge on Firebase RTDB as the coordination primitive. A
`QueueItem` at `queues/<stage>/<job>` is the unit of work; whoever claims it
first (K8s pod or long-running worker) wins. The queue is indifferent to the
consumer's lifecycle — this is what makes the dual-mode design tractable.

## When to use which

| Scenario | Mode |
|---|---|
| Local dev, frontend/feature work | worker mode |
| CI integration tests | worker mode |
| Production | job mode (K8s Jobs) |
| Debugging `orchestrator.rs` itself | job mode (against a `kind` cluster — not yet implemented) |

## Do not remove either mode

Both are load-bearing. Worker mode is what the hermetic local stack depends
on; job mode is what prod depends on. The env-var gate in each stage's
`main.rs` is the contract.
