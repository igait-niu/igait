<script lang="ts">
	import { Loader2, Plus, Trash2, Save, X, Play, Pause, SkipBack, SkipForward } from '@lucide/svelte';
	import { Button } from '$lib/components/ui/button';
	import * as Dialog from '$lib/components/ui/dialog';
	import { Badge } from '$lib/components/ui/badge';
	import { updateCycles, rerunJob } from '$lib/api';
	import type { FileEntry, GaitCycle } from '$lib/api';
	import { toast } from 'svelte-sonner';

	interface Props {
		/** Whether the editor is open */
		open: boolean;
		/** Callback to close the editor */
		onclose: () => void;
		/** The job ID */
		jobId: string;
		/** Front video file entry (from stage 4) */
		frontVideo: FileEntry | null;
		/** Side video file entry (from stage 4) */
		sideVideo: FileEntry | null;
		/** Front gait analysis JSON file entry (from stage 5) */
		frontJsonFile: FileEntry | null;
		/** Side gait analysis JSON file entry (from stage 5) */
		sideJsonFile: FileEntry | null;
	}

	let { open, onclose, jobId, frontVideo, sideVideo, frontJsonFile, sideJsonFile }: Props = $props();

	// ── State ─────────────────────────────────────────────
	let loading = $state(true);
	let saving = $state(false);
	let frontCycles: GaitCycle[] = $state([]);
	let sideCycles: GaitCycle[] = $state([]);
	let videoEl: HTMLVideoElement | undefined = $state(undefined);
	let isPlaying = $state(false);
	let currentTime = $state(0);
	let frontDuration = $state(0);
	let sideDuration = $state(0);
	let frontTotalFrames = $state(0);
	let sideTotalFrames = $state(0);
	let activeTab: 'front' | 'side' = $state('front');
	let selectedCycleIndex: number | null = $state(null);
	let editingCycle: GaitCycle | null = $state(null);
	let hasChanges = $state(false);
	let fps = 30; // Default framerate assumption
	let isDraggingScrubber = $state(false);

	// ── Derived ───────────────────────────────────────────
	const activeCycles = $derived(activeTab === 'front' ? frontCycles : sideCycles);
	const activeVideo = $derived(activeTab === 'front' ? frontVideo : sideVideo);
	const activeJsonFile = $derived(activeTab === 'front' ? frontJsonFile : sideJsonFile);
	const totalFrames = $derived(activeTab === 'front' ? frontTotalFrames : sideTotalFrames);
	const duration = $derived(activeTab === 'front' ? frontDuration : sideDuration);
	const currentFrame = $derived(Math.round(currentTime * fps));
	const hasFrontTab = $derived(!!frontVideo || !!frontJsonFile);
	const hasSideTab = $derived(!!sideVideo || !!sideJsonFile);

	// ── Load cycle data ───────────────────────────────────
	$effect(() => {
		if (open) {
			loadCycleData();
			// Default to the first available tab
			if (hasFrontTab) activeTab = 'front';
			else if (hasSideTab) activeTab = 'side';
		} else {
			// Reset state when closed
			frontCycles = [];
			sideCycles = [];
			loading = true;
			hasChanges = false;
			selectedCycleIndex = null;
			editingCycle = null;
			isPlaying = false;
			currentTime = 0;
		}
	});

	async function loadCycleData() {
		loading = true;
		try {
			const loads: Promise<void>[] = [];

			if (frontJsonFile) {
				loads.push(
					fetch(frontJsonFile.url)
						.then((r) => r.json())
						.then((data) => {
							frontCycles = (data.gait_cycles ?? []).map((c: GaitCycle) => ({ ...c }));
							if (data.landmark_data?.length) {
								frontTotalFrames = data.landmark_data.length;
							}
						})
				);
			}

			if (sideJsonFile) {
				loads.push(
					fetch(sideJsonFile.url)
						.then((r) => r.json())
						.then((data) => {
							sideCycles = (data.gait_cycles ?? []).map((c: GaitCycle) => ({ ...c }));
							if (data.landmark_data?.length) {
								sideTotalFrames = data.landmark_data.length;
							}
						})
				);
			}

			await Promise.all(loads);
		} catch (err) {
			console.error('Failed to load cycle data:', err);
			toast.error('Failed to load cycle data');
		} finally {
			loading = false;
		}
	}

	// ── Video controls ────────────────────────────────────
	function handleVideoLoaded(e: Event) {
		const video = e.target as HTMLVideoElement;
		if (video.duration) {
			if (activeTab === 'front') frontDuration = video.duration;
			else sideDuration = video.duration;
		}
	}

	function handleTabSwitch(tab: 'front' | 'side') {
		if (tab === activeTab) return;
		// Pause current video
		if (isPlaying) {
			videoEl?.pause();
			isPlaying = false;
		}
		activeTab = tab;
		currentTime = 0;
		selectedCycleIndex = null;
		editingCycle = null;
	}

	function togglePlayback() {
		if (!videoEl) return;

		if (isPlaying) {
			videoEl.pause();
			isPlaying = false;
		} else {
			videoEl.play();
			isPlaying = true;
		}
	}

	function handleTimeUpdate() {
		if (videoEl && !isDraggingScrubber) {
			currentTime = videoEl.currentTime;
		}
	}

	function handleVideoEnded() {
		isPlaying = false;
	}

	function seekToFrame(frame: number) {
		const time = frame / fps;
		if (videoEl) videoEl.currentTime = time;
		currentTime = time;
	}

	function stepFrame(delta: number) {
		const newFrame = Math.max(0, currentFrame + delta);
		seekToFrame(newFrame);
	}

	function handleTimelineClick(e: MouseEvent) {
		const target = e.currentTarget as HTMLElement;
		const rect = target.getBoundingClientRect();
		const fraction = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width));
		const maxFrames = totalFrames || Math.round(duration * fps);
		if (maxFrames > 0) {
			seekToFrame(Math.round(fraction * maxFrames));
		}
	}

	// ── Scrubber drag ─────────────────────────────────────
	function handleScrubberDown(e: MouseEvent) {
		e.preventDefault();
		e.stopPropagation();
		isDraggingScrubber = true;

		const timeline = (e.target as HTMLElement).closest('.timeline') as HTMLElement;
		if (!timeline) return;

		const onMove = (ev: MouseEvent) => {
			const rect = timeline.getBoundingClientRect();
			const fraction = Math.max(0, Math.min(1, (ev.clientX - rect.left) / rect.width));
			const maxFrames = totalFrames || Math.round(duration * fps);
			if (maxFrames > 0) {
				const frame = Math.round(fraction * maxFrames);
				const time = frame / fps;
				if (videoEl) videoEl.currentTime = time;
				currentTime = time;
			}
		};

		const onUp = () => {
			isDraggingScrubber = false;
			window.removeEventListener('mousemove', onMove);
			window.removeEventListener('mouseup', onUp);
		};

		window.addEventListener('mousemove', onMove);
		window.addEventListener('mouseup', onUp);
	}

	// ── Cycle CRUD ────────────────────────────────────────
	function addCycle() {
		const newCycle: GaitCycle = {
			start: currentFrame,
			end: Math.min(currentFrame + 30, totalFrames || Math.round(duration * fps)),
			side: 'L'
		};

		if (activeTab === 'front') {
			frontCycles = [...frontCycles, newCycle];
		} else {
			sideCycles = [...sideCycles, newCycle];
		}

		hasChanges = true;
		selectedCycleIndex = activeCycles.length;
		editingCycle = { ...newCycle };
	}

	function selectCycle(index: number) {
		if (selectedCycleIndex === index) {
			selectedCycleIndex = null;
			editingCycle = null;
		} else {
			selectedCycleIndex = index;
			editingCycle = { ...activeCycles[index] };
			seekToFrame(activeCycles[index].start);
		}
	}

	function updateSelectedCycle() {
		if (selectedCycleIndex === null || !editingCycle) return;

		if (activeTab === 'front') {
			frontCycles = frontCycles.map((c, i) =>
				i === selectedCycleIndex ? { ...editingCycle! } : c
			);
		} else {
			sideCycles = sideCycles.map((c, i) =>
				i === selectedCycleIndex ? { ...editingCycle! } : c
			);
		}

		hasChanges = true;
	}

	function deleteCycle(index: number) {
		if (activeTab === 'front') {
			frontCycles = frontCycles.filter((_, i) => i !== index);
		} else {
			sideCycles = sideCycles.filter((_, i) => i !== index);
		}

		if (selectedCycleIndex === index) {
			selectedCycleIndex = null;
			editingCycle = null;
		} else if (selectedCycleIndex !== null && selectedCycleIndex > index) {
			selectedCycleIndex--;
		}

		hasChanges = true;
	}

	function toggleCycleSide() {
		if (!editingCycle) return;
		editingCycle = { ...editingCycle, side: editingCycle.side === 'L' ? 'R' : 'L' };
		updateSelectedCycle();
	}

	// ── Save ──────────────────────────────────────────────
	async function handleSave() {
		saving = true;

		try {
			const saves: Promise<void>[] = [];

			if (frontJsonFile) {
				const result = updateCycles(jobId, 'front_gait_analysis.json', frontCycles);
				saves.push(
					result.then((r) => {
						if (r.isErr()) throw new Error(r.error.rootCause);
					})
				);
			}

			if (sideJsonFile) {
				const result = updateCycles(jobId, 'side_gait_analysis.json', sideCycles);
				saves.push(
					result.then((r) => {
						if (r.isErr()) throw new Error(r.error.rootCause);
					})
				);
			}

			await Promise.all(saves);
			hasChanges = false;

			// Trigger re-run from stage 6 (ML Prediction)
			const rerunResult = await rerunJob(jobId, 6);
			if (rerunResult.isOk()) {
				toast.success('Cycles saved — re-running from Stage 6!');
				onclose();
			} else {
				toast.error(`Cycles saved, but re-run failed: ${rerunResult.error.rootCause}`);
			}
		} catch (err) {
			console.error('Save failed:', err);
			toast.error(`Failed to save: ${err instanceof Error ? err.message : 'Unknown error'}`);
		} finally {
			saving = false;
		}
	}

	function handleClose() {
		if (hasChanges) {
			if (!confirm('You have unsaved changes. Are you sure you want to close?')) {
				return;
			}
		}
		onclose();
	}

	// ── Timeline helpers ──────────────────────────────────
	function getCyclePosition(cycle: GaitCycle): { left: string; width: string } {
		const maxFrames = totalFrames || Math.round(duration * fps) || 1;
		const left = (cycle.start / maxFrames) * 100;
		const width = ((cycle.end - cycle.start) / maxFrames) * 100;
		return {
			left: `${Math.max(0, left)}%`,
			width: `${Math.max(0.5, Math.min(width, 100 - left))}%`
		};
	}

	function getPlayheadPosition(): string {
		const maxFrames = totalFrames || Math.round(duration * fps) || 1;
		return `${(currentFrame / maxFrames) * 100}%`;
	}

	function formatTime(seconds: number): string {
		const mins = Math.floor(seconds / 60);
		const secs = Math.floor(seconds % 60);
		const ms = Math.floor((seconds % 1) * 100);
		return `${mins}:${secs.toString().padStart(2, '0')}.${ms.toString().padStart(2, '0')}`;
	}
