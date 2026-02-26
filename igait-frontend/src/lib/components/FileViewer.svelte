<script lang="ts">
	import { Loader2, FileText, FileQuestion, Eye, Film } from '@lucide/svelte';
	import FileViewerModal from './FileViewerModal.svelte';
	import CycleEditor from './CycleEditor.svelte';
	import type { FileEntry, JobFilesResponse } from '$lib/api';

	interface Props {
		/** Files for the current stage */
		files: FileEntry[] | undefined;
		/** Whether files are still loading */
		loading: boolean;
		/** Error message if fetch failed */
		error: string | null;
		/** Label shown in the empty state */
		label: string;
		/** Current stage number (1-7) */
		stageNumber?: number;
		/** All files data for cross-stage lookups */
		allFiles?: JobFilesResponse | null;
		/** Whether the current user is an admin */
		isAdmin?: boolean;
		/** Job ID for API calls */
		jobId?: string;
	}

	let { files, loading, error, label, stageNumber, allFiles, isAdmin, jobId }: Props = $props();

	/** Currently selected file for the modal viewer */
	let selectedFile: FileEntry | null = $state(null);

	/** Whether the cycle editor is open */
	let cycleEditorOpen = $state(false);

	/** Get the file extension from a filename */
	function getExtension(name: string): string {
		const dot = name.lastIndexOf('.');
		return dot === -1 ? '' : name.slice(dot + 1).toLowerCase();
	}

	/** Check if a file is a cycle detection JSON that can be edited */
	function isCycleJson(fileName: string): boolean {
		return fileName === 'front_gait_analysis.json' || fileName === 'side_gait_analysis.json';
	}

	/** Check if the cycle editor button should show for this file */
	function canEditCycles(file: FileEntry): boolean {
		if (!isAdmin || stageNumber !== 5 || !allFiles || !jobId) return false;
		if (!isCycleJson(file.name)) return false;

		// Check that stage 4 has the corresponding video(s)
		const stage4Files = allFiles.stages['stage_4'] ?? [];
		const hasFrontVideo = stage4Files.some((f) => f.name.startsWith('front') && f.name.endsWith('.mp4'));
		const hasSideVideo = stage4Files.some((f) => f.name.startsWith('side') && f.name.endsWith('.mp4'));

		return hasFrontVideo || hasSideVideo;
	}

	// ── Derived: gather files for the cycle editor ────────
	const stage4Files = $derived(allFiles?.stages['stage_4'] ?? []);
	const stage5Files = $derived(allFiles?.stages['stage_5'] ?? []);

	const frontVideoFile = $derived(
		stage4Files.find((f) => f.name.startsWith('front') && f.name.endsWith('.mp4')) ?? null
	);
	const sideVideoFile = $derived(
		stage4Files.find((f) => f.name.startsWith('side') && f.name.endsWith('.mp4')) ?? null
	);
	const frontJsonFile = $derived(
		stage5Files.find((f) => f.name === 'front_gait_analysis.json') ?? null
	);
	const sideJsonFile = $derived(
		stage5Files.find((f) => f.name === 'side_gait_analysis.json') ?? null
	);

	function handleRowClick(file: FileEntry) {
		selectedFile = file;
	}

	function handleModalClose() {
		selectedFile = null;
	}

	function handleCycleEditorOpen(e: MouseEvent) {
		e.stopPropagation();
		cycleEditorOpen = true;
	}

	function handleCycleEditorClose() {
		cycleEditorOpen = false;
	}
</script>

{#if loading}
	<div class="viewer-state">
		<Loader2 class="spinner" />
		<p>Loading files…</p>
	</div>
{:else if error}
	<div class="viewer-state">
		<p class="viewer-error">{error}</p>
	</div>
{:else if !files || files.length === 0}
	<div class="viewer-state">
		<p class="viewer-label">{label}</p>
	</div>
{:else}
	<div class="file-list">
		{#each files as file (file.name)}
			{@const ext = getExtension(file.name)}
			{@const showCycleBtn = canEditCycles(file)}
			<button class="file-row" onclick={() => handleRowClick(file)}>
				<div class="file-row-left">
					{#if ext === 'json'}
						<FileText class="file-row-icon file-row-icon--json" />
					{:else}
						<FileQuestion class="file-row-icon" />
					{/if}
					<span class="file-row-name">{file.name}</span>
					<span class="file-row-ext">.{ext}</span>
				</div>
				<div class="file-row-right">
					{#if showCycleBtn}
						<!-- svelte-ignore a11y_no_static_element_interactions -->
						<span
							class="cycle-edit-btn"
							onclick={handleCycleEditorOpen}
							onkeydown={(e) => { if (e.key === 'Enter') { e.stopPropagation(); cycleEditorOpen = true; } }}
							role="button"
							tabindex="0"
							title="Open Cycle Editor"
						>
							<Film class="file-row-action file-row-action--cycle" />
						</span>
					{/if}
					<Eye class="file-row-action" />
				</div>
			</button>
		{/each}
	</div>
{/if}

<FileViewerModal file={selectedFile} onclose={handleModalClose} />

{#if jobId}
	<CycleEditor
		open={cycleEditorOpen}
		onclose={handleCycleEditorClose}
		{jobId}
		frontVideo={frontVideoFile}
		sideVideo={sideVideoFile}
		{frontJsonFile}
		{sideJsonFile}
	/>
{/if}

<style>
	/* ── States ───────────────────────────────────────────── */

	.viewer-state {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		padding: 3rem 2rem;
		text-align: center;
		color: hsl(var(--muted-foreground));
		gap: 0.5rem;
	}

	.viewer-state p {
		margin: 0;
	}

	.viewer-label {
		font-size: 0.875rem;
	}

	.viewer-error {
		font-size: 0.8125rem;
		color: hsl(var(--destructive));
	}

	/* ── Row list ─────────────────────────────────────────── */

	.file-list {
		display: flex;
		flex-direction: column;
	}

	.file-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 0.5rem 1rem;
		gap: 0.75rem;
		border: none;
		background: none;
		cursor: pointer;
		text-align: left;
		transition: background-color 0.15s ease;
		border-bottom: 1px solid color-mix(in oklch, var(--border) 50%, transparent);
	}

	.file-row:last-child {
		border-bottom: none;
	}

	.file-row:hover {
		background-color: color-mix(in oklch, var(--muted) 50%, transparent);
	}

	.file-row-left {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		min-width: 0;
	}

	:global(.file-row-icon) {
		width: 1rem;
		height: 1rem;
		flex-shrink: 0;
		color: var(--muted-foreground);
	}

	:global(.file-row-icon--json) {
		color: var(--primary);
	}

	.file-row-name {
		font-family: ui-monospace, monospace;
		font-size: 0.8125rem;
		font-weight: 500;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.file-row-ext {
		font-size: 0.6875rem;
		color: var(--muted-foreground);
		flex-shrink: 0;
	}

	.file-row-right {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		flex-shrink: 0;
	}

	:global(.file-row-action) {
		width: 0.875rem;
		height: 0.875rem;
		color: var(--muted-foreground);
		opacity: 0;
		transition: opacity 0.15s ease;
	}

	.file-row:hover :global(.file-row-action) {
		opacity: 1;
	}

	:global(.file-row-action--cycle) {
		color: var(--primary);
	}

	.cycle-edit-btn {
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 0.125rem;
		border: none;
		background: none;
		cursor: pointer;
		border-radius: var(--radius-sm);
		transition: background-color 0.15s ease;
	}

	.cycle-edit-btn:hover {
		background-color: color-mix(in oklch, var(--primary) 15%, transparent);
	}
</style>
