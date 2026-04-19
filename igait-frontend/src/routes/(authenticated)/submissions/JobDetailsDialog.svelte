<script lang="ts">
	import * as Dialog from '$lib/components/ui/dialog';
	import { Badge } from '$lib/components/ui/badge';
	import { Separator } from '$lib/components/ui/separator';
	import { Button } from '$lib/components/ui/button';
	import * as Progress from '$lib/components/ui/progress';
	import {
		FileVideo,
		CheckCircle2,
		Clock,
		XCircle,
		User as UserIcon,
		Calendar,
		AlertTriangle
	} from '@lucide/svelte';
	import type { Job } from '../../../types/Job';
	import type { JobStatus } from '../../../types/JobStatus';
	import { registryStore, isRegistryLoaded } from '$lib/stores';

	type Props = {
		job: Job;
		onClose: () => void;
	};

	let { job, onClose }: Props = $props();

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
		status: JobStatus
	): 'default' | 'secondary' | 'destructive' | 'outline' {
		switch (status.code) {
			case 'Complete':
				return 'default';
			case 'Error':
				return 'destructive';
			case 'Processing':
				return 'secondary';
			case 'Submitted':
			default:
				return 'outline';
		}
	}

	function getStatusIcon(status: JobStatus) {
		switch (status.code) {
			case 'Complete':
				return CheckCircle2;
			case 'Error':
				return XCircle;
			case 'Processing':
				return Clock;
			case 'Submitted':
			default:
				return Clock;
		}
	}

	const statusVariant = $derived(getStatusVariant(job.status));
	const StatusIcon = $derived(getStatusIcon(job.status));
	const formattedDate = $derived(formatDate(job.timestamp));
	const isComplete = $derived(job.status.code === 'Complete');
	const isProcessing = $derived(job.status.code === 'Processing');
	const isError = $derived(job.status.code === 'Error');

	const registryState = $derived(registryStore.state);
	const stages = $derived(isRegistryLoaded(registryState) ? registryState.stages : []);

	// Processing progress: position of the active stage over total stage count.
	const processingProgress = $derived.by(() => {
		const status = job.status;
		if (status.code !== 'Processing' || stages.length === 0) return 0;
		const idx = stages.findIndex((s) => s.key === status.stage);
		if (idx < 0) return 0;
		return ((idx + 1) / stages.length) * 100;
	});

	const processingStageName = $derived.by(() => {
		const status = job.status;
		if (status.code !== 'Processing') return '';
		const spec = stages.find((s) => s.key === status.stage);
		return spec?.display_name ?? status.stage;
	});

	const processingStageIndex = $derived.by(() => {
		const status = job.status;
		if (status.code !== 'Processing') return -1;
		return stages.findIndex((s) => s.key === status.stage);
	});

	// Complete results
	const completeResult = $derived.by(() => {
		if (job.status.code === 'Complete') {
			return job.status;
		}
		return null;
	});
</script>

<Dialog.Root open={true} onOpenChange={onClose}>
	<Dialog.Content class="max-h-[80vh] overflow-y-auto sm:max-w-[600px]">
		<Dialog.Header>
			<Dialog.Title class="flex items-center gap-2">
				<FileVideo class="h-5 w-5" />
				Submission Details
			</Dialog.Title>
			<Dialog.Description>
				Complete information about this gait analysis submission
			</Dialog.Description>
		</Dialog.Header>

		<div class="space-y-6 py-4">
			<!-- Status Section -->
			<div class="space-y-3">
				{#if StatusIcon}
					{@const Icon = StatusIcon}
					<h3 class="flex items-center gap-2 text-sm font-medium">
						<Icon class="h-4 w-4" />
						Status
					</h3>
				{/if}
				<div class="flex items-center gap-2">
					<Badge variant={statusVariant} class="text-sm">
						{job.status.value}
					</Badge>
				</div>

				<!-- Processing Progress Bar -->
				{#if isProcessing && stages.length > 0 && processingStageIndex >= 0}
					<div class="space-y-2">
						<Progress.Root value={processingProgress} class="w-full" />
						<p class="text-xs text-muted-foreground">
							{processingStageName}
							({processingStageIndex + 1} of {stages.length})
						</p>
					</div>
				{/if}
			</div>

			<Separator />

			<!-- Submission Info -->
			<div class="space-y-3">
				<h3 class="flex items-center gap-2 text-sm font-medium">
					<Calendar class="h-4 w-4" />
					Submission Information
				</h3>
				<div class="grid gap-2 text-sm">
					<div class="flex justify-between">
						<span class="text-muted-foreground">Submitted:</span>
						<span class="font-medium">{formattedDate}</span>
					</div>
					<div class="flex justify-between">
						<span class="text-muted-foreground">Email:</span>
						<span class="font-medium">{job.email}</span>
					</div>
				</div>
			</div>

			<Separator />

			<!-- Patient Information -->
			<div class="space-y-3">
				<h3 class="flex items-center gap-2 text-sm font-medium">
					<UserIcon class="h-4 w-4" />
					Patient Information
				</h3>
				<div class="grid grid-cols-2 gap-3 text-sm">
					<div>
						<span class="mb-1 block text-muted-foreground">Age</span>
						<span class="font-medium">{job.age} years</span>
					</div>
					<div>
						<span class="mb-1 block text-muted-foreground">Sex</span>
						<span class="font-medium">{job.sex}</span>
					</div>
					<div>
						<span class="mb-1 block text-muted-foreground">Height</span>
						<span class="font-medium">{job.height}</span>
					</div>
					<div>
						<span class="mb-1 block text-muted-foreground">Weight</span>
						<span class="font-medium">{job.weight} lbs</span>
					</div>
					<div class="col-span-2">
						<span class="mb-1 block text-muted-foreground">Ethnicity</span>
						<span class="font-medium">{job.ethnicity}</span>
					</div>
				</div>
			</div>

			{#if isComplete && completeResult}
				<Separator />

				<!-- Results Section -->
				<div class="space-y-3">
					<h3 class="flex items-center gap-2 text-sm font-medium">
						<CheckCircle2 class="h-4 w-4" />
						Results
					</h3>
					<div class="rounded-lg bg-muted p-3 text-sm">
						{completeResult.asd
							? 'Submitted videos show walking pattern more similar to autistic children than non-autistic children.'
							: 'Submitted videos show walking pattern more similar to non-autistic children than autistic children.'}
					</div>
				</div>
			{/if}

			{#if isError}
				<Separator />

				<!-- Error Section (user-friendly) -->
				<div class="space-y-3">
					<h3 class="flex items-center gap-2 text-sm font-medium text-destructive">
						<AlertTriangle class="h-4 w-4" />
						Analysis Failed
					</h3>
					<div
						class="rounded-lg border border-destructive/20 bg-destructive/10 p-3 text-sm text-destructive"
					>
						Something went wrong processing your submission. Please contact GaitStudy@niu.edu for
						assistance.
					</div>
				</div>
			{/if}
		</div>

		<Dialog.Footer>
			<Button variant="outline" onclick={onClose}>Close</Button>
		</Dialog.Footer>
	</Dialog.Content>
</Dialog.Root>

<style>
	:global(.max-h-\[80vh\]) {
		max-height: 80vh;
	}
</style>