</script>

<Dialog.Root
	open={open}
	onOpenChange={(o) => { if (!o) handleClose(); }}
>
	<Dialog.Content class="cycle-editor-dialog sm:max-w-[95vw] max-h-[95vh]">
		<Dialog.Header>
			<Dialog.Title class="cycle-editor-title">
				Cycle Editor
				<Badge variant="outline" class="ml-2">Stage 5</Badge>
				{#if hasChanges}
					<Badge variant="destructive" class="ml-2">Unsaved</Badge>
				{/if}
			</Dialog.Title>
			<Dialog.Description class="sr-only">
				Edit gait cycle detection timeframes for this job
			</Dialog.Description>
		</Dialog.Header>

		{#if loading}
			<div class="editor-loading">
				<Loader2 class="spinner" />
				<p>Loading cycle data…</p>
			</div>
		{:else}
			<div class="editor-body">
				<!-- Video + Timeline Section -->
				<div class="video-section">
					<!-- Front / Side Video Tabs -->
					<div class="video-tabs">
						{#if hasFrontTab}
							<button
								class="video-tab"
								class:active={activeTab === 'front'}
								onclick={() => handleTabSwitch('front')}
							>
								Front View
								<Badge variant="outline" class="ml-1.5 video-tab-badge">{frontCycles.length}</Badge>
							</button>
						{/if}
						{#if hasSideTab}
							<button
								class="video-tab"
								class:active={activeTab === 'side'}
								onclick={() => handleTabSwitch('side')}
							>
								Side View
								<Badge variant="outline" class="ml-1.5 video-tab-badge">{sideCycles.length}</Badge>
							</button>
						{/if}
					</div>

					<!-- Single Active Video -->
					<div class="video-wrapper">
						{#if activeVideo}
							{#key activeTab}
								<!-- svelte-ignore a11y_media_has_caption -->
								<video
									bind:this={videoEl}
									src={activeVideo.url}
									onloadedmetadata={handleVideoLoaded}
									ontimeupdate={handleTimeUpdate}
									onended={handleVideoEnded}
									preload="metadata"
									class="video-player"
								></video>
							{/key}
						{:else}
							<div class="no-video">
								<p>No {activeTab} video available</p>
							</div>
						{/if}
					</div>

					<!-- Timeline (directly below video) -->
					<!-- svelte-ignore a11y_click_events_have_key_events -->
					<!-- svelte-ignore a11y_no_static_element_interactions -->
					<div class="timeline" onclick={handleTimelineClick}>
						<div class="timeline-track">
							<!-- Cycle overlays -->
							{#each activeCycles as cycle, i}
								{@const pos = getCyclePosition(cycle)}
								<div
									class="cycle-block"
									class:cycle-block--left={cycle.side === 'L'}
									class:cycle-block--right={cycle.side === 'R'}
									class:cycle-block--selected={selectedCycleIndex === i}
									style="left: {pos.left}; width: {pos.width}"
									onclick={(e) => { e.stopPropagation(); selectCycle(i); }}
									onkeydown={(e) => { if (e.key === 'Enter') selectCycle(i); }}
									role="button"
									tabindex="0"
									title="Cycle {i + 1}: {cycle.side} ({cycle.start}–{cycle.end})"
								>
									<span class="cycle-block-label">{cycle.side}{i + 1}</span>
								</div>
							{/each}

							<!-- Scrubber / Playhead -->
							<!-- svelte-ignore a11y_no_static_element_interactions -->
							<div
								class="scrubber"
								style="left: {getPlayheadPosition()}"
								onmousedown={handleScrubberDown}
							>
								<div class="scrubber-head"></div>
								<div class="scrubber-line"></div>
							</div>
						</div>
						<div class="timeline-frame-label" style="left: {getPlayheadPosition()}">
							{currentFrame}
						</div>
					</div>

					<!-- Playback Controls -->
					<div class="playback-controls">
						<div class="control-group">
							<button class="control-btn" onclick={() => stepFrame(-5)} title="Back 5 frames">
								<SkipBack class="h-4 w-4" />
							</button>
							<button class="control-btn control-btn--play" onclick={togglePlayback} title={isPlaying ? 'Pause' : 'Play'}>
								{#if isPlaying}
									<Pause class="h-5 w-5" />
								{:else}
									<Play class="h-5 w-5" />
								{/if}
							</button>
							<button class="control-btn" onclick={() => stepFrame(5)} title="Forward 5 frames">
								<SkipForward class="h-4 w-4" />
							</button>
						</div>
						<div class="time-display">
							<span class="frame-indicator">Frame {currentFrame}</span>
							<span class="time-indicator">{formatTime(currentTime)} / {formatTime(duration)}</span>
						</div>
					</div>
				</div>

				<!-- Cycle List Panel -->
				<div class="cycles-panel">
					<div class="panel-header">
						<span class="panel-title">{activeTab === 'front' ? 'Front' : 'Side'} Cycles</span>
						<Badge variant="outline" class="panel-count-badge">{activeCycles.length}</Badge>
					</div>

					<!-- Cycle list -->
					<div class="cycle-list">
						{#each activeCycles as cycle, i (i)}
							<div
								class="cycle-item"
								class:cycle-item--selected={selectedCycleIndex === i}
								onclick={() => selectCycle(i)}
								onkeydown={(e) => { if (e.key === 'Enter') selectCycle(i); }}
								role="button"
								tabindex="0"
							>
								<div class="cycle-item-info">
									<Badge
										variant={cycle.side === 'L' ? 'default' : 'secondary'}
										class="cycle-side-badge"
									>
										{cycle.side === 'L' ? 'Left' : 'Right'}
									</Badge>
									<span class="cycle-frames">
										{cycle.start} → {cycle.end}
									</span>
									<span class="cycle-duration">
										({cycle.end - cycle.start} frames)
									</span>
								</div>
								<button
									class="cycle-delete-btn"
									onclick={(e) => { e.stopPropagation(); deleteCycle(i); }}
									title="Delete cycle"
								>
									<Trash2 class="h-3.5 w-3.5" />
								</button>
							</div>
						{/each}

						{#if activeCycles.length === 0}
							<div class="no-cycles">
								<p>No cycles detected</p>
								<p class="no-cycles-hint">Click "Add Cycle" to create one</p>
							</div>
						{/if}
					</div>

					<!-- Edit panel (when a cycle is selected) -->
					{#if editingCycle && selectedCycleIndex !== null}
						<div class="edit-panel">
							<h4 class="edit-title">Edit Cycle {selectedCycleIndex + 1}</h4>
							<div class="edit-fields">
								<div class="edit-field">
									<label for="cycle-start">Start Frame</label>
									<input
										id="cycle-start"
										type="number"
										min="0"
										max={editingCycle.end - 1}
										bind:value={editingCycle.start}
										onchange={updateSelectedCycle}
										class="edit-input"
									/>
								</div>
								<div class="edit-field">
									<label for="cycle-end">End Frame</label>
									<input
										id="cycle-end"
										type="number"
										min={editingCycle.start + 1}
										bind:value={editingCycle.end}
										onchange={updateSelectedCycle}
										class="edit-input"
									/>
								</div>
								<div class="edit-field">
									<label>Side</label>
									<button class="side-toggle" onclick={toggleCycleSide}>
										<Badge variant={editingCycle.side === 'L' ? 'default' : 'secondary'}>
											{editingCycle.side === 'L' ? 'Left' : 'Right'}
										</Badge>
									</button>
								</div>
							</div>
							<div class="edit-actions">
								<Button variant="outline" size="sm" onclick={() => seekToFrame(editingCycle!.start)}>
									Go to Start
								</Button>
								<Button variant="outline" size="sm" onclick={() => seekToFrame(editingCycle!.end)}>
									Go to End
								</Button>
								<Button variant="outline" size="sm" onclick={() => { if (editingCycle) { editingCycle = { ...editingCycle, start: currentFrame }; updateSelectedCycle(); } }}>
									Set Start to Current
								</Button>
								<Button variant="outline" size="sm" onclick={() => { if (editingCycle) { editingCycle = { ...editingCycle, end: currentFrame }; updateSelectedCycle(); } }}>
									Set End to Current
								</Button>
							</div>
						</div>
					{/if}

					<!-- Add button -->
					<div class="panel-actions">
						<Button variant="outline" size="sm" class="w-full" onclick={addCycle}>
							<Plus class="h-4 w-4 mr-1" />
							Add Cycle at Current Frame
						</Button>
					</div>
				</div>
			</div>
		{/if}

		<div class="editor-footer">
			<div></div>
			<Button
				variant="default"
				size="sm"
				onclick={handleSave}
				disabled={saving || !hasChanges}
			>
				{#if saving}
					<Loader2 class="h-4 w-4 mr-1 animate-spin" />
					Saving…
				{:else}
					<Save class="h-4 w-4 mr-1" />
					Save Changes
				{/if}
			</Button>
		</div>
	</Dialog.Content>
</Dialog.Root>

<style>
	:global(.cycle-editor-dialog) {
		display: flex;
		flex-direction: column;
		overflow: hidden;
		width: 95vw !important;
		max-width: 95vw !important;
		height: 90vh;
		max-height: 90vh;
	}

	:global(.cycle-editor-title) {
		display: flex;
		align-items: center;
		font-size: 1rem !important;
	}

	/* ── Loading ──────────────────────────────────────────── */

	.editor-loading {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		padding: 4rem 2rem;
		color: var(--muted-foreground);
		gap: 0.75rem;
	}

	.editor-loading p {
		margin: 0;
		font-size: 0.875rem;
	}

	/* ── Body ─────────────────────────────────────────────── */

	.editor-body {
		display: grid;
		grid-template-columns: 1fr 320px;
		gap: 1rem;
		flex: 1;
		overflow: hidden;
		padding: 0.5rem 0;
		min-height: 0;
	}

	/* ── Video Section ────────────────────────────────────── */

	.video-section {
		display: flex;
		flex-direction: column;
		gap: 0;
		min-height: 0;
		overflow: hidden;
	}

	/* ── Video Tabs ───────────────────────────────────────── */

	.video-tabs {
		display: flex;
		gap: 0.25rem;
		padding: 0 0.25rem;
		flex-shrink: 0;
		background: color-mix(in oklch, var(--muted) 30%, transparent);
		border-radius: var(--radius) var(--radius) 0 0;
		padding-top: 0.25rem;
	}

	.video-tab {
		display: flex;
		align-items: center;
		padding: 0.5rem 1.25rem;
		border: 1px solid transparent;
		border-bottom: none;
		background: transparent;
		cursor: pointer;
		font-size: 0.8125rem;
		font-weight: 600;
		color: var(--muted-foreground);
		transition: all 0.15s ease;
		border-radius: var(--radius) var(--radius) 0 0;
		position: relative;
	}

	.video-tab.active {
		color: var(--foreground);
		background: var(--background);
		border-color: var(--border);
		box-shadow: 0 -2px 6px color-mix(in oklch, var(--primary) 8%, transparent);
	}

	/* hides bottom border to blend into content */
	.video-tab.active::after {
		content: '';
		position: absolute;
		bottom: -1px;
		left: 0;
		right: 0;
		height: 1px;
		background: var(--background);
	}

	.video-tab:hover:not(.active) {
		color: color-mix(in oklch, var(--foreground) 80%, transparent);
		background: color-mix(in oklch, var(--muted) 50%, transparent);
	}

	:global(.video-tab-badge) {
		font-size: 0.625rem !important;
		padding: 0 0.375rem !important;
		height: 1.125rem !important;
	}

	/* ── Video Player ─────────────────────────────────────── */

	.video-wrapper {
		position: relative;
		display: flex;
		flex-direction: column;
		flex: 1;
		min-height: 0;
		background: hsl(0 0% 5%);
		overflow: hidden;
		border-left: 1px solid var(--border);
		border-right: 1px solid var(--border);
	}

	.video-player {
		width: 100%;
		height: 100%;
		object-fit: contain;
		background: black;
	}

	.no-video {
		display: flex;
		align-items: center;
		justify-content: center;
		flex: 1;
		color: var(--muted-foreground);
		font-size: 0.875rem;
	}

	.no-video p {
		margin: 0;
	}

	/* ── Timeline ─────────────────────────────────────────── */

	.timeline {
		padding: 0.625rem 0.5rem 1.25rem;
		cursor: pointer;
		flex-shrink: 0;
		position: relative;
		background: color-mix(in oklch, var(--muted) 25%, transparent);
		border: 1px solid var(--border);
		border-top: none;
		border-radius: 0 0 var(--radius) var(--radius);
	}

	.timeline-track {
		position: relative;
		height: 2.25rem;
		background: color-mix(in oklch, var(--muted) 50%, transparent);
		border-radius: var(--radius-sm);
		overflow: visible;
		border: 1px solid color-mix(in oklch, var(--border) 60%, transparent);
		box-shadow: inset 0 1px 3px hsl(0 0% 0% / 0.06);
	}

	.cycle-block {
		position: absolute;
		top: 2px;
		bottom: 2px;
		border-radius: 3px;
		display: flex;
		align-items: center;
		justify-content: center;
		cursor: pointer;
		transition: opacity 0.15s ease, filter 0.15s ease;
		z-index: 1;
	}

	.cycle-block--left {
		background: hsl(210 80% 55% / 0.5);
		border: 1px solid hsl(210 80% 55% / 0.8);
	}

	.cycle-block--right {
		background: hsl(150 60% 45% / 0.5);
		border: 1px solid hsl(150 60% 45% / 0.8);
	}

	.cycle-block--selected {
		z-index: 2;
		filter: brightness(1.3);
		outline: 2px solid var(--primary);
		outline-offset: 1px;
	}

	.cycle-block:hover {
		filter: brightness(1.2);
	}

	.cycle-block-label {
		font-size: 0.5625rem;
		font-weight: 700;
		color: white;
		text-shadow: 0 1px 2px rgba(0, 0, 0, 0.5);
		user-select: none;
		white-space: nowrap;
		overflow: hidden;
	}

	/* ── Scrubber ──────────────────────────────────────────── */

	.scrubber {
		position: absolute;
		top: -6px;
		bottom: -6px;
		z-index: 10;
		cursor: grab;
		transform: translateX(-50%);
		display: flex;
		flex-direction: column;
		align-items: center;
		touch-action: none;
		transition: left 0.06s linear;
	}

	.scrubber:active {
		cursor: grabbing;
		transition: none;
	}

	.scrubber-head {
		width: 12px;
		height: 12px;
		background: var(--primary);
		border-radius: 50%;
		border: 2px solid var(--background);
		box-shadow: 0 0 0 1px color-mix(in oklch, var(--primary) 30%, transparent), 0 1px 4px rgba(0, 0, 0, 0.25);
		flex-shrink: 0;
		transition: transform 0.1s ease;
	}

	.scrubber:hover .scrubber-head {
		transform: scale(1.25);
	}

	.scrubber-line {
		width: 2px;
		flex: 1;
		background: var(--primary);
		border-radius: 1px;
		box-shadow: 0 0 4px color-mix(in oklch, var(--primary) 25%, transparent);
	}

	.timeline-frame-label {
		position: absolute;
		bottom: -1rem;
		transform: translateX(-50%);
		font-family: ui-monospace, monospace;
		font-size: 0.625rem;
		font-weight: 600;
		color: var(--primary);
		pointer-events: none;
		white-space: nowrap;
		background: color-mix(in oklch, var(--primary) 10%, transparent);
		padding: 0.0625rem 0.375rem;
		border-radius: var(--radius-sm);
		transition: left 0.06s linear;
	}

	/* ── Playback Controls ────────────────────────────────── */

	.playback-controls {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 0.5rem 0.5rem 0.25rem;
		flex-shrink: 0;
	}

	.control-group {
		display: flex;
		align-items: center;
		gap: 0.25rem;
	}

	.control-btn {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 2rem;
		height: 2rem;
		border: none;
		background: none;
		cursor: pointer;
		border-radius: var(--radius-sm);
		color: var(--foreground);
		transition: background-color 0.15s ease;
	}

	.control-btn:hover {
		background: var(--muted);
	}

	.control-btn--play {
		width: 2.5rem;
		height: 2.5rem;
		background: var(--primary);
		color: var(--primary-foreground);
		border-radius: 50%;
	}

	.control-btn--play:hover {
		background: color-mix(in oklch, var(--primary) 85%, transparent);
	}

	.time-display {
		display: flex;
		align-items: center;
		gap: 1rem;
		font-family: ui-monospace, monospace;
		font-size: 0.75rem;
	}

	.frame-indicator {
		color: var(--primary);
		font-weight: 600;
	}

	.time-indicator {
		color: var(--muted-foreground);
	}

	/* ── Cycles Panel ─────────────────────────────────────── */

	.cycles-panel {
		display: flex;
		flex-direction: column;
		border: 1px solid var(--border);
		border-radius: var(--radius);
		overflow: hidden;
		min-height: 0;
		background: var(--card);
		box-shadow: 0 1px 3px hsl(0 0% 0% / 0.04);
	}

	.panel-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 0.625rem 0.75rem;
		border-bottom: 1px solid var(--border);
		flex-shrink: 0;
		background: color-mix(in oklch, var(--muted) 25%, transparent);
	}

	.panel-title {
		font-size: 0.8125rem;
		font-weight: 600;
		color: var(--foreground);
		letter-spacing: -0.01em;
	}

	:global(.panel-count-badge) {
		font-size: 0.625rem !important;
		font-variant-numeric: tabular-nums;
	}

	.cycle-list {
		flex: 1;
		overflow-y: auto;
		min-height: 0;
	}

	.cycle-item {
		display: flex;
		align-items: center;
		justify-content: space-between;
		width: 100%;
		padding: 0.5rem 0.75rem;
		border: none;
		background: none;
		cursor: pointer;
		text-align: left;
		transition: all 0.15s ease;
		border-bottom: 1px solid color-mix(in oklch, var(--border) 50%, transparent);
		border-left: 3px solid transparent;
	}

	.cycle-item:hover {
		background: color-mix(in oklch, var(--muted) 35%, transparent);
	}

	.cycle-item--selected {
		background: color-mix(in oklch, var(--primary) 6%, transparent);
		border-left-color: var(--primary);
	}

	.cycle-item-info {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		min-width: 0;
	}

	.cycle-frames {
		font-family: ui-monospace, monospace;
		font-size: 0.75rem;
		font-weight: 500;
	}

	.cycle-duration {
		font-size: 0.6875rem;
		color: var(--muted-foreground);
	}

	.cycle-delete-btn {
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 0.25rem;
		border: none;
		background: none;
		cursor: pointer;
		color: var(--muted-foreground);
		border-radius: var(--radius-sm);
		opacity: 0;
		transition: all 0.15s ease;
	}

	.cycle-item:hover .cycle-delete-btn {
		opacity: 1;
	}

	.cycle-delete-btn:hover {
		color: var(--destructive);
		background: color-mix(in oklch, var(--destructive) 10%, transparent);
	}

	.no-cycles {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		padding: 2rem 1rem;
		color: var(--muted-foreground);
		text-align: center;
	}

	.no-cycles p {
		margin: 0;
		font-size: 0.8125rem;
	}

	.no-cycles-hint {
		font-size: 0.6875rem;
		opacity: 0.7;
		margin-top: 0.25rem !important;
	}

	/* ── Edit Panel ───────────────────────────────────────── */

	.edit-panel {
		border-top: 1px solid var(--border);
		padding: 0.75rem;
		flex-shrink: 0;
		background: color-mix(in oklch, var(--muted) 15%, transparent);
	}

	.edit-title {
		font-size: 0.75rem;
		font-weight: 600;
		margin: 0 0 0.5rem;
		color: var(--primary);
	}

	.edit-fields {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	.edit-field {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 0.5rem;
	}

	.edit-field label {
		font-size: 0.6875rem;
		font-weight: 500;
		color: var(--muted-foreground);
		flex-shrink: 0;
	}

	.edit-input {
		width: 5rem;
		padding: 0.25rem 0.5rem;
		font-family: ui-monospace, monospace;
		font-size: 0.75rem;
		border: 1px solid var(--border);
		border-radius: var(--radius-sm);
		background: var(--background);
		color: var(--foreground);
		text-align: right;
	}

	.edit-input:focus {
		outline: none;
		border-color: var(--primary);
		box-shadow: 0 0 0 2px color-mix(in oklch, var(--primary) 20%, transparent);
	}

	.side-toggle {
		border: none;
		background: none;
		cursor: pointer;
		padding: 0;
	}

	.edit-actions {
		display: flex;
		flex-wrap: wrap;
		gap: 0.375rem;
		margin-top: 0.5rem;
	}

	/* ── Panel Actions ────────────────────────────────────── */

	.panel-actions {
		padding: 0.75rem;
		border-top: 1px solid var(--border);
		flex-shrink: 0;
		background: color-mix(in oklch, var(--muted) 15%, transparent);
	}

	/* ── Footer ───────────────────────────────────────────── */

	.editor-footer {
		display: flex;
		justify-content: flex-end;
		padding-top: 0.75rem;
		border-top: 1px solid var(--border);
		flex-shrink: 0;
	}
</style>
