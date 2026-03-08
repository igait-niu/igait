<script lang="ts">
	import { onDestroy } from 'svelte';
	import { page } from '$app/stores';
	import { getUser, subscribeToJob, type SingleJobState } from '$lib/hooks';
	import { rerunJob } from '$lib/api';
	import { getJobFiles } from '$lib/api';
	import type { FileEntry, JobFilesResponse } from '$lib/api';
	import { FileViewer } from '$lib/components';
	import CycleEditor from '$lib/components/CycleEditor.svelte';
	import VideoEditor from '$lib/components/VideoEditor.svelte';
	import { Badge } from '$lib/components/ui/badge';
	import { Button } from '$lib/components/ui/button';
	import { Separator } from '$lib/components/ui/separator';
	import * as Dialog from '$lib/components/ui/dialog';
	import {
		ArrowLeft,
		RotateCcw,
		AlertTriangle,
		ScrollText,
		FileOutput,
		User as UserIcon,
		Calendar,
		CheckCircle2,
		XCircle,
		Clock,
		ShieldCheck,
		Loader2,
		AlertCircle,
		Film
	} from '@lucide/svelte';
	import type { Job } from '../../../../types/Job';
	import type { JobStatus } from '../../../../types/JobStatus';
	import type { StageStatus } from '../../../../types/StageStatus';

	// ── Auth ──────────────────────────────────────────────
	const user = getUser();
	const isAdmin = $derived(user.administrator);

	// ── Route param ────────────────────────────────────────
	const jobId = $derived($page.params.id as string);

	// ── State ──────────────────────────────────────────────
	let jobState = $state<SingleJobState>({ status: 'loading' });
	let activeStage: string = $state('stage_1');
	let activeSubTab: 'output' | 'logs' = $state('output');
	let showRerunDialog = $state(false);
	let rerunLoading = $state(false);
	let rerunError: string | null = $state(null);
	let rerunSuccess: string | null = $state(null);
	let cycleEditorOpen = $state(false);
	let videoEditorOpen = $state(false);

	// ── Files state ───────────────────────────────────────
	let filesLoading = $state(false);
	let filesError: string | null = $state(null);
	let filesData: JobFilesResponse | null = $state(null);

	// ── Subscription ───────────────────────────────────────
	let unsubscribe: (() => void) | undefined;

	$effect(() => {
		unsubscribe?.();

		if (jobId) {
			unsubscribe = subscribeToJob(jobId, (state) => {
				jobState = state;
			});
		}
	});

	onDestroy(() => {
		unsubscribe?.();
	});

	// ── Fetch files ──────────────────────────────────────
	async function loadFiles() {
		if (!jobId) return;
		filesLoading = true;
		filesError = null;

		const result = await getJobFiles(jobId);
		if (result.isOk()) {
			filesData = result.value;
		} else {
			filesError = result.error.rootCause;
		}
		filesLoading = false;
	}

	$effect(() => {
		if (jobId) {
			loadFiles();
		}
	});

	// ── Stage info ─────────────────────────────────────────
	const stageInfo = [
		{ key: 'stage_1', name: 'Stage 1', description: 'Media Conversion' },
		{ key: 'stage_2', name: 'Stage 2', description: 'Validity Check' },
		{ key: 'stage_3', name: 'Stage 3', description: 'Reframing' },
		{ key: 'stage_4', name: 'Stage 4', description: 'Pose Estimation' },
		{ key: 'stage_5', name: 'Stage 5', description: 'Cycle Detection' },
		{ key: 'stage_6', name: 'Stage 6', description: 'ML Prediction' },
		{ key: 'stage_7', name: 'Stage 7', description: 'Finalize' }
	] as const;

	// ── Derived ────────────────────────────────────────────
	const job = $derived(jobState.status === 'loaded' ? jobState.job : null);

	const activeStageInfo = $derived(stageInfo.find((s) => s.key === activeStage) ?? stageInfo[0]);

	const activeStageNumber = $derived(parseInt(activeStage.replace('stage_', ''), 10));

	const currentStageLogs = $derived.by(() => {
		if (!job?.stage_logs) return null;
		return job.stage_logs[activeStage] ?? null;
	});

	const jobIndex = $derived.by(() => {
		const lastUnderscore = jobId.lastIndexOf('_');
		if (lastUnderscore === -1) return 0;
		return parseInt(jobId.slice(lastUnderscore + 1), 10);
	});

	// ── Files for active stage ──────────────────────────
	const outputFiles = $derived.by((): FileEntry[] | undefined => {
		if (!filesData) return undefined;
		const outputStageKey = `stage_${activeStageNumber}`;
		return filesData.stages[outputStageKey] ?? [];
	});

	// ── Tab counts ──────────────────────────────────────
	const outputFileCount = $derived(outputFiles?.length ?? 0);
	const logLineCount = $derived.by(() => {
		if (!currentStageLogs) return 0;
		return currentStageLogs.split('\n').length;
	});

	// ── Cycle editor file lookups ────────────────────────
	const stage1Files = $derived((filesData as JobFilesResponse | null)?.stages['stage_1'] ?? []);

	const stage1FrontVideo = $derived(
		stage1Files.find((f: FileEntry) => f.name.startsWith('front') && f.name.endsWith('.mp4')) ?? null
	);
	const stage1SideVideo = $derived(
		stage1Files.find((f: FileEntry) => f.name.startsWith('side') && f.name.endsWith('.mp4')) ?? null
	);
	const canOpenVideoEditor = $derived(!!(stage1FrontVideo || stage1SideVideo));

	const stage4Files = $derived((filesData as JobFilesResponse | null)?.stages['stage_4'] ?? []);
	const stage5Files = $derived((filesData as JobFilesResponse | null)?.stages['stage_5'] ?? []);

	const frontVideoFile = $derived(
		stage4Files.find((f: FileEntry) => f.name.startsWith('front') && f.name.endsWith('.mp4')) ?? null
	);
	const sideVideoFile = $derived(
		stage4Files.find((f: FileEntry) => f.name.startsWith('side') && f.name.endsWith('.mp4')) ?? null
	);
	const frontJsonFile = $derived(
		stage5Files.find((f: FileEntry) => f.name === 'front_gait_analysis.json') ?? null
	);
	const sideJsonFile = $derived(
		stage5Files.find((f: FileEntry) => f.name === 'side_gait_analysis.json') ?? null
	);
	const canOpenCycleEditor = $derived(
		!!(frontVideoFile || sideVideoFile) && !!(frontJsonFile || sideJsonFile)
	);

	// ── Stage status helpers ────────────────────────────────
	function getStageStatus(stageKey: string): StageStatus {
		return job?.stage_statuses?.[stageKey] ?? 'not_started';
	}

	const activeStageStatus = $derived(getStageStatus(activeStage));
	const stageHasContent = $derived(activeStageStatus !== 'not_started');

	// ── Status helpers ─────────────────────────────────────
	function getStatusLabel(status: JobStatus): string {
		switch (status.code) {
			case 'Complete':
				return status.asd ? 'ASD Detected' : 'No ASD';
			case 'Error':
				return 'Error';
			case 'Processing':
				return `Stage ${status.stage}/${status.num_stages}`;
			case 'Submitted':
				return 'Submitted';
		}
	}

	function getStatusVariant(
		status: JobStatus
	): 'default' | 'secondary' | 'destructive' | 'outline' {
		switch (status.code) {
			case 'Complete':
				return status.asd ? 'destructive' : 'default';
			case 'Error':
				return 'destructive';
			case 'Processing':
				return 'secondary';
			case 'Submitted':
				return 'outline';
		}
	}

	function formatJobId(id: string): string {
		const lastUnderscore = id.lastIndexOf('_');
		if (lastUnderscore === -1) return id;
		const uid = id.slice(0, lastUnderscore);
		const index = id.slice(lastUnderscore + 1);
		return `${uid.slice(0, 8)}…#${index}`;
	}

	function formatDate(timestamp: number): string {
		return new Date(timestamp * 1000).toLocaleDateString('en-US', {
			year: 'numeric',
			month: 'short',
			day: 'numeric',
			hour: '2-digit',
			minute: '2-digit'
		});
	}

	// ── Handlers ───────────────────────────────────────────
	function handleBack() {
		history.back();
	}

	function handleStageClick(stageKey: string) {
		activeStage = stageKey;
	}

	function handleSubTabClick(tab: 'output' | 'logs') {
		activeSubTab = tab;
	}

	function handleRerunClick() {
		rerunError = null;
		rerunSuccess = null;
		showRerunDialog = true;
	}

	async function handleRerunConfirm() {
		rerunLoading = true;
		rerunError = null;

		try {
			const result = await rerunJob(jobId, activeStageNumber);

			if (result.isOk()) {
				rerunSuccess = result.value.message;
				showRerunDialog = false;
			} else {
				rerunError = result.error.rootCause;
			}
		} catch (err) {
			rerunError = err instanceof Error ? err.message : 'An unexpected error occurred.';
		} finally {
			rerunLoading = false;
		}
	}
