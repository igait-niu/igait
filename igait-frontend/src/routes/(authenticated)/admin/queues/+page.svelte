<script lang="ts">
	import { onDestroy } from 'svelte';
	import { goto } from '$app/navigation';
	import {
		subscribeToQueues,
		subscribeToQueueConfigs,
		isQueuesLoading,
		isQueuesError,
		isQueuesLoaded,
		isQueueConfigLoaded,
		setQueueRequiresApproval,
		queueItemToJob,
		type QueuesState,
		type QueuesData,
		type QueueConfigState,
		type QueueItem,
		type FinalizeQueueItem
	} from '$lib/hooks';
	import { Badge } from '$lib/components/ui/badge';
	import { Switch } from '$lib/components/ui/switch';
	import { JobsDataTable } from '$lib/components/jobs';
	import { Inbox, Activity, ShieldCheck } from '@lucide/svelte';
	import AdminLoadingState from '../AdminLoadingState.svelte';
	import AdminErrorState from '../AdminErrorState.svelte';
	import type { Job } from '../../../../types/Job';

	// ── State ──────────────────────────────────────────────

	let queuesState: QueuesState = $state({ status: 'loading' });
	let configState: QueueConfigState = $state({ status: 'loading' });
	let activeStage: string = $state('stage_1');

	// ── Subscriptions ──────────────────────────────────────

	const unsubQueues = subscribeToQueues((state) => {
		queuesState = state;
	});

	const unsubConfigs = subscribeToQueueConfigs((state) => {
		configState = state;
	});

	onDestroy(() => {
		unsubQueues();
		unsubConfigs();
	});

	// ── Stage info ─────────────────────────────────────────

	const stageInfo = [
		{ key: 'stage_1', name: 'Stage 1', description: 'Media Conversion' },
		{ key: 'stage_2', name: 'Stage 2', description: 'Validity Check' },
		{ key: 'stage_3', name: 'Stage 3', description: 'Reframing' },
		{ key: 'stage_4', name: 'Stage 4', description: 'Pose Estimation' },
		{ key: 'stage_5', name: 'Stage 5', description: 'Cycle Detection' },
		{ key: 'stage_6', name: 'Stage 6', description: 'ML Prediction' },
		{ key: 'finalize', name: 'Stage 7', description: 'Finalize' }
	] as const;

	// ── Derived data ───────────────────────────────────────

	/** Get the count of items in a queue */
	function getQueueItemCount(key: string): number {
		if (!isQueuesLoaded(queuesState)) return 0;
		const queue = queuesState.queues[key as keyof QueuesData];
		return Object.keys(queue || {}).length;
	}

	/** Items for the active stage, preserving RTDB keys */
	const activeQueueEntries = $derived.by(() => {
		if (!isQueuesLoaded(queuesState)) return [];
		const queue = queuesState.queues[activeStage as keyof QueuesData];
		if (!queue) return [];
		return Object.entries(queue).map(([key, item]) => ({
			key,
			item: item as QueueItem | FinalizeQueueItem
		}));
	});

	/** Queue items converted to Job format for the data table */
	const jobsForTable = $derived(activeQueueEntries.map(({ item }) => queueItemToJob(item)));

	/** Total jobs across all queues */
	const totalJobs = $derived.by(() => {
		return stageInfo.reduce((total, stage) => total + getQueueItemCount(stage.key), 0);
	});

	/** Whether the active stage's queue requires manual approval */
	const activeRequiresApproval = $derived.by(() => {
		if (!isQueueConfigLoaded(configState)) return false;
		return configState.configs[activeStage]?.requires_approval ?? false;
	});

	/** Active stage display info */
	const activeStageInfo = $derived(stageInfo.find((s) => s.key === activeStage) ?? stageInfo[0]);

	/** Count for active stage */
	const activeStageCount = $derived(getQueueItemCount(activeStage));

	// ── Handlers ───────────────────────────────────────────

	function handleSelectStage(stageKey: string) {
		activeStage = stageKey;
	}

	function handleSelectJob(job: Job & { id: string }) {
		goto(`/job/${encodeURIComponent(job.id)}`);
	}

	async function handleToggleApproval(value: boolean) {
		await setQueueRequiresApproval(activeStage, value);
	}
</script>

<svelte:head>
	<title>Queue Overview - Admin - iGait</title>
</svelte:head>

