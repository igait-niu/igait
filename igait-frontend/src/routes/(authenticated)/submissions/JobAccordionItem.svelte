<script lang="ts">
	import * as Accordion from '$lib/components/ui/accordion';
	import { Badge } from '$lib/components/ui/badge';
	import { Separator } from '$lib/components/ui/separator';
	import {
		FileVideo,
		CheckCircle2,
		Clock,
		XCircle,
		Activity,
		User as UserIcon,
		Calendar
	} from '@lucide/svelte';
	import type { Job } from '../../../types/Job';
	import type { JobStatus } from '../../../types/JobStatus';

	interface Props {
		job: Job;
		index: number;
		totalJobs: number;
	}

	let { job, index, totalJobs }: Props = $props();

	// Computed values
	const submissionNumber = $derived(totalJobs - index);
	const formattedDate = $derived(formatDate(job.timestamp));
	const statusVariant = $derived(getStatusVariant(job.status.code));
	const statusText = $derived(getStatusText(job.status.code));
	const StatusIcon = $derived(getStatusIcon(job.status.code));

	function formatDate(timestamp: number): string {
		const date = new Date(timestamp * 1000);
		return date.toLocaleDateString('en-US', {
			year: 'numeric',
			month: 'short',
			day: 'numeric',
			hour: '2-digit',
			minute: '2-digit'
		});
	}

	function getStatusVariant(
		code: JobStatus['code']
	): 'default' | 'secondary' | 'destructive' | 'outline' {
		switch (code) {
			case 'Complete':
				return 'default';
			case 'Processing':
				return 'secondary';
			case 'Error':
				return 'destructive';
			case 'Submitted':
			default:
				return 'outline';
		}
	}

	function getStatusIcon(code: JobStatus['code']) {
		switch (code) {
			case 'Complete':
				return CheckCircle2;
			case 'Processing':
				return Activity;
			case 'Error':
				return XCircle;
			case 'Submitted':
			default:
				return Clock;
		}
	}

	function getStatusText(code: JobStatus['code']): string {
		switch (code) {
			case 'Complete':
				return 'Completed';
			case 'Processing':
				return 'Processing';
			case 'Error':
				return 'Error';
			case 'Submitted':
				return 'Submitted';
			default:
				return 'Unknown';
		}
	}

	function formatSex(sex: string): string {
		if (sex === 'M') return 'Male';
		if (sex === 'F') return 'Female';
		return sex;
	}
</script>

<Accordion.Item value={`job-${index}`} class="job-item">
	<Accordion.Trigger class="job-trigger">
		<div class="job-header">
			<div class="job-header__info">
				<div class="job-header__icon">
					<FileVideo class="icon" />
				</div>
				<div class="job-header__text">
					<p class="job-header__title">Submission #{submissionNumber}</p>
					<p class="job-header__date">{formattedDate}</p>
				</div>
			</div>
			<Badge variant={statusVariant}>
				<StatusIcon class="badge-icon" />
				{statusText}
			</Badge>
		</div>
	</Accordion.Trigger>
	<Accordion.Content>
		<div class="job-details">
			<div class="job-details__card">
				<!-- Status Details -->
				<div class="job-section">
					<h4 class="job-section__title">
						<Activity class="section-icon" />
						Status Details
					</h4>
					<p class="job-section__value">{job.status.value}</p>
				</div>

				<Separator />

				<!-- Patient Information -->
				<div class="job-section">
					<h4 class="job-section__title">
						<UserIcon class="section-icon" />
						Patient Information
					</h4>
					<div class="patient-grid">
						<div class="patient-field">
							<p class="patient-field__label">Age</p>
							<p class="patient-field__value">{job.age} years</p>
						</div>
						<div class="patient-field">
							<p class="patient-field__label">Sex</p>
							<p class="patient-field__value">{formatSex(job.sex)}</p>
						</div>
						<div class="patient-field">
							<p class="patient-field__label">Ethnicity</p>
							<p class="patient-field__value">{job.ethnicity}</p>
						</div>
						<div class="patient-field">
							<p class="patient-field__label">Height</p>
							<p class="patient-field__value">{job.height}</p>
						</div>
						<div class="patient-field">
							<p class="patient-field__label">Weight</p>
							<p class="patient-field__value">{job.weight} lbs</p>
						</div>
						<div class="patient-field">
							<p class="patient-field__label">Contact</p>
							<p class="patient-field__value patient-field__value--truncate" title={job.email}>
								{job.email}
							</p>
						</div>
					</div>
				</div>

				<!-- Timestamp -->
				<div class="job-timestamp">
					<Calendar class="timestamp-icon" />
					Submitted on {formattedDate}
				</div>
			</div>
		</div>
	</Accordion.Content>
</Accordion.Item>

<style>
	:global(.job-item) {
		border-bottom: 1px solid hsl(var(--border));
	}

	:global(.job-item:last-child) {
		border-bottom: none;
	}

	:global(.job-trigger) {
		padding-left: 1rem;
		padding-right: 1rem;
	}

	:global(.job-trigger:hover) {
		text-decoration: none;
	}

	.job-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		width: 100%;
		padding-right: 1rem;
	}

	.job-header__info {
		display: flex;
		align-items: center;
		gap: 0.75rem;
	}

	.job-header__icon {
		display: flex;
		height: 2.5rem;
		width: 2.5rem;
		align-items: center;
		justify-content: center;
		border-radius: 9999px;
		background-color: hsl(var(--muted));
	}

	.job-header__icon :global(.icon) {
		height: 1.25rem;
		width: 1.25rem;
		color: hsl(var(--muted-foreground));
	}

	.job-header__text {
		text-align: left;
	}

	.job-header__title {
		font-weight: 500;
	}

	.job-header__date {
		font-size: 0.875rem;
		color: hsl(var(--muted-foreground));
	}

	:global(.badge-icon) {
		margin-right: 0.25rem;
		height: 0.75rem;
		width: 0.75rem;
	}

	.job-details {
		padding: 0.5rem 1rem 1rem;
	}

	.job-details__card {
		border-radius: var(--radius-lg);
		background-color: hsl(var(--muted) / 0.5);
		padding: 1rem;
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}

	.job-section__title {
		font-size: 0.875rem;
		font-weight: 500;
		margin-bottom: 0.5rem;
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}

	.job-section__title :global(.section-icon) {
		height: 1rem;
		width: 1rem;
	}

	.job-section__value {
		font-size: 0.875rem;
		color: hsl(var(--muted-foreground));
	}

	.patient-grid {
		display: grid;
		grid-template-columns: repeat(2, 1fr);
		gap: 0.75rem;
	}

	@media (min-width: 640px) {
		.patient-grid {
			grid-template-columns: repeat(3, 1fr);
		}
	}

	.patient-field {
		border-radius: var(--radius-md);
		background-color: hsl(var(--background));
		padding: 0.5rem;
	}

	.patient-field__label {
		font-size: 0.75rem;
		color: hsl(var(--muted-foreground));
	}

	.patient-field__value {
		font-size: 0.875rem;
		font-weight: 500;
	}

	.patient-field__value--truncate {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.job-timestamp {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		font-size: 0.75rem;
		color: hsl(var(--muted-foreground));
	}

	.job-timestamp :global(.timestamp-icon) {
		height: 0.75rem;
		width: 0.75rem;
	}
</style>
