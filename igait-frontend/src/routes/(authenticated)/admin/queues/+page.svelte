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
		approveQueueItem,
		queueItemToJob,
		type QueuesState,
		type QueueConfigState,
		type QueueItem,
		type FinalizeQueueItem
	} from '$lib/hooks';
	import {
		registryStore,
		queuePathSegment,
		isRegistryLoaded,
		isRegistryError
	} from '$lib/stores';
	import { Badge } from '$lib/components/ui/badge';
	import { Switch } from '$lib/components/ui/switch';
	import { JobsDataTable } from '$lib/components/jobs';
	import { StageTab } from '$lib/components';
	import { Inbox, Activity, ShieldCheck } from '@lucide/svelte';
	import AdminLoadingState from '../AdminLoadingState.svelte';
	import AdminErrorState from '../AdminErrorState.svelte';
	import type { Job } from '../../../../types/Job';

	// ── State ──────────────────────────────────────────────

	let queuesState: QueuesState = $state({ status: 'loading' });
	let configState: QueueConfigState = $state({ status: 'loading' });
	let activeStageKey: string = $state('');

	let approveError: string | null = $state(null);
	let approveSuccess: string | null = $state(null);

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

	// ── Registry-driven stage list ─────────────────────────

	const registryState = $derived(registryStore.state);
	const stages = $derived(isRegistryLoaded(registryState) ? registryState.stages : []);

	// Default the active stage to the first one once the registry loads.
	$effect(() => {
		if (stages.length > 0 && !stages.some((s) => s.key === activeStageKey)) {
			activeStageKey = stages[0].key;
		}
	});

	const activeStage = $derived(stages.find((s) => s.key === activeStageKey));
	const activeQueueSegment = $derived(
		activeStage ? queuePathSegment(activeStage) : ''
	);

	// ── Derived data ───────────────────────────────────────

	/** Count of items in a given stage's queue. */
	function getQueueItemCount(queueSegment: string): number {
		if (!isQueuesLoaded(queuesState)) return 0;
		const queue = queuesState.queues[queueSegment];
		return Object.keys(queue || {}).length;
	}

	/** Entries for the active stage's queue, preserving RTDB keys. */
	const activeQueueEntries = $derived.by(() => {
		if (!isQueuesLoaded(queuesState) || !activeQueueSegment) return [];
		const queue = queuesState.queues[activeQueueSegment];
		if (!queue) return [];
		return Object.entries(queue).map(([key, item]) => ({
			key,
			item: item as QueueItem | FinalizeQueueItem
		}));
	});

	/** Queue items converted to Job format for the data table. */
	const jobsForTable = $derived(
		activeQueueEntries.map(({ item }) => queueItemToJob(item))
	);

	/** Total jobs across all queues. */
	const totalJobs = $derived.by(() =>
		stages.reduce((total, s) => total + getQueueItemCount(queuePathSegment(s)), 0)
	);

	/** Whether the active stage's queue requires manual approval. */
	const activeRequiresApproval = $derived.by(() => {
		if (!isQueueConfigLoaded(configState) || !activeQueueSegment) return false;
		return configState.configs[activeQueueSegment]?.requires_approval ?? false;
	});

	const activeStageCount = $derived(
		activeQueueSegment ? getQueueItemCount(activeQueueSegment) : 0
	);

	// ── Handlers ───────────────────────────────────────────

	function handleSelectStage(stageKey: string) {
		activeStageKey = stageKey;
		approveError = null;
		approveSuccess = null;
	}

	function handleSelectJob(job: Job & { id: string }) {
		goto(`/job/${encodeURIComponent(job.id)}`);
	}

	async function handleToggleApproval(value: boolean) {
		if (!activeQueueSegment) return;
		await setQueueRequiresApproval(activeQueueSegment, value);
	}

	async function handleApproveJobs(jobIds: string[]) {
		approveError = null;
		approveSuccess = null;

		const targets = activeQueueEntries.filter(({ item }) => jobIds.includes(item.job_id));

		if (targets.length === 0) {
			approveError = 'Selected jobs are no longer in this queue.';
			throw new Error(approveError);
		}

		const results = await Promise.allSettled(
			targets.map(({ key, item }) => approveQueueItem(activeQueueSegment, key, item))
		);

		const failures = results.filter(
			(r): r is PromiseRejectedResult => r.status === 'rejected'
		);
		const succeeded = results.length - failures.length;

		if (failures.length === 0) {
			approveSuccess = `Approved ${succeeded} job${succeeded === 1 ? '' : 's'}.`;
			return;
		}

		const firstReason =
			failures[0].reason instanceof Error
				? failures[0].reason.message
				: String(failures[0].reason);
		approveError =
			succeeded > 0
				? `Approved ${succeeded} of ${results.length}; ${failures.length} failed: ${firstReason}`
				: `Approval failed: ${firstReason}`;

		// Re-throw so the table keeps the selection intact for retry.
		throw new Error(approveError);
	}
