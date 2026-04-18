/**
 * Stage registry store — subscribes to `/registry/stages` in RTDB
 * (published by the backend on startup from igait-lib's STAGES const)
 * and exposes it as a reactive, ordered list.
 *
 * This is the single source of truth for which stages exist, their
 * ordering, and their UI metadata. No stage info should be hardcoded
 * anywhere on the frontend; instead, consumers read from this store
 * and the backend is the only place that defines new stages.
 */

import { getFirebaseDatabase } from '$lib/firebase';
import { onValue, ref } from 'firebase/database';

/**
 * The panel variant associated with a stage. Serialized by the Rust
 * `StagePanel` enum (kebab-case). Adding a new variant requires a
 * matching entry both here and in the Rust enum.
 */
export type StagePanel = 'default' | 'video-edit' | 'gait-cycles';

/**
 * One stage's published shape. Mirrors the `PublishedStage` struct in
 * igait-backend/src/helper/registry_publish.rs.
 */
export interface StageSpec {
	key: string;
	display_name: string;
	description: string;
	terminal: boolean;
	panel: StagePanel;
	order: number;
}

export type RegistryState =
	| { readonly status: 'loading' }
	| { readonly status: 'error'; readonly error: string }
	| { readonly status: 'loaded'; readonly stages: StageSpec[] };

class RegistryStore {
	#state = $state<RegistryState>({ status: 'loading' });
	#unsubscribe: (() => void) | undefined;

	get state(): RegistryState {
		return this.#state;
	}

	/**
	 * Start subscribing to `/registry/stages`. Safe to call multiple
	 * times — only subscribes once.
	 */
	initialize(): void {
		if (this.#unsubscribe) return;

		const db = getFirebaseDatabase();
		const registryRef = ref(db, 'registry/stages');

		this.#unsubscribe = onValue(
			registryRef,
			(snapshot) => {
				const data = snapshot.val() as Record<string, StageSpec> | null;
				if (!data) {
					this.#state = {
						status: 'error',
						error: 'Registry missing at /registry/stages. Is the backend running?'
					};
					return;
				}
				const stages = Object.values(data).sort((a, b) => a.order - b.order);
				this.#state = { status: 'loaded', stages };
			},
			(error) => {
				console.error('Error subscribing to registry:', error);
				this.#state = { status: 'error', error: error.message };
			}
		);
	}

	destroy(): void {
		this.#unsubscribe?.();
		this.#unsubscribe = undefined;
	}
}

export const registryStore = new RegistryStore();

// ── Path helpers ───────────────────────────────────────────────────
// Every runtime path now uses the stage key directly. These helpers
// exist so callers don't have to remember which field to use — and
// so we have one place to change if the naming convention ever
// shifts again.

/** RTDB `/queues/{stage.key}` segment. */
export function queuePathSegment(stage: StageSpec): string {
	return stage.key;
}

/** Key into `job.stage_logs` / `job.stage_statuses`. */
export function stageLogsKey(stage: StageSpec): string {
	return stage.key;
}

/** Key into the files-API response (`stages.{stage.key}`). */
export function stageFilesKey(stage: StageSpec): string {
	return stage.key;
}

/** 1-indexed stage number, for APIs that still expect a number (e.g. /rerun). */
export function stageNumber(stage: StageSpec): number {
	return stage.order + 1;
}

// ── Type guards ────────────────────────────────────────────────────

export function isRegistryLoading(
	state: RegistryState
): state is { status: 'loading' } {
	return state.status === 'loading';
}

export function isRegistryError(
	state: RegistryState
): state is { status: 'error'; error: string } {
	return state.status === 'error';
}

export function isRegistryLoaded(
	state: RegistryState
): state is { status: 'loaded'; stages: StageSpec[] } {
	return state.status === 'loaded';
}
