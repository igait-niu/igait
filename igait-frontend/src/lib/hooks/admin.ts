/**
 * Admin hooks for subscribing to queues and all jobs (admin only)
 */

import { getFirebaseDatabase } from '$lib/firebase';
import { ref, onValue, set, type Unsubscribe } from 'firebase/database';
import type { Job } from '../../types/Job';
import type { JobStatus } from '../../types/JobStatus';

/**
 * Queue item structure from Firebase RTDB
 */
export interface QueueItem {
	job_id: string;
	user_id: string;
	enqueued_at: number;
	claimed_by?: string;
	claimed_at?: number;
	input_keys: Record<string, string>;
	metadata: {
		age?: number;
		email?: string;
		ethnicity?: string;
		sex?: string;
		height?: string;
		weight?: number;
	};
	requires_approval?: boolean;
	approved?: boolean;
}

/**
 * Finalize queue item (slightly different structure)
 */
export interface FinalizeQueueItem extends QueueItem {
	success: boolean;
	error?: string;
	error_logs?: string;
	failed_at_stage?: number;
}

/**
 * All queues data structure
 */
export interface QueuesData {
	stage_1: Record<string, QueueItem>;
	stage_2: Record<string, QueueItem>;
	stage_3: Record<string, QueueItem>;
	stage_4: Record<string, QueueItem>;
	stage_5: Record<string, QueueItem>;
	stage_6: Record<string, QueueItem>;
	finalize: Record<string, FinalizeQueueItem>;
}

/**
 * State of queues loading
 */
export type QueuesState =
	| { readonly status: 'loading' }
	| { readonly status: 'error'; readonly error: string }
	| { readonly status: 'loaded'; readonly queues: QueuesData };

/**
 * Subscribe to all queues in Firebase RTDB (admin only)
 */
export function subscribeToQueues(onUpdate: (state: QueuesState) => void): Unsubscribe {
	const db = getFirebaseDatabase();
	const queuesRef = ref(db, 'queues');

	onUpdate({ status: 'loading' });

	const unsubscribe = onValue(
		queuesRef,
		(snapshot) => {
			const data = snapshot.val();

			const queues: QueuesData = {
				stage_1: data?.stage_1 ?? {},
				stage_2: data?.stage_2 ?? {},
				stage_3: data?.stage_3 ?? {},
				stage_4: data?.stage_4 ?? {},
				stage_5: data?.stage_5 ?? {},
				stage_6: data?.stage_6 ?? {},
				finalize: data?.finalize ?? {}
			};

			onUpdate({ status: 'loaded', queues });
		},
		(error) => {
			console.error('Error fetching queues:', error);
			onUpdate({ status: 'error', error: error.message });
		}
	);

	return unsubscribe;
}

/**
 * Job with user ID for admin view - extends Job with id field
 */
export type AdminJob = Job & {
	id: string; // Full job ID: userId_jobIndex
};

/**
 * State of all jobs loading
 */
export interface AllJobsState {
	loading: boolean;
	jobs: AdminJob[];
	error?: string;
}

/**
 * Subscribe to all jobs across all users (admin only)
 */
export function subscribeToAllJobs(onUpdate: (state: AllJobsState) => void): Unsubscribe {
	const db = getFirebaseDatabase();
	const usersRef = ref(db, 'users');

	onUpdate({ loading: true, jobs: [] });

	const unsubscribe = onValue(
		usersRef,
		(snapshot) => {
			const data = snapshot.val();

			if (!data) {
				onUpdate({ loading: false, jobs: [] });
				return;
			}

			const allJobs: AdminJob[] = [];

			// Iterate through all users
			for (const [userId, userData] of Object.entries(data)) {
				const user = userData as { jobs?: Job[]; administrator?: boolean };
				if (!user.jobs) continue;

				// Handle both array and object formats
				const jobs: Job[] = Array.isArray(user.jobs) ? user.jobs : Object.values(user.jobs);

				jobs.forEach((job, index) => {
					if (!job || !job.email) return;

					allJobs.push({
						...job,
						id: `${userId}_${index}`
					});
				});
			}

			// Sort by timestamp (newest first)
			allJobs.sort((a, b) => b.timestamp - a.timestamp);

			onUpdate({ loading: false, jobs: allJobs });
		},
		(error) => {
			console.error('Error fetching all jobs:', error);
			onUpdate({ loading: false, jobs: [], error: error.message });
		}
	);

	return unsubscribe;
}

// Helper type guards for queues
export function isQueuesLoading(state: QueuesState): state is { status: 'loading' } {
	return state.status === 'loading';
}

export function isQueuesError(state: QueuesState): state is { status: 'error'; error: string } {
	return state.status === 'error';
}

export function isQueuesLoaded(
	state: QueuesState
): state is { status: 'loaded'; queues: QueuesData } {
	return state.status === 'loaded';
}

// ============================================================================
// QUEUE CONFIG
// ============================================================================

