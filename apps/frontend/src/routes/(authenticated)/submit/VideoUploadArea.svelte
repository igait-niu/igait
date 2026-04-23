<script lang="ts">
	import { Label } from '$lib/components/ui/label';
	import { Video, CheckCircle } from '@lucide/svelte';

	interface Props {
		label: string;
		id: string;
		file: File | undefined;
		disabled: boolean;
		onchange: (e: Event) => void;
	}

	let { label, id, file = $bindable(), disabled, onchange }: Props = $props();

	let isDragging = $state(false);

	function handleDragOver(e: DragEvent) {
		if (disabled) return;
		e.preventDefault();
		e.stopPropagation();
		isDragging = true;
	}

	function handleDragLeave(e: DragEvent) {
		if (disabled) return;
		e.preventDefault();
		e.stopPropagation();
		isDragging = false;
	}

	function handleDrop(e: DragEvent) {
		if (disabled) return;
		e.preventDefault();
		e.stopPropagation();
		isDragging = false;

		const files = e.dataTransfer?.files;
		if (files && files.length > 0) {
			const droppedFile = files[0];
			// Check if it's a video file
			if (droppedFile.type.startsWith('video/')) {
				file = droppedFile;
				// Trigger the onchange handler with a synthetic event
				const syntheticEvent = new Event('change', { bubbles: true });
				Object.defineProperty(syntheticEvent, 'target', {
					value: { files: [droppedFile] },
					enumerable: true
				});
				onchange(syntheticEvent);
			}
		}
	}
</script>

<div class="form-group">
	<Label for={id}>{label}</Label>
	<div class="upload-container">
		<label
			for={id}
			class="upload-area"
			class:has-file={file}
			class:disabled
			class:dragging={isDragging}
			ondragover={handleDragOver}
			ondragleave={handleDragLeave}
			ondrop={handleDrop}
		>
			<input {id} type="file" accept="video/*" class="hidden" {onchange} {disabled} />
			{#if file}
				<CheckCircle class="upload-icon-success" />
				<span class="upload-filename">{file.name}</span>
				<span class="upload-filesize">
					{(file.size / 1024 / 1024).toFixed(1)} MB
				</span>
			{:else}
				<Video class="upload-icon" />
				<span class="upload-placeholder">
					{isDragging ? 'Drop video here' : `Click or drag to upload ${label.toLowerCase()}`}
				</span>
			{/if}
		</label>
	</div>
</div>

<style>
	.form-group {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-xs);
	}

	.upload-container {
		position: relative;
	}

	.upload-area {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		padding: 1.25rem;
		border: 2px dashed var(--border);
		border-radius: var(--radius-lg);
		cursor: pointer;
		transition: all 0.2s;
		background-color: color-mix(in oklch, var(--muted) 30%, transparent);
		min-height: 140px;
		gap: 0.5rem;
	}

	.upload-area:hover:not(.disabled) {
		border-color: var(--primary);
		background-color: color-mix(in oklch, var(--primary) 5%, transparent);
	}

	.upload-area.has-file {
		border-color: var(--primary);
		background-color: color-mix(in oklch, var(--primary) 10%, transparent);
	}

	.upload-area.disabled {
		opacity: 0.6;
		cursor: not-allowed;
	}

	.upload-area.dragging {
		border-color: var(--primary);
		background-color: color-mix(in oklch, var(--primary) 15%, transparent);
		transform: scale(1.02);
	}

	:global(.upload-icon) {
		width: 2.5rem;
		height: 2.5rem;
		color: var(--muted-foreground);
		transition: transform 0.2s;
	}

	.dragging :global(.upload-icon) {
		transform: scale(1.1);
		color: var(--primary);
	}

	:global(.upload-icon-success) {
		width: 2.5rem;
		height: 2.5rem;
		color: var(--primary);
	}

	.upload-placeholder {
		font-size: 0.8125rem;
		color: var(--muted-foreground);
		text-align: center;
	}

	.upload-filename {
		font-weight: 500;
		font-size: 0.875rem;
		text-align: center;
		word-break: break-word;
		max-width: 100%;
	}

	.upload-filesize {
		font-size: 0.75rem;
		color: var(--muted-foreground);
	}

	:global(.upload-icon-success) {
		width: 3rem;
		height: 3rem;
		color: var(--primary);
	}

	.upload-placeholder {
		font-size: 0.875rem;
		color: var(--muted-foreground);
	}

	.upload-filename {
		font-weight: 500;
		text-align: center;
		word-break: break-word;
	}

	.upload-filesize {
		font-size: 0.875rem;
		color: var(--muted-foreground);
	}

	.hidden {
		position: absolute;
		width: 1px;
		height: 1px;
		padding: 0;
		margin: -1px;
		overflow: hidden;
		clip: rect(0, 0, 0, 0);
		white-space: nowrap;
		border-width: 0;
	}
</style>
