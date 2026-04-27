<script lang="ts">
	import { onDestroy } from 'svelte';
	import { page } from '$app/stores';
	import { getUser, subscribeToJob, type SingleJobState } from '$lib/hooks';
	import { rerunJob } from '$lib/api';
	import { getJobFiles } from '$lib/api';
	import type { FileEntry, JobFilesResponse } from '$lib/api';
	import { FileViewer, StageTab } from '$lib/components';
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
		Loader2,
		AlertCircle,
		Film
	} from '@lucide/svelte';
	import type { StageStatus } from '../../../../types/StageStatus';
	import {
		registryStore,
		stageLogsKey,
		stageFilesKey,
		stageNumber,
		isRegistryLoaded,
		isRegistryError,
		type StageSpec
	} from '$lib/stores';
	import { getEffectiveJobStatus, jobStatusLabelVerbose, jobStatusVariant } from '$lib/jobStatus';

	// ── Auth ──────────────────────────────────────────────
	const user = getUser();
	const isAdmin = $derived(user.administrator);

	// ── Route param ────────────────────────────────────────
	const jobId = $derived($page.params.id as string);

	// ── State ──────────────────────────────────────────────
	let jobState = $state<SingleJobState>({ status: 'loading' });
	let activeStageKey: string = $state('');
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

	// ── Registry-driven stage list ──────────────────────────
	const registryState = $derived(registryStore.state);
	const stages = $derived(isRegistryLoaded(registryState) ? registryState.stages : []);

	// Default the active stage to the first one once the registry loads.
	$effect(() => {
		if (stages.length > 0 && !stages.some((s) => s.key === activeStageKey)) {
			activeStageKey = stages[0].key;
		}
	});

	const activeStage = $derived(stages.find((s) => s.key === activeStageKey));

	// ── Derived ────────────────────────────────────────────
	const job = $derived(jobState.status === 'loaded' ? jobState.job : null);

	const activeStageNumber = $derived(activeStage ? stageNumber(activeStage) : 1);

	const currentStageLogs = $derived.by(() => {
		if (!job?.stage_logs || !activeStage) return null;
		return job.stage_logs[stageLogsKey(activeStage)] ?? null;
	});

	// ── Files for active stage ──────────────────────────
	const outputFiles = $derived.by((): FileEntry[] | undefined => {
		if (!filesData || !activeStage) return undefined;
		return filesData.stages[stageFilesKey(activeStage)] ?? [];
	});

	// ── Tab counts ──────────────────────────────────────
	const outputFileCount = $derived(outputFiles?.length ?? 0);
	const logLineCount = $derived.by(() => {
		if (!currentStageLogs) return 0;
		return currentStageLogs.split('\n').length;
	});

	// ── Custom editor prerequisites (registry-driven lookups) ──
	function filesForStage(stageKey: string): FileEntry[] {
		const spec = stages.find((s) => s.key === stageKey);
		if (!spec || !filesData) return [];
		return filesData.stages[stageFilesKey(spec)] ?? [];
	}

	const videoEditFrontVideo = $derived(
		filesForStage('media-conversion').find(
			(f) => f.name.startsWith('front') && f.name.endsWith('.mp4')
		) ?? null
	);
	const videoEditSideVideo = $derived(
		filesForStage('media-conversion').find(
			(f) => f.name.startsWith('side') && f.name.endsWith('.mp4')
		) ?? null
	);
	const canOpenVideoEditor = $derived(!!(videoEditFrontVideo || videoEditSideVideo));

	const cycleFrontVideo = $derived(
		filesForStage('pose-estimation').find(
			(f) => f.name.startsWith('front') && f.name.endsWith('.mp4')
		) ?? null
	);
	const cycleSideVideo = $derived(
		filesForStage('pose-estimation').find(
			(f) => f.name.startsWith('side') && f.name.endsWith('.mp4')
		) ?? null
	);
	const cycleFrontJsonFile = $derived(
		filesForStage('cycle-detection').find((f) => f.name === 'front_gait_analysis.json') ?? null
	);
	const cycleSideJsonFile = $derived(
		filesForStage('cycle-detection').find((f) => f.name === 'side_gait_analysis.json') ?? null
	);
	const canOpenCycleEditor = $derived(
		!!(cycleFrontVideo || cycleSideVideo) && !!(cycleFrontJsonFile || cycleSideJsonFile)
	);

	/** Stages with a non-default panel — each renders a global action button. */
	const customPanelStages = $derived(stages.filter((s) => s.panel !== 'default'));

	// ── Stage status helpers ────────────────────────────────
	function getStageStatus(stage: StageSpec): StageStatus {
		return job?.stage_statuses?.[stageLogsKey(stage)] ?? 'not_started';
	}

	const activeStageStatus = $derived(activeStage ? getStageStatus(activeStage) : 'not_started');
	const stageHasContent = $derived(activeStageStatus !== 'not_started');

	// ── Status helpers ─────────────────────────────────────
	const effectiveStatus = $derived(job ? getEffectiveJobStatus(job, stages) : null);

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
		activeStageKey = stageKey;
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
			const result = await rerunJob(jobId, activeStage!.key);

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

	function handleOpenPanel(panel: StageSpec['panel']) {
		if (panel === 'video-edit') {
			videoEditorOpen = true;
		} else if (panel === 'gait-cycles') {
			cycleEditorOpen = true;
		}
	}

	function canOpenPanel(panel: StageSpec['panel']): boolean {
		if (panel === 'video-edit') return canOpenVideoEditor;
		if (panel === 'gait-cycles') return canOpenCycleEditor;
		return false;
	}

	function panelButtonLabel(panel: StageSpec['panel']): string {
		if (panel === 'video-edit') return 'Video Editor';
		if (panel === 'gait-cycles') return 'Cycle Editor';
		return '';
	}

	function panelButtonTitle(panel: StageSpec['panel']): string {
		if (panel === 'video-edit')
			return 'Media-conversion outputs must exist to use the Video Editor';
		if (panel === 'gait-cycles')
			return 'Pose-estimation videos and cycle-detection JSON must exist';
		return '';
	}