/**
 * Configuration for a single queue stage
 */
export interface QueueConfigItem {
	requires_approval: boolean;
}

/**
 * All queue configs keyed by stage
 */
export interface QueueConfigData {
	[key: string]: QueueConfigItem | undefined;
}

/**
 * State of queue config loading
 */
export type QueueConfigState =
	| { readonly status: 'loading' }
	| { readonly status: 'error'; readonly error: string }
	| { readonly status: 'loaded'; readonly configs: QueueConfigData };

/**
 * Subscribe to queue configuration in Firebase RTDB
 */
export function subscribeToQueueConfigs(onUpdate: (state: QueueConfigState) => void): Unsubscribe {
	const db = getFirebaseDatabase();
	const configRef = ref(db, 'queue_config');

	onUpdate({ status: 'loading' });

	const unsubscribe = onValue(
		configRef,
		(snapshot) => {
			const data = snapshot.val();
			onUpdate({ status: 'loaded', configs: data ?? {} });
		},
		(error) => {
			console.error('Error fetching queue configs:', error);
			onUpdate({ status: 'error', error: error.message });
		}
	);

	return unsubscribe;
}

/**
 * Set the requires_approval flag for a queue stage
 */
export async function setQueueRequiresApproval(stageKey: string, value: boolean): Promise<void> {
	const db = getFirebaseDatabase();
	const configRef = ref(db, `queue_config/${stageKey}/requires_approval`);
	await set(configRef, value);
}

/**
 * Approve a queue item directly in RTDB.
 * Updates both the queue item and the user's job record.
 */
export async function approveQueueItem(
	stageKey: string,
	itemKey: string,
	item: QueueItem | FinalizeQueueItem
): Promise<void> {
	const db = getFirebaseDatabase();

	// Update queue item
	const queueRef = ref(db, `queues/${stageKey}/${itemKey}/approved`);
	await set(queueRef, true);

	// Also update user's job record
	const jobIndex = parseInt(item.job_id.split('_').pop() ?? '0', 10);
	const userJobRef = ref(db, `users/${item.user_id}/jobs/${jobIndex}/approved`);
	await set(userJobRef, true);
}

/**
 * Convert a QueueItem to a Job-compatible shape for the data table
 */
export function queueItemToJob(item: QueueItem | FinalizeQueueItem): Job & { id: string } {
	return {
		id: item.job_id,
		age: item.metadata?.age ?? 0,
		email: item.metadata?.email ?? '',
		ethnicity: (item.metadata?.ethnicity ?? 'Unknown') as any,
		sex: (item.metadata?.sex ?? 'O') as any,
		height: item.metadata?.height ?? '',
		weight: item.metadata?.weight ?? 0,
		timestamp: Math.floor(item.enqueued_at / 1000),
		status: item.claimed_by
			? { code: 'Submitted' as const, value: 'Claimed' }
			: { code: 'Submitted' as const, value: 'Waiting in queue' },
		requires_approval: item.requires_approval ?? false,
		approved: item.approved ?? false,
		stage_logs: {},
		stage_statuses: {}
	};
}

// Helper type guards for queue config
export function isQueueConfigLoaded(
	state: QueueConfigState
): state is { status: 'loaded'; configs: QueueConfigData } {
	return state.status === 'loaded';
}

// ============================================================================
// SINGLE JOB SUBSCRIPTION (ADMIN)
// ============================================================================

/**
 * State of a single job loading
 */
export type SingleJobState =
	| { readonly status: 'loading' }
	| { readonly status: 'error'; readonly error: string }
	| { readonly status: 'loaded'; readonly job: Job };

/**
 * Subscribe to a single job by user ID and job index (admin only).
 * Parses a composite job ID of the form "userId_jobIndex".
 */
export function subscribeToJob(
	jobId: string,
	onUpdate: (state: SingleJobState) => void
): Unsubscribe {
	const db = getFirebaseDatabase();

	// Parse "userId_jobIndex" — the last segment after '_' is the index
	const lastUnderscore = jobId.lastIndexOf('_');
	if (lastUnderscore === -1) {
		onUpdate({ status: 'error', error: `Invalid job ID format: ${jobId}` });
		return () => {};
	}
	const userId = jobId.slice(0, lastUnderscore);
	const jobIndex = jobId.slice(lastUnderscore + 1);

	const jobRef = ref(db, `users/${userId}/jobs/${jobIndex}`);

	onUpdate({ status: 'loading' });

	const unsubscribe = onValue(
		jobRef,
		(snapshot) => {
			const data = snapshot.val();
			if (!data) {
				onUpdate({ status: 'error', error: 'Job not found' });
				return;
			}
			onUpdate({ status: 'loaded', job: data as Job });
		},
		(error) => {
			console.error('Error fetching job:', error);
			onUpdate({ status: 'error', error: error.message });
		}
	);

	return unsubscribe;
}
