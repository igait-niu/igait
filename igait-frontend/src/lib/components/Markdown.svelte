<script lang="ts">
	import { marked } from 'marked';
	import DOMPurify from 'dompurify';

	let { content, class: className = '' }: { content: string; class?: string } = $props();

	// Configure marked for GFM (tables, strikethrough, etc.)
	marked.setOptions({
		gfm: true,
		breaks: true
	});

	const html = $derived(DOMPurify.sanitize(marked.parse(content) as string));
</script>

<div class="markdown-content {className}">
	{@html html}
</div>

<style>
	.markdown-content :global(p) {
		margin: 0.25rem 0;
	}

	.markdown-content :global(ul),
	.markdown-content :global(ol) {
		margin: 0.5rem 0;
		padding-left: 1.5rem;
	}

	.markdown-content :global(li) {
		margin: 0.25rem 0;
	}

	.markdown-content :global(table) {
		width: 100%;
		border-collapse: collapse;
		margin: 0.5rem 0;
		font-size: 0.8125rem;
	}

	.markdown-content :global(th),
	.markdown-content :global(td) {
		padding: 0.375rem 0.625rem;
		text-align: left;
		border: 1px solid var(--border);
	}

	.markdown-content :global(th) {
		background-color: var(--muted);
		font-weight: 600;
	}

	.markdown-content :global(code) {
		background-color: var(--muted);
		padding: 0.125rem 0.375rem;
		border-radius: 0.25rem;
		font-family: monospace;
		font-size: 0.875em;
	}

	.markdown-content :global(pre) {
		background-color: var(--muted);
		padding: 0.75rem;
		border-radius: 0.5rem;
		overflow-x: auto;
		margin: 0.5rem 0;
	}

	.markdown-content :global(pre code) {
		background: none;
		padding: 0;
	}

	.markdown-content :global(strong) {
		font-weight: 600;
	}

	.markdown-content :global(a) {
		color: var(--primary);
		text-decoration: underline;
	}

	.markdown-content :global(blockquote) {
		border-left: 3px solid var(--border);
		padding-left: 0.75rem;
		margin: 0.5rem 0;
		color: var(--muted-foreground);
	}
</style>