</script>

<svelte:head>
	<title>Queue Overview - Admin - iGait</title>
</svelte:head>

{#if isQueuesLoading(queuesState) || registryState.status === 'loading'}
	<AdminLoadingState message="Loading queues..." />
{:else if isQueuesError(queuesState)}
	<AdminErrorState message="Failed to load queues: {queuesState.error}" />
{:else if isRegistryError(registryState)}
	<AdminErrorState message="Failed to load stage registry: {registryState.error}" />
{:else if isQueuesLoaded(queuesState) && activeStage}
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
					{#each stages as stage (stage.key)}
						<StageTab
							{stage}
							active={activeStageKey === stage.key}
							count={getQueueItemCount(queuePathSegment(stage))}
							onclick={() => handleSelectStage(stage.key)}
						/>
					{/each}
				</div>
			</div>

			<!-- Main Content Card -->
			<div class="main-content-card">
				<!-- Controls Row -->
				<div class="controls-row">
					<div class="controls-left">
						<span class="active-stage-label">{activeStage.display_name}</span>
						<Badge
							variant="outline"
							class="queue-count-badge {activeStageCount === 0 ? 'queue-count-zero' : ''}"
							>{activeStageCount} job{activeStageCount !== 1 ? 's' : ''}</Badge
						>
					</div>

					<label class="approval-toggle">
						<ShieldCheck class="approval-icon" />
						<span class="toggle-label">Manual Approval</span>
						<Switch checked={activeRequiresApproval} onCheckedChange={handleToggleApproval} />
					</label>
				</div>

				<p class="stage-description">{activeStage.description}</p>

				{#if approveError}
					<div class="approve-banner approve-banner--error" role="alert">
						{approveError}
					</div>
				{/if}
				{#if approveSuccess}
					<div class="approve-banner approve-banner--success" role="status">
						{approveSuccess}
					</div>
				{/if}

				<!-- Content -->
				<div class="content-area">
					{#if jobsForTable.length === 0}
						<div class="empty-state">
							<Inbox class="empty-icon" />
							<p class="empty-title">No jobs in queue</p>
							<p class="empty-description">
								{activeStage.display_name} has no pending items right now.
							</p>
						</div>
					{:else}
						<JobsDataTable
							data={jobsForTable}
							uid=""
							showEmail={true}
							selectable={activeRequiresApproval}
							onRowClick={handleSelectJob}
							onApprove={handleApproveJobs}
						/>
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

	.stage-description {
		margin: 0;
		padding: 0.5rem 0.875rem 0;
		font-size: 0.75rem;
		color: var(--muted-foreground);
		line-height: 1.4;
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

	/* ── Approve Banner ─────────────────────────────────── */

	.approve-banner {
		margin: 0.625rem 0.875rem 0;
		padding: 0.5rem 0.75rem;
		border-radius: var(--radius-sm, 0.375rem);
		font-size: 0.8125rem;
		line-height: 1.4;
		border: 1px solid;
	}

	.approve-banner--error {
		background: oklch(from var(--destructive) l c h / 0.1);
		border-color: oklch(from var(--destructive) l c h / 0.3);
		color: var(--destructive);
	}

	.approve-banner--success {
		background: oklch(0.65 0.18 142 / 0.1);
		border-color: oklch(0.65 0.18 142 / 0.3);
		color: oklch(0.45 0.18 142);
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