</script>

<svelte:head>
	<title>Job {formatJobId(jobId)} - iGait</title>
</svelte:head>

{#if jobState.status === 'loading' || registryState.status === 'loading'}
	<div class="state-message">
		<Loader2 class="spinner" />
		<p>Loading job details...</p>
	</div>
{:else if jobState.status === 'error'}
	<div class="state-message">
		<AlertCircle class="error-icon" />
		<p>{jobState.error}</p>
	</div>
{:else if isRegistryError(registryState)}
	<div class="state-message">
		<AlertCircle class="error-icon" />
		<p>Failed to load stage registry: {registryState.error}</p>
	</div>
{:else if job && activeStage}
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
				{#if effectiveStatus}
					<Badge variant={jobStatusVariant(effectiveStatus)}
						>{jobStatusLabelVerbose(effectiveStatus)}</Badge
					>
				{/if}
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
							<span class="detail-label">Result</span>
							<span class="detail-value">
								{job.status.asd
									? 'Submitted videos show walking pattern more similar to autistic children than non-autistic children.'
									: 'Submitted videos show walking pattern more similar to non-autistic children than autistic children.'}
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
						{#if isAdmin}
							<pre class="error-preview">{job.status.logs}</pre>
						{:else}
							<p class="text-sm text-muted-foreground">
								Something went wrong processing your submission. Please contact GaitStudy@niu.edu
								for assistance.
							</p>
						{/if}
					</div>
				{/if}
			</div>
		</div>

		<!-- Stage Tabs (admin only) -->
		{#if isAdmin}
			<div class="stage-tabs-container">
				<div class="stage-tabs">
					{#each stages as stage (stage.key)}
						<StageTab
							{stage}
							active={activeStageKey === stage.key}
							status={getStageStatus(stage)}
							onclick={() => handleStageClick(stage.key)}
						/>
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
									<Badge
										variant="outline"
										class="sub-tab-badge {outputFileCount === 0 ? 'sub-tab-badge-zero' : ''}"
										>{outputFileCount}</Badge
									>
								{/if}
							</button>
							<button
								class="sub-tab"
								class:active={activeSubTab === 'logs'}
								onclick={() => handleSubTabClick('logs')}
							>
								<ScrollText class="sub-tab-icon" />
								Logs
								<Badge
									variant="outline"
									class="sub-tab-badge {logLineCount === 0 ? 'sub-tab-badge-zero' : ''}"
									>{logLineCount}</Badge
								>
							</button>
						</div>

						<div class="sub-tab-actions">
							{#each customPanelStages as panelStage (panelStage.key)}
								{@const enabled = canOpenPanel(panelStage.panel)}
								<Button
									variant="outline"
									size="sm"
									disabled={!enabled}
									title={enabled ? '' : panelButtonTitle(panelStage.panel)}
									onclick={() => handleOpenPanel(panelStage.panel)}
								>
									<Film class="mr-1 h-4 w-4" />
									{panelButtonLabel(panelStage.panel)}
								</Button>
							{/each}
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
						<p class="not-started-title">{activeStage.display_name}</p>
						<p class="not-started-description">{activeStage.description}</p>
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
					<strong>{activeStage.display_name}</strong>.
				</p>
				<div class="warning-callout">
					<AlertTriangle class="callout-icon" />
					<span>
						This will <strong>clear all outputs</strong> from {activeStage.display_name}
						onward. The job will be re-queued for processing.
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
						Re-Run from {activeStage.display_name}
					{/if}
				</Button>
			</Dialog.Footer>
		</Dialog.Content>
	</Dialog.Root>

	<CycleEditor
		open={cycleEditorOpen}
		onclose={() => (cycleEditorOpen = false)}
		{jobId}
		frontVideo={cycleFrontVideo}
		sideVideo={cycleSideVideo}
		frontJsonFile={cycleFrontJsonFile}
		sideJsonFile={cycleSideJsonFile}
	/>

	<VideoEditor
		open={videoEditorOpen}
		onclose={() => (videoEditorOpen = false)}
		{jobId}
		frontVideo={videoEditFrontVideo}
		sideVideo={videoEditSideVideo}
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

	/* Stage not started placeholder */
	.stage-not-started {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 0.5rem;
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

	.not-started-description {
		font-size: 0.75rem;
		margin: 0;
		max-width: 34rem;
		text-align: center;
		line-height: 1.4;
	}

	.not-started-subtitle {
		font-size: 0.8125rem;
		margin: 0.25rem 0 0;
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
		from {
			transform: rotate(0deg);
		}
		to {
			transform: rotate(360deg);
		}
	}
</style>