</script>

<svelte:head>
	<title>Job {formatJobId(jobId)} - iGait</title>
</svelte:head>

{#if jobState.status === 'loading'}
	<div class="state-message">
		<Loader2 class="spinner" />
		<p>Loading job details...</p>
	</div>
{:else if jobState.status === 'error'}
	<div class="state-message">
		<AlertCircle class="error-icon" />
		<p>{jobState.error}</p>
	</div>
{:else if job}
	<div class="job-detail-page">
		<!-- Header -->
		<header class="detail-header">
			<Button variant="ghost" size="sm" class="back-btn" onclick={handleBack}>
				<ArrowLeft class="h-4 w-4" />
				Back
			</Button>
			<div class="header-info">
				<h2 class="header-title">
					Job <span class="mono">{jobId}</span>
				</h2>
				<Badge variant={getStatusVariant(job.status)}>{getStatusLabel(job.status)}</Badge>
			</div>
		</header>

		<!-- Job Details Card -->
		<div class="details-card">
			<div class="details-grid">
				<!-- Submission Info -->
				<div class="detail-section">
					<h4 class="section-title">
						<Calendar class="section-icon" />
						Submission
					</h4>

					<div class="detail-row">
						<span class="detail-label">Email</span>
						<span class="detail-value">{job.email}</span>
					</div>
					<div class="detail-row">
						<span class="detail-label">Submitted</span>
						<span class="detail-value">{formatDate(job.timestamp)}</span>
					</div>
					<div class="detail-row">
						<span class="detail-label">Status</span>
						<span class="detail-value">{job.status.value}</span>
					</div>
					<div class="detail-row">
						<span class="detail-label">Approval</span>
						<Badge variant={job.approved ? 'default' : 'outline'} class="detail-badge">
							{job.approved ? 'Approved' : job.requires_approval ? 'Pending' : 'Auto'}
						</Badge>
					</div>
				</div>

				<Separator class="details-separator details-separator--vertical" orientation="vertical" />
				<Separator
					class="details-separator details-separator--horizontal"
					orientation="horizontal"
				/>

				<!-- Patient Info -->
				<div class="detail-section">
					<h4 class="section-title">
						<UserIcon class="section-icon" />
						Patient
					</h4>
					<div class="detail-row">
						<span class="detail-label">Age</span>
						<span class="detail-value">{job.age} years</span>
					</div>
					<div class="detail-row">
						<span class="detail-label">Sex</span>
						<span class="detail-value">{job.sex}</span>
					</div>
					<div class="detail-row">
						<span class="detail-label">Height</span>
						<span class="detail-value">{job.height}</span>
					</div>
					<div class="detail-row">
						<span class="detail-label">Weight</span>
						<span class="detail-value">{job.weight} lbs</span>
					</div>
					<div class="detail-row">
						<span class="detail-label">Ethnicity</span>
						<span class="detail-value">{job.ethnicity}</span>
					</div>
				</div>

				<!-- Results (if complete) -->
				{#if job.status.code === 'Complete'}
					<Separator class="details-separator details-separator--vertical" orientation="vertical" />
					<Separator
						class="details-separator details-separator--horizontal"
						orientation="horizontal"
					/>
					<div class="detail-section">
						<h4 class="section-title">
							<CheckCircle2 class="section-icon" />
							Results
						</h4>
						<div class="detail-row">
							<span class="detail-label">ASD Detection</span>
							<Badge variant={job.status.asd ? 'destructive' : 'default'} class="detail-badge">
								{job.status.asd ? 'ASD Indicators Detected' : 'No ASD Indicators'}
							</Badge>
						</div>
						<div class="detail-row">
							<span class="detail-label">Confidence</span>
							<span class="detail-value">
								{job.status.asd
									? (job.status.prediction * 100).toFixed(1)
									: ((1 - job.status.prediction) * 100).toFixed(1)}%
							</span>
						</div>
					</div>
				{/if}

				<!-- Error (if failed) -->
				{#if job.status.code === 'Error'}
					<Separator class="details-separator details-separator--vertical" orientation="vertical" />
					<Separator
						class="details-separator details-separator--horizontal"
						orientation="horizontal"
					/>
					<div class="detail-section">
						<h4 class="section-title section-title--error">
							<XCircle class="section-icon" />
							Error
						</h4>
						<pre class="error-preview">{job.status.logs}</pre>
					</div>
				{/if}
			</div>
		</div>

		<!-- Stage Tabs (admin only) -->
		{#if isAdmin}
			<div class="stage-tabs-container">
				<div class="stage-tabs">
					{#each stageInfo as stage}
						{@const status = getStageStatus(stage.key)}
						<button
							class="stage-tab stage-tab--{status}"
							class:active={activeStage === stage.key}
							onclick={() => handleStageClick(stage.key)}
						>
							<span class="tab-header">
								{#if status === 'complete'}
									<CheckCircle2 class="tab-status-icon tab-status-icon--complete" />
								{:else if status === 'running'}
									<Loader2 class="tab-status-icon tab-status-icon--running" />
								{:else if status === 'error'}
									<XCircle class="tab-status-icon tab-status-icon--error" />
								{:else}
									<Clock class="tab-status-icon tab-status-icon--not-started" />
								{/if}
								<span class="tab-name">{stage.name}</span>
							</span>
							<span class="tab-desc">{stage.description}</span>
						</button>
					{/each}
				</div>
			</div>

			<!-- Main content card -->
			<div class="main-content-card">
				{#if stageHasContent}
					<!-- Sub-tabs + Re-Run row -->
					<div class="sub-tab-row">
						<div class="sub-tabs">
							<button
								class="sub-tab"
								class:active={activeSubTab === 'output'}
								onclick={() => handleSubTabClick('output')}
							>
								<FileOutput class="sub-tab-icon" />
								Output Files
								{#if !filesLoading}
									<Badge variant="outline" class="sub-tab-badge {outputFileCount === 0 ? 'sub-tab-badge-zero' : ''}">{outputFileCount}</Badge>
								{/if}
							</button>
							<button
								class="sub-tab"
								class:active={activeSubTab === 'logs'}
								onclick={() => handleSubTabClick('logs')}
							>
								<ScrollText class="sub-tab-icon" />
								Logs
								<Badge variant="outline" class="sub-tab-badge {logLineCount === 0 ? 'sub-tab-badge-zero' : ''}">{logLineCount}</Badge>
							</button>
						</div>

						<div class="sub-tab-actions">
							{#if canOpenVideoEditor}
								<Button variant="outline" size="sm" onclick={() => (videoEditorOpen = true)}>
									<Film class="mr-1 h-4 w-4" />
									Video Editor
								</Button>
							{:else}
								<Button variant="outline" size="sm" disabled title="Stage 1 outputs must exist to use the Video Editor">
									<Film class="mr-1 h-4 w-4" />
									Video Editor
								</Button>
							{/if}
							{#if canOpenCycleEditor}
								<Button variant="outline" size="sm" onclick={() => (cycleEditorOpen = true)}>
									<Film class="mr-1 h-4 w-4" />
									Cycle Editor
								</Button>
							{/if}
							<Button variant="destructive" size="sm" onclick={handleRerunClick}>
								<RotateCcw class="mr-1 h-4 w-4" />
								Re-Run
							</Button>
						</div>
					</div>

					<!-- Success banner -->
					{#if rerunSuccess}
						<div class="success-banner">
							{rerunSuccess}
						</div>
					{/if}

					<!-- Tab Content -->
					<div class="tab-content">
					{#if activeSubTab === 'output'}
						<FileViewer
							files={outputFiles}
							loading={filesLoading}
							error={filesError}
							label=""
							stageNumber={activeStageNumber}
							allFiles={filesData}
							{isAdmin}
							{jobId}
						/>
					{:else if activeSubTab === 'logs'}
						<div class="logs-content">
							{#if currentStageLogs}
								<pre class="log-output">{currentStageLogs}</pre>
							{/if}
						</div>
					{/if}
					</div>
				{:else}
					<div class="stage-not-started">
						<Clock class="not-started-icon" />
						<p class="not-started-title">{activeStageInfo.name}: {activeStageInfo.description}</p>
						<p class="not-started-subtitle">This stage hasn't started yet</p>
						<Button variant="destructive" size="sm" onclick={handleRerunClick}>
							<RotateCcw class="mr-1 h-4 w-4" />
							Re-Run from here
						</Button>
					</div>
				{/if}
			</div>
		{/if}
	</div>

	<!-- Re-Run Warning Dialog -->
	<Dialog.Root bind:open={showRerunDialog}>
		<Dialog.Content class="sm:max-w-[480px]">
			<Dialog.Header>
				<Dialog.Title class="flex items-center gap-2 text-destructive">
					<AlertTriangle class="h-5 w-5" />
					Confirm Re-Run
				</Dialog.Title>
				<Dialog.Description>This action cannot be undone.</Dialog.Description>
			</Dialog.Header>

			<div class="rerun-warning-body">
				<p>
					You are about to re-run <strong>{formatJobId(jobId)}</strong> starting from
					<strong>{activeStageInfo.name} ({activeStageInfo.description})</strong>.
				</p>
				<div class="warning-callout">
					<AlertTriangle class="callout-icon" />
					<span>
						This will <strong>clear all outputs</strong> from Stage {activeStageNumber}
						onward (through Stage 7). The job will be re-queued for processing.
					</span>
				</div>

				{#if rerunError}
					<div class="rerun-error">
						{rerunError}
					</div>
				{/if}
			</div>

			<Dialog.Footer>
				<Button variant="outline" onclick={() => (showRerunDialog = false)} disabled={rerunLoading}>
					Cancel
				</Button>
				<Button variant="destructive" onclick={handleRerunConfirm} disabled={rerunLoading}>
					{#if rerunLoading}
						Re-Running…
					{:else}
						<RotateCcw class="mr-1 h-4 w-4" />
						Re-Run from Stage {activeStageNumber}
					{/if}
				</Button>
			</Dialog.Footer>
		</Dialog.Content>
	</Dialog.Root>

	<CycleEditor
		open={cycleEditorOpen}
		onclose={() => (cycleEditorOpen = false)}
		{jobId}
		frontVideo={frontVideoFile}
		sideVideo={sideVideoFile}
		{frontJsonFile}
		{sideJsonFile}
	/>

	<VideoEditor
		open={videoEditorOpen}
		onclose={() => (videoEditorOpen = false)}
		{jobId}
		frontVideo={stage1FrontVideo}
		sideVideo={stage1SideVideo}
	/>
{/if}

<style>
	.job-detail-page {
		display: flex;
		flex-direction: column;
		gap: 1.25rem;
	}

	/* ── Loading / Error states ─────────────────────────── */

	.state-message {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 0.75rem;
		padding: 2.5rem 1.5rem;
		color: hsl(var(--muted-foreground));
	}

	.state-message p {
		font-size: 0.8125rem;
		margin: 0;
		text-align: center;
	}

	:global(.spinner) {
		width: 1.5rem;
		height: 1.5rem;
		animation: spin 1s linear infinite;
	}

	:global(.error-icon) {
		width: 1.5rem;
		height: 1.5rem;
		color: hsl(var(--destructive));
	}

	@keyframes spin {
		from {
			transform: rotate(0deg);
		}
		to {
			transform: rotate(360deg);
		}
	}

	/* ── Header ─────────────────────────────────────────── */

	.detail-header {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	:global(.back-btn) {
		align-self: flex-start;
		margin-left: -0.5rem;
	}

	.header-info {
		display: flex;
		align-items: center;
		gap: 0.75rem;
	}

	.header-title {
		font-size: 1.125rem;
		font-weight: 600;
		margin: 0;
	}

	.mono {
		font-family: ui-monospace, monospace;
		font-size: 0.9375rem;
	}

	/* ── Details Card ──────────────────────────────────── */

	.details-card {
		background: hsl(var(--card));
		border: 1px solid hsl(var(--border));
		border-radius: var(--radius-md);
		padding: 1rem 1.25rem;
	}

	.details-grid {
		display: flex;
		gap: 1.25rem;
		flex-wrap: wrap;
	}

	.details-grid > .detail-section {
		flex: 1 1 220px;
		min-width: 220px;
	}

	:global(.details-separator--vertical) {
		display: block;
		align-self: stretch;
	}

	:global(.details-separator--horizontal) {
		display: none;
	}

	@media (max-width: 768px) {
		:global(.details-separator--vertical) {
			display: none;
		}

		:global(.details-separator--horizontal) {
			display: block;
		}
	}

	.detail-section {
		display: flex;
		flex-direction: column;
		gap: 0.375rem;
	}

	.section-title {
		font-size: 0.75rem;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.05em;
		color: hsl(var(--muted-foreground));
		margin: 0 0 0.25rem;
		display: flex;
		align-items: center;
		gap: 0.375rem;
	}

	.section-title--error {
		color: hsl(var(--destructive));
	}

	:global(.section-icon) {
		width: 0.8125rem;
		height: 0.8125rem;
	}

	.detail-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 0.5rem;
	}

	.detail-label {
		font-size: 0.8125rem;
		color: hsl(var(--muted-foreground));
	}

	.detail-value {
		font-size: 0.8125rem;
		font-weight: 500;
		text-align: right;
	}

	:global(.detail-badge) {
		font-size: 0.6875rem;
	}

	.error-preview {
		font-family: ui-monospace, monospace;
		font-size: 0.6875rem;
		line-height: 1.5;
		white-space: pre-wrap;
		word-break: break-word;
		background: hsl(var(--destructive) / 0.06);
		border: 1px solid hsl(var(--destructive) / 0.15);
		padding: 0.5rem 0.625rem;
		border-radius: var(--radius-sm);
		max-height: 80px;
		overflow-y: auto;
		margin: 0;
		color: hsl(var(--destructive));
	}

	/* ── Stage Tabs ─────────────────────────────────────── */

	.stage-tabs-container {
		background: hsl(var(--primary) / 0.04);
		border: 1px solid hsl(var(--primary) / 0.15);
		border-radius: var(--radius-md);
		padding: 0.5rem;
		overflow-x: auto;
	}

	.stage-tabs {
		display: flex;
		gap: 0.25rem;
		justify-content: center;
	}

	.stage-tab {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 0.125rem;
		padding: 0.5rem 1rem;
		border: 1px solid transparent;
		background: none;
		cursor: pointer;
		color: hsl(var(--muted-foreground));
		transition: all 0.15s ease;
		white-space: nowrap;
		border-radius: var(--radius-sm);
	}

	.stage-tab:hover {
		color: hsl(var(--foreground));
		background-color: hsl(var(--primary) / 0.06);
	}

	.stage-tab.active {
		background-color: hsl(var(--background));
		box-shadow: 0 1px 3px hsl(var(--primary) / 0.1);
	}

	/* Stage status color variants */
	.stage-tab--complete {
		color: hsl(142 76% 36%);
	}
	.stage-tab--complete.active {
		border-color: hsl(142 76% 36% / 0.4);
	}

	.stage-tab--running {
		color: hsl(var(--primary));
	}
	.stage-tab--running.active {
		border-color: hsl(var(--primary) / 0.4);
	}

	.stage-tab--error {
		color: hsl(var(--destructive));
	}
	.stage-tab--error.active {
		border-color: hsl(var(--destructive) / 0.4);
	}

	.stage-tab--not_started {
		color: hsl(var(--muted-foreground));
		opacity: 0.7;
	}
	.stage-tab--not_started.active {
		border-color: hsl(var(--border));
		opacity: 1;
	}

	.tab-header {
		display: flex;
		align-items: center;
		gap: 0.3rem;
	}

	:global(.tab-status-icon) {
		width: 0.8125rem;
		height: 0.8125rem;
		flex-shrink: 0;
	}

	:global(.tab-status-icon--complete) {
		color: hsl(142 76% 36%);
	}

	:global(.tab-status-icon--running) {
		color: hsl(var(--primary));
		animation: spin 1s linear infinite;
	}

	:global(.tab-status-icon--error) {
		color: hsl(var(--destructive));
	}

	:global(.tab-status-icon--not-started) {
		color: hsl(var(--muted-foreground));
		opacity: 0.5;
	}

	.tab-name {
		font-size: 0.8125rem;
		font-weight: 600;
	}

	.tab-desc {
		font-size: 0.6875rem;
		font-weight: 400;
		opacity: 0.7;
	}

	.stage-tab.active .tab-desc {
		opacity: 1;
	}

	/* Stage not started placeholder */
	.stage-not-started {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 0.625rem;
		padding: 3rem 1.5rem;
		color: hsl(var(--muted-foreground));
	}

	:global(.not-started-icon) {
		width: 2rem;
		height: 2rem;
		opacity: 0.35;
	}

	.not-started-title {
		font-size: 0.875rem;
		font-weight: 600;
		margin: 0;
	}

	.not-started-subtitle {
		font-size: 0.8125rem;
		margin: 0;
		opacity: 0.7;
	}

	/* ── Main Content Card ────────────────────────────────── */

	.main-content-card {
		border: 1px solid hsl(var(--border));
		border-radius: var(--radius-md);
		background: hsl(var(--card));
		overflow: hidden;
	}

	/* ── Sub-tabs ───────────────────────────────────────── */

	.sub-tab-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 1rem;
		padding: 0.5rem 0.75rem;
		background: hsl(var(--muted) / 0.35);
		border-bottom: 1px solid hsl(var(--border));
	}

	.sub-tab-actions {
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}

	.sub-tabs {
		display: flex;
		gap: 0.25rem;
	}

	.sub-tab {
		display: flex;
		align-items: center;
		gap: 0.375rem;
		padding: 0.375rem 0.75rem;
		border: 1px solid transparent;
		background: none;
		border-radius: var(--radius-sm);
		cursor: pointer;
		font-size: 0.8125rem;
		font-weight: 500;
		color: hsl(var(--muted-foreground));
		transition: all 0.15s ease;
	}

	.sub-tab:hover {
		color: hsl(var(--foreground));
		background-color: hsl(var(--background));
	}

	.sub-tab.active {
		color: hsl(var(--foreground));
		background-color: hsl(var(--background));
		border-color: hsl(var(--border));
		font-weight: 600;
	}

	:global(.sub-tab-icon) {
		width: 0.875rem;
		height: 0.875rem;
	}

	:global(.sub-tab-badge) {
		font-size: 0.625rem !important;
		padding: 0 0.375rem !important;
		height: 1.125rem !important;
		min-width: 1.125rem !important;
		scale: 0.9;
	}
	:global(.sub-tab-badge-zero) {
		border-color: oklch(0.637 0.237 25.331) !important;
		color: oklch(0.637 0.237 25.331) !important;
	}

	/* ── Tab Content ────────────────────────────────────── */

	.tab-content {
		min-height: 300px;
	}

	.logs-content {
		padding: 1rem;
	}

	.log-output {
		font-family: ui-monospace, monospace;
		font-size: 0.75rem;
		line-height: 1.6;
		white-space: pre-wrap;
		word-break: break-word;
		background: hsl(var(--muted) / 0.4);
		padding: 1rem;
		border-radius: var(--radius-sm);
		max-height: 500px;
		overflow-y: auto;
		margin: 0;
	}

	/* ── Re-Run Dialog ──────────────────────────────────── */

	.rerun-warning-body {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
		padding: 0.5rem 0;
		font-size: 0.875rem;
	}

	.rerun-warning-body p {
		margin: 0;
		line-height: 1.5;
	}

	.warning-callout {
		display: flex;
		gap: 0.625rem;
		padding: 0.75rem;
		background: hsl(var(--destructive) / 0.08);
		border: 1px solid hsl(var(--destructive) / 0.2);
		border-radius: var(--radius-sm);
		font-size: 0.8125rem;
		line-height: 1.5;
		color: hsl(var(--destructive));
		align-items: flex-start;
	}

	:global(.callout-icon) {
		width: 1rem;
		height: 1rem;
		flex-shrink: 0;
		margin-top: 0.125rem;
	}

	.rerun-error {
		padding: 0.5rem 0.75rem;
		background: hsl(var(--destructive) / 0.1);
		border: 1px solid hsl(var(--destructive) / 0.3);
		border-radius: var(--radius-sm);
		font-size: 0.8125rem;
		color: hsl(var(--destructive));
	}

	.success-banner {
		padding: 0.625rem 0.875rem;
		margin: 0.5rem 0.75rem 0;
		background: hsl(142 76% 36% / 0.1);
		border: 1px solid hsl(142 76% 36% / 0.25);
		border-radius: var(--radius-sm);
		font-size: 0.8125rem;
		color: hsl(142 76% 36%);
		font-weight: 500;
	}

	/* ── Responsive ─────────────────────────────────────── */

	@media (max-width: 768px) {
		.sub-tab-row {
			flex-direction: column;
			align-items: stretch;
		}

		.sub-tabs {
			justify-content: center;
		}

		.details-grid {
			flex-direction: column;
		}

		.stage-tabs {
			justify-content: flex-start;
		}
	}

	@keyframes spin {
		from { transform: rotate(0deg); }
		to { transform: rotate(360deg); }
	}
</style>
