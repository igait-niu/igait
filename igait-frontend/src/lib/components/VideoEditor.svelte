<script lang="ts">
	import {
		Loader2,
		Save,
		Play,
		Pause,
		SkipBack,
		SkipForward,
		RotateCw,
		Scissors,
		Crop,
		RotateCcw,
		X
	} from '@lucide/svelte';
	import { Button } from '$lib/components/ui/button';
	import * as Dialog from '$lib/components/ui/dialog';
	import { Badge } from '$lib/components/ui/badge';
	import { saveVideoEdit } from '$lib/api';
	import type { FileEntry, VideoTransform } from '$lib/api';
	import { toast } from 'svelte-sonner';

	interface Props {
		open: boolean;
		onclose: () => void;
		jobId: string;
		frontVideo: FileEntry | null;
		sideVideo: FileEntry | null;
	}

	let { open, onclose, jobId, frontVideo, sideVideo }: Props = $props();

	// ── State ─────────────────────────────────────────────
	let saving = $state(false);
	let videoEl: HTMLVideoElement | undefined = $state(undefined);
	let videoContainerEl: HTMLDivElement | undefined = $state(undefined);
	let isPlaying = $state(false);
	let currentTime = $state(0);
	let frontDuration = $state(0);
	let sideDuration = $state(0);
	let activeTab: 'front' | 'side' = $state('front');
	let hasChanges = $state(false);

	// Per-video transforms
	let frontTransform: VideoTransform = $state({});
	let sideTransform: VideoTransform = $state({});

	// Active editing mode
	let editMode: 'rotate' | 'trim' | 'crop' | null = $state(null);

	// Crop drag state
	let isCropping = $state(false);
	let cropStartX = $state(0);
	let cropStartY = $state(0);
	let cropCurrentX = $state(0);
	let cropCurrentY = $state(0);

	// Scrubber drag state
	let isDraggingScrubber = $state(false);

	// Trim drag state
	let isDraggingTrimStart = $state(false);
	let isDraggingTrimEnd = $state(false);

	// Video natural dimensions (for crop coordinate mapping)
	let videoNaturalWidth = $state(1920);
	let videoNaturalHeight = $state(1080);

	// ── Derived ───────────────────────────────────────────
	const activeVideo = $derived(activeTab === 'front' ? frontVideo : sideVideo);
	const activeTransform = $derived(activeTab === 'front' ? frontTransform : sideTransform);
	const duration = $derived(activeTab === 'front' ? frontDuration : sideDuration);
	const hasFrontTab = $derived(!!frontVideo);
	const hasSideTab = $derived(!!sideVideo);

	const hasAnyEdits = $derived(
		hasTransformEdits(frontTransform) || hasTransformEdits(sideTransform)
	);

	function hasTransformEdits(t: VideoTransform): boolean {
		return !!(
			(t.rotation && t.rotation !== 0) ||
			t.trim_start !== undefined ||
			t.trim_end !== undefined ||
			t.crop_width !== undefined
		);
	}

	// Crop preview rectangle (in % of video element)
	const cropPreview = $derived.by(() => {
		const t = activeTransform;
		if (t.crop_width && t.crop_height) {
			return {
				left: ((t.crop_x ?? 0) / videoNaturalWidth) * 100,
				top: ((t.crop_y ?? 0) / videoNaturalHeight) * 100,
				width: (t.crop_width / videoNaturalWidth) * 100,
				height: (t.crop_height / videoNaturalHeight) * 100
			};
		}
		return null;
	});

	// Current crop drag rectangle (in % of video element)
	const dragCropRect = $derived.by(() => {
		if (!isCropping) return null;
		const left = Math.min(cropStartX, cropCurrentX);
		const top = Math.min(cropStartY, cropCurrentY);
		const width = Math.abs(cropCurrentX - cropStartX);
		const height = Math.abs(cropCurrentY - cropStartY);
		return { left, top, width, height };
	});

	// Trim markers as percentages
	const trimStartPct = $derived(
		duration > 0 && activeTransform.trim_start !== undefined
			? (activeTransform.trim_start / duration) * 100
			: 0
	);
	const trimEndPct = $derived(
		duration > 0 && activeTransform.trim_end !== undefined
			? (activeTransform.trim_end / duration) * 100
			: 100
	);

	// CSS rotation for preview
	const rotationDeg = $derived(activeTransform.rotation ?? 0);

	// ── Lifecycle ─────────────────────────────────────────
	$effect(() => {
		if (open) {
			// Default to first available tab
			if (hasFrontTab) activeTab = 'front';
			else if (hasSideTab) activeTab = 'side';
		} else {
			// Reset on close
			frontTransform = {};
			sideTransform = {};
			hasChanges = false;
			editMode = null;
			isPlaying = false;
			currentTime = 0;
			isCropping = false;
		}
	});

	// ── Video controls ────────────────────────────────────
	function handleVideoLoaded(e: Event) {
		const video = e.target as HTMLVideoElement;
		if (video.duration) {
			if (activeTab === 'front') frontDuration = video.duration;
			else sideDuration = video.duration;
		}
		videoNaturalWidth = video.videoWidth || 1920;
		videoNaturalHeight = video.videoHeight || 1080;
	}

	function handleTabSwitch(tab: 'front' | 'side') {
		if (tab === activeTab) return;
		if (isPlaying) {
			videoEl?.pause();
			isPlaying = false;
		}
		activeTab = tab;
		currentTime = 0;
		editMode = null;
		isCropping = false;
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
		if (videoEl && !isDraggingScrubber && !isDraggingTrimStart && !isDraggingTrimEnd) {
			currentTime = videoEl.currentTime;
		}
	}

	function handleVideoEnded() {
		isPlaying = false;
	}

	function stepFrame(delta: number) {
		const fps = 60;
		const newTime = Math.max(0, Math.min(duration, currentTime + delta / fps));
		if (videoEl) videoEl.currentTime = newTime;
		currentTime = newTime;
	}

	// ── Timeline click ────────────────────────────────────
	function handleTimelineClick(e: MouseEvent) {
		if (isDraggingTrimStart || isDraggingTrimEnd) return;
		const target = e.currentTarget as HTMLElement;
		const rect = target.getBoundingClientRect();
		const fraction = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width));
		const time = fraction * duration;
		if (videoEl) videoEl.currentTime = time;
		currentTime = time;
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
			const time = fraction * duration;
			if (videoEl) videoEl.currentTime = time;
			currentTime = time;
		};

		const onUp = () => {
			isDraggingScrubber = false;
			window.removeEventListener('mousemove', onMove);
			window.removeEventListener('mouseup', onUp);
		};

		window.addEventListener('mousemove', onMove);
		window.addEventListener('mouseup', onUp);
	}

	// ── Rotation ──────────────────────────────────────────
	function rotateCW() {
		const current = activeTransform.rotation ?? 0;
		const next = ((current + 90) % 360) as 0 | 90 | 180 | 270;
		setTransformField('rotation', next);
	}

	function rotateCCW() {
		const current = activeTransform.rotation ?? 0;
		const next = ((current + 270) % 360) as 0 | 90 | 180 | 270;
		setTransformField('rotation', next);
	}

	// ── Trim ──────────────────────────────────────────────
	function setTrimStart() {
		setTransformField('trim_start', currentTime);
	}

	function setTrimEnd() {
		setTransformField('trim_end', currentTime);
	}

	function clearTrim() {
		setTransformField('trim_start', undefined);
		setTransformField('trim_end', undefined);
	}

	// Trim handle dragging
	function handleTrimStartDown(e: MouseEvent) {
		e.preventDefault();
		e.stopPropagation();
		isDraggingTrimStart = true;
		const timeline = (e.target as HTMLElement).closest('.timeline') as HTMLElement;
		if (!timeline) return;

		const onMove = (ev: MouseEvent) => {
			const rect = timeline.getBoundingClientRect();
			const fraction = Math.max(0, Math.min(1, (ev.clientX - rect.left) / rect.width));
			const time = fraction * duration;
			setTransformField('trim_start', Math.min(time, (activeTransform.trim_end ?? duration) - 0.1));
		};
		const onUp = () => {
			isDraggingTrimStart = false;
			window.removeEventListener('mousemove', onMove);
			window.removeEventListener('mouseup', onUp);
		};
		window.addEventListener('mousemove', onMove);
		window.addEventListener('mouseup', onUp);
	}

	function handleTrimEndDown(e: MouseEvent) {
		e.preventDefault();
		e.stopPropagation();
		isDraggingTrimEnd = true;
		const timeline = (e.target as HTMLElement).closest('.timeline') as HTMLElement;
		if (!timeline) return;

		const onMove = (ev: MouseEvent) => {
			const rect = timeline.getBoundingClientRect();
			const fraction = Math.max(0, Math.min(1, (ev.clientX - rect.left) / rect.width));
			const time = fraction * duration;
			setTransformField('trim_end', Math.max(time, (activeTransform.trim_start ?? 0) + 0.1));
		};
		const onUp = () => {
			isDraggingTrimEnd = false;
			window.removeEventListener('mousemove', onMove);
			window.removeEventListener('mouseup', onUp);
		};
		window.addEventListener('mousemove', onMove);
		window.addEventListener('mouseup', onUp);
	}

	// ── Crop ──────────────────────────────────────────────
	function handleCropMouseDown(e: MouseEvent) {
		if (editMode !== 'crop' || !videoContainerEl) return;
		e.preventDefault();
		const rect = videoContainerEl.getBoundingClientRect();
		const x = ((e.clientX - rect.left) / rect.width) * 100;
		const y = ((e.clientY - rect.top) / rect.height) * 100;
		cropStartX = x;
		cropStartY = y;
		cropCurrentX = x;
		cropCurrentY = y;
		isCropping = true;

		const onMove = (ev: MouseEvent) => {
			cropCurrentX = Math.max(0, Math.min(100, ((ev.clientX - rect.left) / rect.width) * 100));
			cropCurrentY = Math.max(0, Math.min(100, ((ev.clientY - rect.top) / rect.height) * 100));
		};

		const onUp = () => {
			isCropping = false;
			window.removeEventListener('mousemove', onMove);
			window.removeEventListener('mouseup', onUp);

			// Convert % to pixel coordinates
			const left = Math.min(cropStartX, cropCurrentX);
			const top = Math.min(cropStartY, cropCurrentY);
			const width = Math.abs(cropCurrentX - cropStartX);
			const height = Math.abs(cropCurrentY - cropStartY);

			if (width > 1 && height > 1) {
				const pxX = Math.round((left / 100) * videoNaturalWidth);
				const pxY = Math.round((top / 100) * videoNaturalHeight);
				const pxW = Math.round((width / 100) * videoNaturalWidth);
				const pxH = Math.round((height / 100) * videoNaturalHeight);
				setTransformField('crop_x', pxX);
				setTransformField('crop_y', pxY);
				setTransformField('crop_width', pxW);
				setTransformField('crop_height', pxH);
			}
		};

		window.addEventListener('mousemove', onMove);
		window.addEventListener('mouseup', onUp);
	}

	function clearCrop() {
		setTransformField('crop_x', undefined);
		setTransformField('crop_y', undefined);
		setTransformField('crop_width', undefined);
		setTransformField('crop_height', undefined);
	}

	// ── Transform helpers ─────────────────────────────────
	function setTransformField(field: keyof VideoTransform, value: number | undefined) {
		if (activeTab === 'front') {
			frontTransform = { ...frontTransform, [field]: value };
		} else {
			sideTransform = { ...sideTransform, [field]: value };
		}
		hasChanges = true;
	}

	// ── Save ──────────────────────────────────────────────
	async function handleSave() {
		saving = true;
		try {
			const edits: import('$lib/api').VideoEditRequest = {};
			if (hasTransformEdits(frontTransform)) edits.front = frontTransform;
			if (hasTransformEdits(sideTransform)) edits.side = sideTransform;

			const result = await saveVideoEdit(jobId, edits);

			if (result.isOk()) {
				toast.success(result.value.message);
				hasChanges = false;
				onclose();
			} else {
				toast.error(`Failed to save: ${result.error.rootCause}`);
			}
		} catch (err) {
			toast.error(`Failed to save: ${err instanceof Error ? err.message : 'Unknown error'}`);
		} finally {
			saving = false;
		}
	}

	function handleClose() {
		if (hasChanges) {
			if (!confirm('You have unsaved changes. Are you sure you want to close?')) return;
		}
		onclose();
	}

	// ── Formatting ────────────────────────────────────────
	function formatTime(seconds: number): string {
		const mins = Math.floor(seconds / 60);
		const secs = Math.floor(seconds % 60);
		const ms = Math.floor((seconds % 1) * 100);
		return `${mins}:${secs.toString().padStart(2, '0')}.${ms.toString().padStart(2, '0')}`;
	}

	function getPlayheadPosition(): string {
		if (duration <= 0) return '0%';
		return `${(currentTime / duration) * 100}%`;
	}
