<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { goto } from '$app/navigation';
	import { subscribeToAllJobs, type AllJobsState } from '$lib/hooks';
	import { JobsDataTable } from '$lib/components/jobs';
	import { Badge } from '$lib/components/ui/badge';
	import { Inbox, ListChecks } from '@lucide/svelte';
	import AdminLoadingState from '../AdminLoadingState.svelte';
	import AdminErrorState from '../AdminErrorState.svelte';
	import type { Job } from '../../../../types/Job';

	let jobsState: AllJobsState = $state({ loading: true, jobs: [] });
	let unsubscribe: (() => void) | undefined;

	const jobCount = $derived(jobsState.jobs.length);

	function handleViewDetails(job: Job & { id: string }) {
		goto(`/job/${encodeURIComponent(job.id)}`);
	}

	onMount(() => {
		unsubscribe = subscribeToAllJobs((newState) => {
			jobsState = newState;
		});
	});

	onDestroy(() => {
		unsubscribe?.();
	});
</script>

<svelte:head>
	<title>Job Overview - Admin - iGait</title>
</svelte:head>

<div class="jobs-page">
	<!-- Header -->
	<div class="jobs-header">
		<ListChecks class="header-icon" />
		<h2 class="header-title">All Submissions</h2>
		{#if !jobsState.loading}
			<Badge variant="secondary" class="jobs-count-badge">{jobCount} total</Badge>
		{/if}
	</div>

	<!-- Main Content Card -->
	<div class="main-content-card">
		{#if jobsState.loading}
			<AdminLoadingState message="Loading jobs..." />
		{:else if jobsState.error}
			<AdminErrorState message={jobsState.error} />
		{:else if jobsState.jobs.length === 0}
			<div class="empty-state">
				<Inbox class="empty-icon" />
				<p class="empty-title">No jobs found</p>
				<p class="empty-description">
					There are no submissions in the system yet.
				</p>
			</div>
		{:else}
			<div class="table-area">
				<JobsDataTable data={jobsState.jobs} uid="" showEmail={true} onRowClick={handleViewDetails} />
			</div>
		{/if}
	</div>
</div>

<style>
	.jobs-page {
		display: flex;
		flex-direction: column;
		gap: 1.25rem;
	}

	/* ── Header ─────────────────────────────────────────── */

	.jobs-header {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 0.625rem;
	}

	:global(.header-icon) {
		width: 1.125rem;
		height: 1.125rem;
		color: var(--primary);
	}

	.header-title {
		font-size: 1.125rem;
		font-weight: 600;
		margin: 0;
	}

	:global(.jobs-count-badge) {
		font-size: 0.6875rem !important;
	}

	/* ── Main Content Card ────────────────────────────────── */

	.main-content-card {
		border: 1px solid var(--border);
		border-radius: var(--radius-md);
		background: var(--card);
		overflow: hidden;
		min-height: 300px;
	}

	/* ── Table Area ─────────────────────────────────────── */

	.table-area {
		padding: 0.75rem;
	}

	/* ── Empty State ────────────────────────────────────── */

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