{#if isQueuesLoading(queuesState)}
	<AdminLoadingState message="Loading queues..." />
{:else if isQueuesError(queuesState)}
	<AdminErrorState message="Failed to load queues: {queuesState.error}" />
{:else if isQueuesLoaded(queuesState)}
	<div class="queue-overview">
		<!-- Pipeline Status Summary -->
		<div class="pipeline-summary">
			<Activity class="summary-icon" />
			<h2 class="summary-title">Pipeline Status</h2>
			<Badge variant="secondary" class="summary-badge">{totalJobs} active</Badge>
		</div>

		<!-- Stage Tabs + Content -->
		<div class="stage-content-wrapper">
			<div class="stage-tabs-container">
				<div class="stage-tabs">
				{#each stageInfo as stage}
					{@const count = getQueueItemCount(stage.key)}
					<button
						class="stage-tab"
						class:active={activeStage === stage.key}
						onclick={() => handleSelectStage(stage.key)}
					>
						<span class="tab-name">{stage.name}</span>
						<span class="tab-desc">{stage.description}</span>
						{#if count > 0}
							<Badge variant="default" class="stage-count-badge">{count}</Badge>
						{/if}
					</button>
					{/each}
				</div>
			</div>

			<!-- Main Content Card -->
			<div class="main-content-card">
			<!-- Controls Row -->
			<div class="controls-row">
				<div class="controls-left">
					<span class="active-stage-label">{activeStageInfo.description}</span>
					<Badge variant="outline" class="queue-count-badge {activeStageCount === 0 ? 'queue-count-zero' : ''}">{activeStageCount} job{activeStageCount !== 1 ? 's' : ''}</Badge>
				</div>

				<label class="approval-toggle">
					<ShieldCheck class="approval-icon" />
					<span class="toggle-label">Manual Approval</span>
					<Switch checked={activeRequiresApproval} onCheckedChange={handleToggleApproval} />
				</label>
			</div>

			<!-- Content -->
			<div class="content-area">
				{#if jobsForTable.length === 0}
					<div class="empty-state">
						<Inbox class="empty-icon" />
						<p class="empty-title">No jobs in queue</p>
						<p class="empty-description">
							{activeStageInfo.description} has no pending items right now.
						</p>
					</div>
				{:else}
					<JobsDataTable data={jobsForTable} uid="" showEmail={true} onRowClick={handleSelectJob} />
				{/if}
				</div>
			</div>
		</div>
	</div>
{/if}

<style>
	.queue-overview {
		display: flex;
		flex-direction: column;
		gap: 1.25rem;
	}

	/* ── Pipeline Summary ───────────────────────────────── */

	.pipeline-summary {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 0.625rem;
	}

	:global(.summary-icon) {
		width: 1.125rem;
		height: 1.125rem;
		color: var(--primary);
	}

	.summary-title {
		font-size: 1.125rem;
		font-weight: 600;
		margin: 0;
	}

	:global(.summary-badge) {
		font-size: 0.6875rem !important;
	}

	/* ── Stage Content Wrapper ───────────────────────────── */

	.stage-content-wrapper {
		border: 1px solid var(--border);
		border-radius: var(--radius-md);
		overflow: hidden;
	}

	/* ── Stage Tabs ─────────────────────────────────────── */

	.stage-tabs-container {
		background: oklch(from var(--primary) l c h / 0.04);
		border-bottom: 1px solid var(--border);
		padding: 0.5rem;
		overflow-x: auto;
	}

	.stage-tabs {
		display: flex;
		gap: 0.25rem;
		justify-content: center;
	}

	.stage-tab {
		position: relative;
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 0.125rem;
		padding: 0.5rem 1rem;
		border: 1px solid transparent;
		background: none;
		cursor: pointer;
		color: var(--muted-foreground);
		transition: all 0.15s ease;
		white-space: nowrap;
		border-radius: var(--radius-sm);
	}

	.stage-tab:hover {
		color: var(--foreground);
		background-color: oklch(from var(--primary) l c h / 0.06);
	}

	.stage-tab.active {
		color: var(--primary);
		background-color: var(--background);
		border-color: oklch(from var(--primary) l c h / 0.3);
		box-shadow: 0 1px 3px oklch(from var(--primary) l c h / 0.1);
	}

	.tab-name {
		font-size: 0.8125rem;
		font-weight: 600;
	}

	.stage-tab.active .tab-name {
		color: var(--primary);
	}

	.tab-desc {
		font-size: 0.6875rem;
		font-weight: 400;
		opacity: 0.7;
	}

	.stage-tab.active .tab-desc {
		opacity: 1;
	}

	:global(.stage-count-badge) {
		position: absolute;
		top: -0.25rem;
		right: -0.25rem;
		font-size: 0.5625rem !important;
		padding: 0 0.3rem !important;
		height: 1rem !important;
		min-width: 1rem !important;
		line-height: 1rem !important;
	}

	/* ── Main Content Card ────────────────────────────────── */

	.main-content-card {
		background: var(--card);
	}

	/* ── Controls Row ───────────────────────────────────── */

	.controls-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 1rem;
		padding: 0.625rem 0.875rem;
		background: oklch(from var(--muted) l c h / 0.35);
		border-bottom: 1px solid var(--border);
	}

	.controls-left {
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}

	.active-stage-label {
		font-size: 0.8125rem;
		font-weight: 600;
		color: var(--foreground);
	}

	:global(.queue-count-badge) {
		font-size: 0.625rem !important;
		padding: 0 0.375rem !important;
		height: 1.125rem !important;
		min-width: 1.125rem !important;
		scale: 0.9;
	}

	:global(.queue-count-zero) {
		border-color: oklch(0.637 0.237 25.331) !important;
		color: oklch(0.637 0.237 25.331) !important;
	}

	.approval-toggle {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		cursor: pointer;
	}

	:global(.approval-icon) {
		width: 0.875rem;
		height: 0.875rem;
		color: var(--muted-foreground);
	}

	.toggle-label {
		font-size: 0.8125rem;
		font-weight: 500;
		color: var(--foreground);
		user-select: none;
	}

	/* ── Content Area ───────────────────────────────────── */

	.content-area {
		min-height: 300px;
	}

	.empty-state {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		text-align: center;
		padding: 3.5rem 2rem;
		color: var(--muted-foreground);
	}

	:global(.empty-icon) {
		width: 2.5rem;
		height: 2.5rem;
		color: oklch(from var(--muted-foreground) l c h / 0.4);
		margin-bottom: 0.75rem;
	}

	.empty-title {
		font-size: 0.9375rem;
		font-weight: 600;
		color: var(--foreground);
		margin: 0 0 0.25rem;
	}

	.empty-description {
		font-size: 0.8125rem;
		margin: 0;
		max-width: 20rem;
		line-height: 1.4;
	}
</style>
