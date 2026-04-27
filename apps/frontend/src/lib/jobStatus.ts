/**
 * Effective job status — derived from the three independent state dimensions
 * the backend exposes: top-level `JobStatus`, the per-stage `StageStatus` map,
 * and the `requires_approval` / `approved` queue gate.
 *
 * `JobStatus.stage` is written when a stage **starts**, not when it finishes,
 * so reading it on its own is misleading the moment a stage completes —
 * it lingers on the *previous* stage until the next worker claims and
 * updates it. Joining all three dimensions here is what lets the UI say
 * "Awaiting manual review" or "Queued for Pose estimation" instead of the
 * misleading "Processing: <stage that already finished>".
 */
import { Activity, CheckCircle2, Clock, Eye, XCircle } from '@lucide/svelte';
import type { Job } from '../types/Job';
import { stageLogsKey, type StageSpec } from './stores';

export type EffectiveJobStatus =
	| { kind: 'submitted' }
	| { kind: 'awaiting_review'; nextStage?: StageSpec }
	| { kind: 'awaiting_pickup'; stage: StageSpec }
	| { kind: 'queued'; stage: StageSpec }
	| { kind: 'running'; stage: StageSpec }
	| { kind: 'complete'; asd: boolean }
	| { kind: 'error' };

export type BadgeVariant = 'default' | 'secondary' | 'destructive' | 'outline';

/**
 * Decision tree (priority order):
 *   1. Terminal states (Complete, Error) trump everything.
 *   2. `requires_approval && !approved` → awaiting_review (no worker will
 *      claim this job until a human flips `approved`).
 *   3. Any stage with StageStatus = running → running (that stage).
 *   4. JobStatus = Processing { stage: N }:
 *        – stage_statuses[N] === 'complete' → awaiting_pickup of the next
 *          not-yet-complete stage (the gap that today reads as
 *          "Processing: N", which is the bug we're fixing).
 *        – otherwise → queued (stage N hasn't started running yet).
 *   5. Submitted → submitted (brand-new, before any stage_statuses exist).
 */
export function getEffectiveJobStatus(job: Job, stages: StageSpec[]): EffectiveJobStatus {
	if (job.status.code === 'Complete') {
		return { kind: 'complete', asd: job.status.asd };
	}
	if (job.status.code === 'Error') {
		return { kind: 'error' };
	}

	const stageStatuses = job.stage_statuses ?? {};
	const firstNonComplete = (): StageSpec | undefined =>
		stages.find((s) => stageStatuses[stageLogsKey(s)] !== 'complete');

	if (job.requires_approval && !job.approved) {
		return { kind: 'awaiting_review', nextStage: firstNonComplete() };
	}

	const running = stages.find((s) => stageStatuses[stageLogsKey(s)] === 'running');
	if (running) {
		return { kind: 'running', stage: running };
	}

	if (job.status.code === 'Processing') {
		const currentKey = job.status.stage;
		const currentStage = stages.find((s) => s.key === currentKey);
		if (currentStage) {
			const currentStatus = stageStatuses[stageLogsKey(currentStage)];
			if (currentStatus === 'complete') {
				const next = firstNonComplete();
				if (next) return { kind: 'awaiting_pickup', stage: next };
			}
			return { kind: 'queued', stage: currentStage };
		}
	}

	return { kind: 'submitted' };
}

// ── Display helpers ────────────────────────────────────────────────

/** Terse label for table cells / accordion badges. */
export function jobStatusLabel(s: EffectiveJobStatus): string {
	switch (s.kind) {
		case 'submitted':
			return 'Submitted';
		case 'awaiting_review':
			return 'Awaiting manual review';
		case 'awaiting_pickup':
		case 'queued':
			return `Queued for ${s.stage.display_name}`;
		case 'running':
			return s.stage.display_name;
		case 'complete':
			return s.asd ? 'ASD Indicators' : 'No ASD Indicators';
		case 'error':
			return 'Error';
	}
}

/** Verbose label for the job-detail page. */
export function jobStatusLabelVerbose(s: EffectiveJobStatus): string {
	switch (s.kind) {
		case 'submitted':
			return 'Submitted';
		case 'awaiting_review':
			return 'Awaiting manual review';
		case 'awaiting_pickup':
		case 'queued':
			return `Queued for ${s.stage.display_name}`;
		case 'running':
			return `Processing: ${s.stage.display_name}`;
		case 'complete':
			return s.asd ? 'ASD indicators detected' : 'No ASD indicators';
		case 'error':
			return 'Analysis failed';
	}
}

export function jobStatusVariant(s: EffectiveJobStatus): BadgeVariant {
	switch (s.kind) {
		case 'submitted':
		case 'awaiting_review':
		case 'awaiting_pickup':
		case 'queued':
			return 'outline';
		case 'running':
		case 'complete':
			return 'secondary';
		case 'error':
			return 'destructive';
	}
}

export function jobStatusIcon(s: EffectiveJobStatus) {
	switch (s.kind) {
		case 'submitted':
			return Clock;
		case 'awaiting_review':
			return Eye;
		case 'awaiting_pickup':
		case 'queued':
			return Clock;
		case 'running':
			return Activity;
		case 'complete':
			return CheckCircle2;
		case 'error':
			return XCircle;
	}
}

/** True for any non-terminal status — used by table filters. */
export function isInFlight(s: EffectiveJobStatus): boolean {
	return s.kind !== 'complete' && s.kind !== 'error';
}
