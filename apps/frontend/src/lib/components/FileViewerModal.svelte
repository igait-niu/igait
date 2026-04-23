<script lang="ts">
	import { Loader2, FileText, FileQuestion, Video, Download } from '@lucide/svelte';
	import { Button } from '$lib/components/ui/button';
	import * as Dialog from '$lib/components/ui/dialog';
	import JsonTree from './JsonTree.svelte';
	import type { FileEntry } from '$lib/api';
	import { VALID_VIDEO_EXTENSIONS } from '$lib/api/config';

	interface Props {
		/** The file to display (null = closed) */
		file: FileEntry | null;
		/** Callback to close the modal */
		onclose: () => void;
	}

	let { file, onclose }: Props = $props();

	/** Lazy-loaded content state */
	let contentState: 'idle' | 'loading' | 'loaded' | 'error' = $state('idle');
	let textContent: string = $state('');
	let parsedJson: unknown = $state(undefined);

	/** Get file extension */
	function getExtension(name: string): string {
		const dot = name.lastIndexOf('.');
		return dot === -1 ? '' : name.slice(dot + 1).toLowerCase();
	}

	/** Fetch content when a file is opened */
	$effect(() => {
		if (!file) {
			contentState = 'idle';
			textContent = '';
			parsedJson = undefined;
			return;
		}

		const ext = getExtension(file.name);
		if (ext !== 'json') {
			contentState = 'loaded';
			return;
		}

		// Lazy-load JSON content
		contentState = 'loading';
		const url = file.url;

		fetch(url)
			.then((res) => {
				if (!res.ok) throw new Error(`HTTP ${res.status}`);
				return res.text();
			})
			.then((text) => {
				textContent = text;
				try {
					parsedJson = JSON.parse(text);
				} catch {
					parsedJson = undefined;
				}
				contentState = 'loaded';
			})
			.catch(() => {
				textContent = 'Failed to load file contents.';
				contentState = 'error';
			});
	});

	const VIDEO_EXTS = new Set(VALID_VIDEO_EXTENSIONS.map((e) => e.slice(1)));

	const isOpen = $derived(file !== null);
	const ext = $derived(file ? getExtension(file.name) : '');
	const isVideo = $derived(VIDEO_EXTS.has(ext));
</script>

<Dialog.Root
	open={isOpen}
	onOpenChange={(open) => {
		if (!open) onclose();
	}}
>
	<Dialog.Content class="file-modal-content max-h-[85vh] sm:max-w-[700px]">
		{#if file}
			<Dialog.Header>
				<Dialog.Title class="file-modal-title">
					{#if ext === 'json'}
						<FileText class="file-modal-icon file-modal-icon--json" />
					{:else if isVideo}
						<Video class="file-modal-icon file-modal-icon--video" />
					{:else}
						<FileQuestion class="file-modal-icon" />
					{/if}
					<span class="file-modal-name">{file.name}</span>
				</Dialog.Title>
				<Dialog.Description class="sr-only">
					Viewing contents of {file.name}
				</Dialog.Description>
			</Dialog.Header>

			<div class="file-modal-body">
				{#if contentState === 'loading'}
					<div class="file-modal-state">
						<Loader2 class="spinner" />
						<p>Loading file…</p>
					</div>
				{:else if contentState === 'error'}
					<pre class="file-modal-code file-modal-code--error">{textContent}</pre>
				{:else if ext === 'json'}
					{#if parsedJson !== undefined}
						<JsonTree data={parsedJson} />
					{:else}
						<pre class="file-modal-code">{textContent}</pre>
					{/if}
				{:else if isVideo}
					<!-- svelte-ignore a11y_media_has_caption -->
					<video src={file.url} controls preload="metadata" class="file-modal-video"></video>
				{:else}
					<div class="file-modal-placeholder">
						<p class="placeholder-face">:3</p>
						<p class="placeholder-ext">.{ext} viewer not yet implemented</p>
					</div>
				{/if}
			</div>

			<div class="file-modal-footer">
				<Button variant="outline" size="sm" href={file.url} target="_blank" rel="noopener">
					<Download class="mr-1 h-4 w-4" />
					Download
				</Button>
			</div>
		{/if}
	</Dialog.Content>
</Dialog.Root>

<style>
	:global(.file-modal-content) {
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}

	:global(.file-modal-title) {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		font-family: ui-monospace, monospace;
		font-size: 0.875rem !important;
	}

	:global(.file-modal-icon) {
		width: 1.125rem;
		height: 1.125rem;
		flex-shrink: 0;
		color: hsl(var(--muted-foreground));
	}

	:global(.file-modal-icon--json) {
		color: hsl(var(--primary));
	}

	:global(.file-modal-icon--video) {
		color: hsl(var(--primary));
	}

	.file-modal-name {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.file-modal-body {
		flex: 1;
		overflow-y: auto;
		min-height: 120px;
		max-height: 60vh;
		padding: 0.5rem 0;
	}

	.file-modal-state {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		padding: 3rem 2rem;
		text-align: center;
		color: hsl(var(--muted-foreground));
		gap: 0.5rem;
	}

	.file-modal-state p {
		margin: 0;
		font-size: 0.875rem;
	}

	.file-modal-code {
		font-family: ui-monospace, monospace;
		font-size: 0.6875rem;
		line-height: 1.6;
		white-space: pre-wrap;
		word-break: break-word;
		background: hsl(var(--muted) / 0.4);
		padding: 0.75rem 1rem;
		margin: 0;
		border-radius: var(--radius-sm);
	}

	.file-modal-code--error {
		color: hsl(var(--destructive));
		background: hsl(var(--destructive) / 0.06);
	}

	.file-modal-placeholder {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		padding: 3rem 2rem;
		text-align: center;
		color: hsl(var(--muted-foreground));
		gap: 0.25rem;
	}

	.placeholder-face {
		font-size: 2rem;
		opacity: 0.6;
		margin: 0;
	}

	.placeholder-ext {
		font-size: 0.8125rem;
		margin: 0;
		opacity: 0.7;
	}

	.file-modal-video {
		width: 100%;
		max-height: 55vh;
		display: block;
		border-radius: var(--radius-sm);
		background: hsl(var(--muted) / 0.4);
	}

	.file-modal-footer {
		display: flex;
		justify-content: flex-end;
		padding-top: 0.75rem;
		border-top: 1px solid hsl(var(--border));
	}
</style>
