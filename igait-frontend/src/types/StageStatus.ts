/**
 * Per-stage status for tracking individual stage progress.
 *
 * Values match the Rust `StageStatus` enum serialized with `rename_all = "snake_case"`.
 */
export type StageStatus = 'not_started' | 'running' | 'complete' | 'error';
