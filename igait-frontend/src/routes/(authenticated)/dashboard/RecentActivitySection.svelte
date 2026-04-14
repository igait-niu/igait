<script lang="ts">
	import { Badge } from '$lib/components/ui/badge';
	import { Activity, Upload, FileVideo, Loader2, ArrowRight } from '@lucide/svelte';
	import { isJobsLoaded, isJobsLoading, type JobsState, type JobWithId } from '$lib/hooks';

	type Props = {
		jobsState: JobsState;
	};

	let { jobsState }: Props = $props();

	const recentActivity = $derived.by(() => {
		if (!isJobsLoaded(jobsState)) {
			return [];
		}

		return jobsState.jobs.slice(0, 3).map((job: JobWithId) => {
			const date = new Date(job.timestamp * 1000);
			const isCompleted = job.status.code === 'Complete';
			const isError = job.status.code.includes('Error') || job.status.code.includes('Failed');

			return {
				id: job.id,
				href: `/job/${encodeURIComponent(job.id)}`,
				status: isCompleted ? 'completed' : isError ? 'error' : 'processing',
				date: date.toLocaleDateString('en-US', {
					month: 'short',
					day: 'numeric',
					year: 'numeric'
				}),
				description: job.status.value
			};
		});
	});
</script>

<section>
	<div class="section-top">
		<h2 class="section-heading">Recent Activity</h2>
		<a href="/submissions" class="view-all">
			View All
			<ArrowRight class="view-all-arrow" />
		</a>
	</div>

	<div class="activity-card">
		{#if isJobsLoading(jobsState)}
			<div class="empty-state">
				<Loader2 class="empty-icon animate-spin" />
				<p class="empty-title">Loading activity...</p>
				<p class="empty-desc">Fetching your recent submissions</p>
			</div>
		{:else if recentActivity.length === 0}
			<div class="empty-state">
				<Activity class="empty-icon" />
				<p class="empty-title">No activity yet</p>
				<p class="empty-desc">Submit your first walking video to get started!</p>
				<a href="/submit" class="empty-cta">
					<Upload class="empty-cta-icon" />
					New Submission
				</a>
			</div>
		{:else}
			{#each recentActivity as activity (activity.id)}
				<a href={activity.href} class="activity-row">
					<div class="activity-left">
						<div class="activity-icon-wrap">
							<FileVideo class="activity-icon" />
						</div>
						<div>
							<p class="activity-title">{activity.description}</p>
							<p class="activity-date">{activity.date}</p>
						</div>
					</div>
					<Badge
						variant={activity.status === 'completed'
							? 'default'
							: activity.status === 'error'
								? 'destructive'
								: 'secondary'}
					>
						{activity.status === 'completed'
							? 'Complete'
							: activity.status === 'error'
								? 'Error'
								: 'Processing'}
					</Badge>
				</a>
			{/each}
		{/if}
	</div>
</section>

<style>
	.section-top {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin-bottom: 0.75rem;
	}

	.section-heading {
		font-size: 1.125rem;
		font-weight: 600;
		margin: 0;
	}

	.view-all {
		display: flex;
		align-items: center;
		gap: 0.25rem;
		font-size: 0.8125rem;
		font-weight: 500;
		color: var(--muted-foreground);
		text-decoration: none;
		transition: color 0.15s ease;
	}

	.view-all:hover {
		color: var(--foreground);
	}

	:global(.view-all-arrow) {
		width: 0.75rem;
		height: 0.75rem;
	}

	.activity-card {
		border: 1px solid var(--border);
		border-radius: var(--radius-md);
		background: var(--card);
		overflow: hidden;
	}

	.activity-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 0.75rem 1rem;
		border-bottom: 1px solid var(--border);
		text-decoration: none;
		color: inherit;
		transition: background-color 0.15s ease;
	}

	.activity-row:hover {
		background-color: oklch(from var(--muted) l c h / 0.4);
	}

	.activity-row:last-child {
		border-bottom: none;
	}

	.activity-left {
		display: flex;
		align-items: center;
		gap: 0.75rem;
	}

	.activity-icon-wrap {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 2rem;
		height: 2rem;
		border-radius: var(--radius-sm);
		background: oklch(from var(--muted) l c h / 0.5);
	}

	:global(.activity-icon) {
		width: 0.875rem;
		height: 0.875rem;
		color: var(--muted-foreground);
	}

	.activity-title {
		font-size: 0.8125rem;
		font-weight: 500;
		margin: 0;
	}

	.activity-date {
		font-size: 0.6875rem;
		color: var(--muted-foreground);
		margin: 0;
	}

	/* ── Empty state ─────────────────────────────── */

	.empty-state {
		display: flex;
		flex-direction: column;
		align-items: center;
		padding: 2.5rem 1.5rem;
		text-align: center;
	}

	:global(.empty-icon) {
		width: 2rem;
		height: 2rem;
		color: oklch(from var(--muted-foreground) l c h / 0.4);
		margin-bottom: 0.75rem;
	}

	.empty-title {
		font-size: 0.875rem;
		font-weight: 600;
		margin: 0 0 0.25rem;
	}

	.empty-desc {
		font-size: 0.8125rem;
		color: var(--muted-foreground);
		margin: 0 0 1rem;
	}

	.empty-cta {
		display: inline-flex;
		align-items: center;
		gap: 0.375rem;
		font-size: 0.8125rem;
		font-weight: 500;
		color: var(--primary);
		text-decoration: none;
	}

	.empty-cta:hover {
		text-decoration: underline;
	}

	:global(.empty-cta-icon) {
		width: 0.875rem;
		height: 0.875rem;
	}
</style>
