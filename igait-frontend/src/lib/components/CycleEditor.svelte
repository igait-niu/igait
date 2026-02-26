<script lang="ts">
	import { Loader2, Plus, Trash2, Save, X, Play, Pause, SkipBack, SkipForward } from '@lucide/svelte';
	import { Button } from '$lib/components/ui/button';
	import * as Dialog from '$lib/components/ui/dialog';
	import { Badge } from '$lib/components/ui/badge';
	import { updateCycles } from '$lib/api';
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
	let frontVideoEl: HTMLVideoElement | undefined = $state(undefined);
	let sideVideoEl: HTMLVideoElement | undefined = $state(undefined);
	let isPlaying = $state(false);
	let currentTime = $state(0);
	let duration = $state(0);
	let frontTotalFrames = $state(0);
	let sideTotalFrames = $state(0);
	let activeTab: 'front' | 'side' = $state('front');
	let selectedCycleIndex: number | null = $state(null);
	let editingCycle: GaitCycle | null = $state(null);
	let hasChanges = $state(false);
	let fps = 30; // Default framerate assumption

	// ── Derived ───────────────────────────────────────────
	const activeCycles = $derived(activeTab === 'front' ? frontCycles : sideCycles);
	const activeJsonFile = $derived(activeTab === 'front' ? frontJsonFile : sideJsonFile);
	const totalFrames = $derived(activeTab === 'front' ? frontTotalFrames : sideTotalFrames);
	const currentFrame = $derived(Math.round(currentTime * fps));

	// ── Load cycle data ───────────────────────────────────
	$effect(() => {
		if (open) {
			loadCycleData();
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
							// Try to deduce total frames from landmark data length
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
		if (video.duration && video.duration > duration) {
			duration = video.duration;
		}
	}

	function togglePlayback() {
		if (!frontVideoEl && !sideVideoEl) return;

		if (isPlaying) {
			frontVideoEl?.pause();
			sideVideoEl?.pause();
			isPlaying = false;
		} else {
			frontVideoEl?.play();
			sideVideoEl?.play();
			isPlaying = true;
		}
	}

	function handleTimeUpdate() {
		if (frontVideoEl) {
			currentTime = frontVideoEl.currentTime;
		} else if (sideVideoEl) {
			currentTime = sideVideoEl.currentTime;
		}
	}

	function handleVideoEnded() {
		isPlaying = false;
	}

	function seekToFrame(frame: number) {
		const time = frame / fps;
		if (frontVideoEl) frontVideoEl.currentTime = time;
		if (sideVideoEl) sideVideoEl.currentTime = time;
		currentTime = time;
	}

	function seekToTime(time: number) {
		if (frontVideoEl) frontVideoEl.currentTime = time;
		if (sideVideoEl) sideVideoEl.currentTime = time;
		currentTime = time;
	}

	function stepFrame(delta: number) {
		const newFrame = Math.max(0, currentFrame + delta);
		seekToFrame(newFrame);
	}

	function handleTimelineClick(e: MouseEvent) {
		const target = e.currentTarget as HTMLElement;
		const rect = target.getBoundingClientRect();
		const fraction = (e.clientX - rect.left) / rect.width;
		const maxFrames = totalFrames || Math.round(duration * fps);
		if (maxFrames > 0) {
			seekToFrame(Math.round(fraction * maxFrames));
		}
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
		selectedCycleIndex = activeCycles.length; // Select the newly added one
		editingCycle = { ...newCycle };
	}

	function selectCycle(index: number) {
		if (selectedCycleIndex === index) {
			selectedCycleIndex = null;
			editingCycle = null;
		} else {
			selectedCycleIndex = index;
			editingCycle = { ...activeCycles[index] };
			// Seek to cycle start
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
			toast.success('Cycle data saved successfully!');
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
				<!-- Video Players -->
				<div class="video-section">
					<div class="video-grid">
						{#if frontVideo}
							<div class="video-wrapper">
								<span class="video-label">Front View</span>
								<!-- svelte-ignore a11y_media_has_caption -->
								<video
									bind:this={frontVideoEl}
									src={frontVideo.url}
									onloadedmetadata={handleVideoLoaded}
									ontimeupdate={handleTimeUpdate}
									onended={handleVideoEnded}
									preload="metadata"
									class="video-player"
								></video>
							</div>
						{/if}
						{#if sideVideo}
							<div class="video-wrapper">
								<span class="video-label">Side View</span>
								<!-- svelte-ignore a11y_media_has_caption -->
								<video
									bind:this={sideVideoEl}
									src={sideVideo.url}
									onloadedmetadata={handleVideoLoaded}
									onended={handleVideoEnded}
									preload="metadata"
									class="video-player"
								></video>
							</div>
						{/if}
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

					<!-- Timeline -->
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

							<!-- Playhead -->
							<div class="playhead" style="left: {getPlayheadPosition()}"></div>
						</div>
					</div>
				</div>

				<!-- Cycle List Panel -->
				<div class="cycles-panel">
					<!-- Tabs: Front / Side -->
					<div class="panel-tabs">
						<button
							class="panel-tab"
							class:active={activeTab === 'front'}
							onclick={() => { activeTab = 'front'; selectedCycleIndex = null; editingCycle = null; }}
						>
							Front ({frontCycles.length})
						</button>
						<button
							class="panel-tab"
							class:active={activeTab === 'side'}
							onclick={() => { activeTab = 'side'; selectedCycleIndex = null; editingCycle = null; }}
						>
							Side ({sideCycles.length})
						</button>
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
			<Button variant="ghost" size="sm" onclick={handleClose}>
				<X class="h-4 w-4 mr-1" />
				Close
			</Button>
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
		color: hsl(var(--muted-foreground));
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
		gap: 0.75rem;
		min-height: 0;
		overflow: hidden;
	}

	.video-grid {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 0.75rem;
		flex: 1;
		min-height: 0;
	}

	.video-wrapper {
		position: relative;
		display: flex;
		flex-direction: column;
		min-height: 0;
		background: hsl(var(--muted) / 0.3);
		border-radius: var(--radius);
		overflow: hidden;
	}

	.video-label {
		position: absolute;
		top: 0.5rem;
		left: 0.5rem;
		z-index: 2;
		font-size: 0.6875rem;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.05em;
		padding: 0.125rem 0.5rem;
		background: hsl(0 0% 0% / 0.6);
		color: white;
		border-radius: var(--radius-sm);
	}

	.video-player {
		width: 100%;
		height: 100%;
		object-fit: contain;
		background: black;
	}

	/* ── Playback Controls ────────────────────────────────── */

	.playback-controls {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 0.375rem 0.5rem;
		background: hsl(var(--muted) / 0.3);
		border-radius: var(--radius);
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
		color: hsl(var(--foreground));
		transition: background-color 0.15s ease;
	}

	.control-btn:hover {
		background: hsl(var(--muted));
	}

	.control-btn--play {
		width: 2.5rem;
		height: 2.5rem;
		background: hsl(var(--primary));
		color: hsl(var(--primary-foreground));
		border-radius: 50%;
	}

	.control-btn--play:hover {
		background: hsl(var(--primary) / 0.85);
	}

	.time-display {
		display: flex;
		align-items: center;
		gap: 1rem;
		font-family: ui-monospace, monospace;
		font-size: 0.75rem;
	}

	.frame-indicator {
		color: hsl(var(--primary));
		font-weight: 600;
	}

	.time-indicator {
		color: hsl(var(--muted-foreground));
	}

	/* ── Timeline ─────────────────────────────────────────── */

	.timeline {
		padding: 0.25rem 0;
		cursor: pointer;
	}

	.timeline-track {
		position: relative;
		height: 2.5rem;
		background: hsl(var(--muted) / 0.4);
		border-radius: var(--radius-sm);
		overflow: hidden;
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
		outline: 2px solid hsl(var(--primary));
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

	.playhead {
		position: absolute;
		top: 0;
		bottom: 0;
		width: 2px;
		background: hsl(var(--destructive));
		z-index: 10;
		pointer-events: none;
		transition: left 0.05s linear;
	}

	.playhead::before {
		content: '';
		position: absolute;
		top: -3px;
		left: -4px;
		width: 10px;
		height: 10px;
		background: hsl(var(--destructive));
		border-radius: 50%;
	}

	/* ── Cycles Panel ─────────────────────────────────────── */

	.cycles-panel {
		display: flex;
		flex-direction: column;
		border: 1px solid hsl(var(--border));
		border-radius: var(--radius);
		overflow: hidden;
		min-height: 0;
	}

	.panel-tabs {
		display: flex;
		border-bottom: 1px solid hsl(var(--border));
		flex-shrink: 0;
	}

	.panel-tab {
		flex: 1;
		padding: 0.5rem;
		border: none;
		background: none;
		cursor: pointer;
		font-size: 0.75rem;
		font-weight: 500;
		color: hsl(var(--muted-foreground));
		transition: all 0.15s ease;
	}

	.panel-tab.active {
		color: hsl(var(--foreground));
		background: hsl(var(--muted) / 0.5);
		box-shadow: inset 0 -2px 0 hsl(var(--primary));
	}

	.panel-tab:hover:not(.active) {
		background: hsl(var(--muted) / 0.3);
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
		transition: background-color 0.15s ease;
		border-bottom: 1px solid hsl(var(--border) / 0.5);
	}

	.cycle-item:hover {
		background: hsl(var(--muted) / 0.4);
	}

	.cycle-item--selected {
		background: hsl(var(--primary) / 0.08);
		border-left: 3px solid hsl(var(--primary));
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
		color: hsl(var(--muted-foreground));
	}

	.cycle-delete-btn {
		display: flex;
		align-items: center;
		justify-content: center;
		padding: 0.25rem;
		border: none;
		background: none;
		cursor: pointer;
		color: hsl(var(--muted-foreground));
		border-radius: var(--radius-sm);
		opacity: 0;
		transition: all 0.15s ease;
	}

	.cycle-item:hover .cycle-delete-btn {
		opacity: 1;
	}

	.cycle-delete-btn:hover {
		color: hsl(var(--destructive));
		background: hsl(var(--destructive) / 0.1);
	}

	.no-cycles {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		padding: 2rem 1rem;
		color: hsl(var(--muted-foreground));
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
		border-top: 1px solid hsl(var(--border));
		padding: 0.75rem;
		flex-shrink: 0;
	}

	.edit-title {
		font-size: 0.75rem;
		font-weight: 600;
		margin: 0 0 0.5rem;
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
		color: hsl(var(--muted-foreground));
		flex-shrink: 0;
	}

	.edit-input {
		width: 5rem;
		padding: 0.25rem 0.5rem;
		font-family: ui-monospace, monospace;
		font-size: 0.75rem;
		border: 1px solid hsl(var(--border));
		border-radius: var(--radius-sm);
		background: hsl(var(--background));
		color: hsl(var(--foreground));
		text-align: right;
	}

	.edit-input:focus {
		outline: none;
		border-color: hsl(var(--primary));
		box-shadow: 0 0 0 2px hsl(var(--primary) / 0.2);
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
		border-top: 1px solid hsl(var(--border));
		flex-shrink: 0;
	}

	/* ── Footer ───────────────────────────────────────────── */

	.editor-footer {
		display: flex;
		justify-content: space-between;
		padding-top: 0.75rem;
		border-top: 1px solid hsl(var(--border));
		flex-shrink: 0;
	}
</style>
