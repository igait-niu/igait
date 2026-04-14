/**
 * Hook for fetching user jobs from Firebase RTDB with real-time updates
 */

import { getFirebaseDatabase } from '$lib/firebase';
import { ref, onValue, type Unsubscribe } from 'firebase/database';
import type { Job } from '../../types/Job';

export type JobWithId = Job & { id: string };

/**
 * State of jobs loading
 */
export type JobsState =
	| { readonly status: 'loading' }
	| { readonly status: 'error'; readonly error: string }
	| { readonly status: 'loaded'; readonly jobs: JobWithId[] };

/**
 * Subscribe to a user's jobs in Firebase RTDB
 * Returns an unsubscribe function to clean up the listener
 *
 * @param uid - The user's Firebase UID
 * @param onUpdate - Callback that receives the new jobs state
 * @returns Unsubscribe function
 */
export function subscribeToJobs(uid: string, onUpdate: (state: JobsState) => void): Unsubscribe {
	const db = getFirebaseDatabase();
	const jobsRef = ref(db, `users/${uid}/jobs`);

	// Set initial loading state
	onUpdate({ status: 'loading' });

	// Subscribe to real-time updates
	const unsubscribe = onValue(
		jobsRef,
		(snapshot) => {
			const data = snapshot.val();

			if (!data) {
				// No jobs yet
				onUpdate({ status: 'loaded', jobs: [] });
				return;
			}

			// Data is stored as an array in RTDB
			// Filter out any placeholder/null entries and attach original index as ID
			const jobs: JobWithId[] = Array.isArray(data)
				? data
						.map((job, index): JobWithId | null => {
							if (job === null || job.email === 'placeholder@placeholder.com') return null;
							return { ...job, id: `${uid}_${index}` };
						})
						.filter((job): job is JobWithId => job !== null)
				: Object.entries(data)
						.map(([key, job]): JobWithId | null => {
							if (job === null || (job as Job).email === 'placeholder@placeholder.com') return null;
							return { ...(job as Job), id: `${uid}_${key}` };
						})
						.filter((job): job is JobWithId => job !== null);

			// Sort by timestamp (newest first)
			jobs.sort((a, b) => b.timestamp - a.timestamp);

			onUpdate({ status: 'loaded', jobs });
		},
		(error) => {
			console.error('Error fetching jobs:', error);
			onUpdate({ status: 'error', error: error.message });
		}
	);

	return unsubscribe;
}

/**
 * Helper to check if jobs state is loading
 */
export function isJobsLoading(state: JobsState): state is { status: 'loading' } {
	return state.status === 'loading';
}

/**
 * Helper to check if jobs state has errored
 */
export function isJobsError(state: JobsState): state is { status: 'error'; error: string } {
	return state.status === 'error';
}

/**
 * Helper to check if jobs state is loaded
 */
export function isJobsLoaded(state: JobsState): state is { status: 'loaded'; jobs: Job[] } {
	return state.status === 'loaded';
}