</script>

<Dialog.Root
	open={open}
	onOpenChange={(o) => { if (!o) handleClose(); }}
>
	<Dialog.Content class="video-editor-dialog sm:max-w-[95vw] max-h-[95vh]">
		<Dialog.Header>
			<Dialog.Title class="video-editor-title">
				Video Editor
				<Badge variant="outline" class="ml-2">Stage 1</Badge>
				{#if hasChanges}
					<Badge variant="destructive" class="ml-2">Unsaved</Badge>
				{/if}
			</Dialog.Title>
			<Dialog.Description class="sr-only">
				Edit video rotation, trim, and crop before re-processing
			</Dialog.Description>
		</Dialog.Header>

		<div class="editor-body">
			<!-- Video + Timeline Section -->
			<div class="video-section">
				<!-- Front / Side Tabs -->
				<div class="video-tabs">
					{#if hasFrontTab}
						<button
							class="video-tab"
							class:active={activeTab === 'front'}
							onclick={() => handleTabSwitch('front')}
						>
							Front View
							{#if hasTransformEdits(frontTransform)}
								<Badge variant="secondary" class="ml-1.5 video-tab-badge">edited</Badge>
							{/if}
						</button>
					{/if}
					{#if hasSideTab}
						<button
							class="video-tab"
							class:active={activeTab === 'side'}
							onclick={() => handleTabSwitch('side')}
						>
							Side View
							{#if hasTransformEdits(sideTransform)}
								<Badge variant="secondary" class="ml-1.5 video-tab-badge">edited</Badge>
							{/if}
						</button>
					{/if}
				</div>

				<!-- Video container with crop overlay -->
				<div
					class="video-wrapper"
					class:crop-mode={editMode === 'crop'}
					bind:this={videoContainerEl}
					onmousedown={handleCropMouseDown}
					role="presentation"
				>
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
								style="transform: rotate({rotationDeg}deg)"
							></video>
						{/key}
					{:else}
						<div class="no-video">
							<p>No {activeTab} video available</p>
						</div>
					{/if}

					<!-- Crop overlay (existing crop) -->
					{#if cropPreview && editMode === 'crop'}
						<div
							class="crop-overlay"
							style="left: {cropPreview.left}%; top: {cropPreview.top}%; width: {cropPreview.width}%; height: {cropPreview.height}%"
						></div>
					{/if}

					<!-- Crop drag preview -->
					{#if dragCropRect}
						<div
							class="crop-drag-rect"
							style="left: {dragCropRect.left}%; top: {dragCropRect.top}%; width: {dragCropRect.width}%; height: {dragCropRect.height}%"
						></div>
					{/if}
				</div>

				<!-- Timeline -->
				<!-- svelte-ignore a11y_click_events_have_key_events -->
				<!-- svelte-ignore a11y_no_static_element_interactions -->
				<div class="timeline" onclick={handleTimelineClick}>
					<div class="timeline-track">
						<!-- Trim region highlight -->
						{#if activeTransform.trim_start !== undefined || activeTransform.trim_end !== undefined}
							<div
								class="trim-region"
								style="left: {trimStartPct}%; width: {trimEndPct - trimStartPct}%"
							></div>
							<!-- Trim start handle -->
							<!-- svelte-ignore a11y_no_static_element_interactions -->
							<div
								class="trim-handle trim-handle--start"
								style="left: {trimStartPct}%"
								onmousedown={handleTrimStartDown}
							>
								<div class="trim-handle-bar"></div>
							</div>
							<!-- Trim end handle -->
							<!-- svelte-ignore a11y_no_static_element_interactions -->
							<div
								class="trim-handle trim-handle--end"
								style="left: {trimEndPct}%"
								onmousedown={handleTrimEndDown}
							>
								<div class="trim-handle-bar"></div>
							</div>
						{/if}

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
					<div class="timeline-time-label" style="left: {getPlayheadPosition()}">
						{formatTime(currentTime)}
					</div>
				</div>

				<!-- Playback Controls -->
				<div class="playback-controls">
					<div class="control-group">
						<button class="control-btn" onclick={() => stepFrame(-5)} title="Back 5 frames">
							<SkipBack class="h-4 w-4" />
						</button>
						<button
							class="control-btn control-btn--play"
							onclick={togglePlayback}
							title={isPlaying ? 'Pause' : 'Play'}
						>
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
						<span class="time-indicator"
							>{formatTime(currentTime)} / {formatTime(duration)}</span
						>
					</div>
				</div>
			</div>

			<!-- Controls Panel -->
			<div class="controls-panel">
				<div class="panel-header">
					<span class="panel-title">Tools</span>
				</div>

				<!-- Tool selectors -->
				<div class="tool-buttons">
					<button
						class="tool-btn"
						class:active={editMode === 'rotate'}
						onclick={() => (editMode = editMode === 'rotate' ? null : 'rotate')}
					>
						<RotateCw class="h-4 w-4" />
						Rotate
					</button>
					<button
						class="tool-btn"
						class:active={editMode === 'trim'}
						onclick={() => (editMode = editMode === 'trim' ? null : 'trim')}
					>
						<Scissors class="h-4 w-4" />
						Trim
					</button>
					<button
						class="tool-btn"
						class:active={editMode === 'crop'}
						onclick={() => (editMode = editMode === 'crop' ? null : 'crop')}
					>
						<Crop class="h-4 w-4" />
						Crop
					</button>
				</div>

				<!-- Rotation controls -->
				{#if editMode === 'rotate'}
					<div class="tool-panel">
						<h4 class="tool-panel-title">Rotation</h4>
						<div class="rotation-controls">
							<Button variant="outline" size="sm" onclick={rotateCCW}>
								<RotateCcw class="h-4 w-4 mr-1" />
								90° CCW
							</Button>
							<span class="rotation-value">{activeTransform.rotation ?? 0}°</span>
							<Button variant="outline" size="sm" onclick={rotateCW}>
								<RotateCw class="h-4 w-4 mr-1" />
								90° CW
							</Button>
						</div>
						{#if activeTransform.rotation && activeTransform.rotation !== 0}
							<Button
								variant="ghost"
								size="sm"
								class="w-full mt-2"
								onclick={() => setTransformField('rotation', 0)}
							>
								<X class="h-3.5 w-3.5 mr-1" /> Reset Rotation
							</Button>
						{/if}
					</div>
				{/if}

				<!-- Trim controls -->
				{#if editMode === 'trim'}
					<div class="tool-panel">
						<h4 class="tool-panel-title">Trim</h4>
						<p class="tool-panel-hint">
							Seek to the desired position, then click "Set Start" or "Set End". You can
							also drag the trim handles on the timeline.
						</p>
						<div class="trim-controls">
							<Button variant="outline" size="sm" onclick={setTrimStart}>
								Set Start ({formatTime(currentTime)})
							</Button>
							<Button variant="outline" size="sm" onclick={setTrimEnd}>
								Set End ({formatTime(currentTime)})
							</Button>
						</div>
						{#if activeTransform.trim_start !== undefined || activeTransform.trim_end !== undefined}
							<div class="trim-info">
								<span>
									{formatTime(activeTransform.trim_start ?? 0)} → {formatTime(
										activeTransform.trim_end ?? duration
									)}
								</span>
								<span class="trim-duration">
									({formatTime(
										(activeTransform.trim_end ?? duration) -
											(activeTransform.trim_start ?? 0)
									)}
									kept)
								</span>
							</div>
							<Button variant="ghost" size="sm" class="w-full mt-1" onclick={clearTrim}>
								<X class="h-3.5 w-3.5 mr-1" /> Reset Trim
							</Button>
						{/if}
					</div>
				{/if}

				<!-- Crop controls -->
				{#if editMode === 'crop'}
					<div class="tool-panel">
						<h4 class="tool-panel-title">Crop</h4>
						<p class="tool-panel-hint">
							Click and drag on the video to select the crop region.
						</p>
						{#if activeTransform.crop_width && activeTransform.crop_height}
							<div class="crop-info">
								<div class="crop-info-row">
									<span class="crop-label">Position:</span>
									<span>{activeTransform.crop_x ?? 0}, {activeTransform.crop_y ?? 0}</span>
								</div>
								<div class="crop-info-row">
									<span class="crop-label">Size:</span>
									<span>{activeTransform.crop_width} × {activeTransform.crop_height}</span>
								</div>
							</div>
							<Button variant="ghost" size="sm" class="w-full mt-1" onclick={clearCrop}>
								<X class="h-3.5 w-3.5 mr-1" /> Reset Crop
							</Button>
						{:else}
							<div class="crop-empty">No crop set</div>
						{/if}
					</div>
				{/if}

				<!-- Summary -->
				{#if hasAnyEdits}
					<div class="edit-summary">
						<h4 class="tool-panel-title">Summary</h4>
						{#if hasTransformEdits(frontTransform)}
							<div class="summary-item">
								<Badge variant="outline">Front</Badge>
								<span class="summary-details">
									{#if frontTransform.rotation}R:{frontTransform.rotation}°{/if}
									{#if frontTransform.trim_start !== undefined || frontTransform.trim_end !== undefined}
										T:{formatTime(frontTransform.trim_start ?? 0)}-{formatTime(frontTransform.trim_end ?? frontDuration)}
									{/if}
									{#if frontTransform.crop_width}C:{frontTransform.crop_width}×{frontTransform.crop_height}{/if}
								</span>
							</div>
						{/if}
						{#if hasTransformEdits(sideTransform)}
							<div class="summary-item">
								<Badge variant="outline">Side</Badge>
								<span class="summary-details">
									{#if sideTransform.rotation}R:{sideTransform.rotation}°{/if}
									{#if sideTransform.trim_start !== undefined || sideTransform.trim_end !== undefined}
										T:{formatTime(sideTransform.trim_start ?? 0)}-{formatTime(sideTransform.trim_end ?? sideDuration)}
									{/if}
									{#if sideTransform.crop_width}C:{sideTransform.crop_width}×{sideTransform.crop_height}{/if}
								</span>
							</div>
						{/if}
					</div>
				{/if}
			</div>
		</div>

		<div class="editor-footer">
			<p class="footer-warning">
				Saving will re-process the entire pipeline from Stage 1.
			</p>
			<Button
				variant="default"
				size="sm"
				onclick={handleSave}
				disabled={saving || !hasAnyEdits}
			>
				{#if saving}
					<Loader2 class="h-4 w-4 mr-1 animate-spin" />
					Saving…
				{:else}
					<Save class="h-4 w-4 mr-1" />
					Save & Re-Process
				{/if}
			</Button>
		</div>
	</Dialog.Content>
</Dialog.Root>

<style>
	:global(.video-editor-dialog) {
		display: flex;
		flex-direction: column;
		overflow: hidden;
		width: 95vw !important;
		max-width: 95vw !important;
		height: 90vh;
		max-height: 90vh;
	}

	:global(.video-editor-title) {
		display: flex;
		align-items: center;
		font-size: 1rem !important;
	}

	/* ── Body ─────────────────────────────────────────────── */

	.editor-body {
		display: grid;
		grid-template-columns: minmax(0, 1fr) 280px;
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
		font-weight: 500;
		color: hsl(var(--muted-foreground));
		border-radius: var(--radius) var(--radius) 0 0;
		transition:
			background-color 0.15s,
			color 0.15s;
	}

	.video-tab:hover {
		color: hsl(var(--foreground));
	}

	.video-tab.active {
		background: hsl(var(--background));
		color: hsl(var(--foreground));
		border-color: hsl(var(--border));
	}

	:global(.video-tab-badge) {
		font-size: 0.6875rem !important;
		padding: 0 0.375rem !important;
		height: 1.125rem !important;
	}

	.video-wrapper {
		position: relative;
		flex: 1;
		min-height: 0;
		background: hsl(var(--muted));
		border: 1px solid hsl(var(--border));
		border-top: none;
		display: flex;
		align-items: center;
		justify-content: center;
		overflow: hidden;
	}

	.video-wrapper.crop-mode {
		cursor: crosshair;
	}

	.video-player {
		max-width: 100%;
		max-height: 100%;
		object-fit: contain;
		transition: transform 0.3s ease;
	}

	.no-video {
		display: flex;
		align-items: center;
		justify-content: center;
		color: hsl(var(--muted-foreground));
		font-size: 0.875rem;
		padding: 2rem;
	}

	/* ── Crop overlays ────────────────────────────────────── */

	.crop-overlay {
		position: absolute;
		border: 2px solid hsl(var(--primary));
		background: hsla(var(--primary) / 0.15);
		pointer-events: none;
		z-index: 10;
	}

	.crop-drag-rect {
		position: absolute;
		border: 2px dashed hsl(var(--primary));
		background: hsla(var(--primary) / 0.1);
		pointer-events: none;
		z-index: 11;
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

	.trim-region {
		position: absolute;
		top: 2px;
		bottom: 2px;
		background: hsl(var(--primary) / 0.2);
		border-left: 2px solid hsl(var(--primary));
		border-right: 2px solid hsl(var(--primary));
		border-radius: 3px;
		pointer-events: none;
		z-index: 1;
	}

	.trim-handle {
		position: absolute;
		top: -4px;
		bottom: -4px;
		width: 14px;
		cursor: ew-resize;
		z-index: 5;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.trim-handle--start {
		transform: translateX(-50%);
	}

	.trim-handle--end {
		transform: translateX(-50%);
	}

	.trim-handle-bar {
		width: 4px;
		height: 100%;
		background: hsl(var(--primary));
		border-radius: 2px;
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
		box-shadow:
			0 0 0 1px color-mix(in oklch, var(--primary) 30%, transparent),
			0 1px 4px rgba(0, 0, 0, 0.25);
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

	.timeline-time-label {
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
		padding: 0.25rem 0;
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
		padding: 0.375rem;
		border: 1px solid hsl(var(--border));
		border-radius: var(--radius);
		background: hsl(var(--background));
		color: hsl(var(--foreground));
		cursor: pointer;
		transition: background-color 0.15s;
	}

	.control-btn:hover {
		background: hsl(var(--muted));
	}

	.control-btn--play {
		padding: 0.5rem;
	}

	.time-display {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		font-size: 0.75rem;
		color: hsl(var(--muted-foreground));
		font-variant-numeric: tabular-nums;
	}

	.time-indicator {
		font-family: monospace;
	}

	/* ── Controls Panel ───────────────────────────────────── */

	.controls-panel {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
		overflow-y: auto;
		overflow-x: hidden;
		padding: 0.5rem;
		border: 1px solid hsl(var(--border));
		border-radius: var(--radius);
		background: hsl(var(--background));
		min-width: 0;
	}

	.panel-header {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding-bottom: 0.5rem;
		border-bottom: 1px solid hsl(var(--border));
	}

	.panel-title {
		font-weight: 600;
		font-size: 0.875rem;
	}

	.tool-buttons {
		display: flex;
		gap: 0.375rem;
	}

	.tool-btn {
		flex: 1;
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 0.375rem;
		padding: 0.5rem;
		border: 1px solid hsl(var(--border));
		border-radius: var(--radius);
		background: transparent;
		font-size: 0.75rem;
		font-weight: 500;
		cursor: pointer;
		color: hsl(var(--muted-foreground));
		transition:
			background-color 0.15s,
			color 0.15s,
			border-color 0.15s;
	}

	.tool-btn:hover {
		background: hsl(var(--muted));
		color: hsl(var(--foreground));
	}

	.tool-btn.active {
		background: hsl(var(--primary));
		color: hsl(var(--primary-foreground));
		border-color: hsl(var(--primary));
	}

	.tool-panel {
		padding: 0.75rem;
		border: 1px solid hsl(var(--border));
		border-radius: var(--radius);
		background: color-mix(in oklch, var(--muted) 30%, transparent);
	}

	.tool-panel-title {
		font-weight: 600;
		font-size: 0.8125rem;
		margin: 0 0 0.5rem;
	}

	.tool-panel-hint {
		font-size: 0.75rem;
		color: hsl(var(--muted-foreground));
		margin: 0 0 0.5rem;
		line-height: 1.4;
	}

	/* ── Rotation ─────────────────────────────────────────── */

	.rotation-controls {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 0.5rem;
	}

	.rotation-value {
		font-weight: 700;
		font-size: 1rem;
		font-variant-numeric: tabular-nums;
		min-width: 3rem;
		text-align: center;
	}

	/* ── Trim ─────────────────────────────────────────────── */

	.trim-controls {
		display: flex;
		gap: 0.375rem;
	}

	.trim-info {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		margin-top: 0.5rem;
		font-size: 0.75rem;
		font-family: monospace;
		color: hsl(var(--muted-foreground));
	}

	.trim-duration {
		color: hsl(var(--primary));
		font-weight: 500;
	}

	/* ── Crop ─────────────────────────────────────────────── */

	.crop-info {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
		font-size: 0.75rem;
		font-family: monospace;
	}

	.crop-info-row {
		display: flex;
		gap: 0.5rem;
	}

	.crop-label {
		color: hsl(var(--muted-foreground));
		min-width: 4rem;
	}

	.crop-empty {
		font-size: 0.75rem;
		color: hsl(var(--muted-foreground));
		font-style: italic;
	}

	/* ── Summary ──────────────────────────────────────────── */

	.edit-summary {
		padding: 0.75rem;
		border: 1px solid hsl(var(--border));
		border-radius: var(--radius);
		background: color-mix(in oklch, var(--muted) 30%, transparent);
	}

	.summary-item {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		font-size: 0.75rem;
		margin-top: 0.375rem;
	}

	.summary-details {
		font-family: monospace;
		color: hsl(var(--muted-foreground));
	}

	/* ── Footer ───────────────────────────────────────────── */

	.editor-footer {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding-top: 0.75rem;
		border-top: 1px solid hsl(var(--border));
		flex-shrink: 0;
	}

	.footer-warning {
		font-size: 0.75rem;
		color: hsl(var(--muted-foreground));
		margin: 0;
	}
</style>
