<script lang="ts">
	import { FileVideo, Activity, Clock } from '@lucide/svelte';
	import { isJobsLoaded, type JobsState } from '$lib/hooks';
	import type { Job } from '../../../types/Job';

	type Props = {
		jobsState: JobsState;
	};

	let { jobsState }: Props = $props();

	const stats = $derived.by(() => {
		if (!isJobsLoaded(jobsState)) {
			return [
				{
					label: 'Total Submissions',
					value: '---',
					description: 'Loading...',
					icon: FileVideo,
					href: '/submissions'
				},
				{
					label: 'Analyses Complete',
					value: '---',
					description: 'Loading...',
					icon: Activity,
					href: '/submissions?filter=complete'
				},
				{
					label: 'In Progress',
					value: '---',
					description: 'Loading...',
					icon: Clock,
					href: '/submissions?filter=processing'
				}
			];
		}

		const jobs = jobsState.jobs;
		const totalSubmissions = jobs.length;
		const completedJobs = jobs.filter((job: Job) => job.status.code === 'Complete').length;
		const inProgressJobs = jobs.filter(
			(job: Job) => job.status.code === 'Processing' || job.status.code === 'Submitted'
		).length;

		return [
			{
				label: 'Total Submissions',
				value: totalSubmissions.toString(),
				description: 'All time submissions',
				icon: FileVideo,
				href: '/submissions'
			},
			{
				label: 'Analyses Complete',
				value: completedJobs.toString(),
				description: 'Successfully processed',
				icon: Activity,
				href: '/submissions?filter=completed'
			},
			{
				label: 'In Progress',
				value: inProgressJobs.toString(),
				description: 'Currently processing',
				icon: Clock,
				href: '/submissions?filter=processing'
			}
		];
	});
</script>

<section>
	<h2 class="section-heading">Your Activity</h2>
	<div class="stats-grid">
		{#each stats as stat (stat.label)}
			<a href={stat.href} class="stat-card">
				<div class="stat-top">
					<span class="stat-label">{stat.label}</span>
					<svelte:component this={stat.icon} class="stat-icon" />
				</div>
				<div class="stat-value">{stat.value}</div>
				<span class="stat-desc">{stat.description}</span>
			</a>
		{/each}
	</div>
</section>

<style>
	.section-heading {
		font-size: 1.125rem;
		font-weight: 600;
		margin-bottom: 0.75rem;
	}

	.stats-grid {
		display: grid;
		gap: 0.5rem;
		grid-template-columns: repeat(3, 1fr);
	}

	@media (max-width: 640px) {
		.stats-grid {
			grid-template-columns: 1fr;
		}
	}

	.stat-card {
		display: flex;
		flex-direction: column;
		padding: 1rem 1.25rem;
		border: 1px solid var(--border);
		border-radius: var(--radius-md);
		background: var(--card);
		text-decoration: none;
		color: inherit;
		transition: border-color 0.15s ease;
	}

	.stat-card:hover {
		border-color: oklch(from var(--primary) l c h / 0.4);
	}

	.stat-top {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin-bottom: 0.5rem;
	}

	.stat-label {
		font-size: 0.8125rem;
		font-weight: 500;
		color: var(--muted-foreground);
	}

	:global(.stat-icon) {
		width: 0.9375rem;
		height: 0.9375rem;
		color: var(--muted-foreground);
	}

	.stat-value {
		font-size: 1.75rem;
		font-weight: 700;
		line-height: 1;
		margin-bottom: 0.25rem;
	}

	.stat-desc {
		font-size: 0.6875rem;
		color: var(--muted-foreground);
	}
</style>
