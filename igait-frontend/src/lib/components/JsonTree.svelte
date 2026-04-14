<script lang="ts">
	import { ChevronRight } from '@lucide/svelte';
	import JsonTree from './JsonTree.svelte';

	interface Props {
		/** The JSON data to display */
		data: unknown;
		/** Current nesting depth (used internally for recursion) */
		depth?: number;
		/** The key name if this is a property of an object */
		keyName?: string;
		/** Whether this node starts collapsed */
		defaultCollapsed?: boolean;
	}

	let { data, depth = 0, keyName, defaultCollapsed = false }: Props = $props();

	/** Whether this collapsible node is open — initialised once per mount */
	let userToggled = $state(false);
	let userOpen = $state(true);
	const open = $derived(userToggled ? userOpen : !defaultCollapsed);

	/** Type detection helpers */
	const isObject = $derived(data !== null && typeof data === 'object' && !Array.isArray(data));
	const isArray = $derived(Array.isArray(data));
	const isCollapsible = $derived(isObject || isArray);

	/** Entries for objects/arrays */
	const entries = $derived.by((): [string, unknown][] => {
		if (isArray) return (data as unknown[]).map((v, i) => [String(i), v]);
		if (isObject) return Object.entries(data as Record<string, unknown>);
		return [];
	});

	/** Display count summary when collapsed */
	const summary = $derived.by((): string => {
		if (isArray) return `${(data as unknown[]).length} items`;
		if (isObject) return `${Object.keys(data as Record<string, unknown>).length} keys`;
		return '';
	});

	/** Get CSS class for a primitive value */
	function valueClass(val: unknown): string {
		if (val === null) return 'json-null';
		if (typeof val === 'string') return 'json-string';
		if (typeof val === 'number') return 'json-number';
		if (typeof val === 'boolean') return 'json-boolean';
		return '';
	}

	/** Format a primitive value for display */
	function formatValue(val: unknown): string {
		if (val === null) return 'null';
		if (typeof val === 'string') return `"${val}"`;
		return String(val);
	}

	function toggle() {
		userToggled = true;
		userOpen = !open;
	}
</script>

<div class="json-node" class:json-node--root={depth === 0}>
	{#if isCollapsible}
		<!-- Collapsible object/array -->
		<button class="json-toggle" onclick={toggle} aria-expanded={open}>
			<ChevronRight class="json-chevron {open ? 'json-chevron--open' : ''}" />
			{#if keyName !== undefined}
				<span class="json-key">{keyName}</span><span class="json-colon">:&nbsp;</span>
			{/if}
			<span class="json-bracket">{isArray ? '[' : '{'}</span>
			{#if !open}
				<span class="json-summary">{summary}</span>
				<span class="json-bracket">{isArray ? ']' : '}'}</span>
			{/if}
		</button>

		{#if open}
			<div class="json-children">
				{#each entries as [key, value] (key)}
					<JsonTree data={value} depth={depth + 1} keyName={key} defaultCollapsed={depth >= 0} />
				{/each}
			</div>
			<span class="json-bracket json-bracket--close">{isArray ? ']' : '}'}</span>
		{/if}
	{:else}
		<!-- Primitive value -->
		<span class="json-leaf">
			{#if keyName !== undefined}
				<span class="json-key">{keyName}</span><span class="json-colon">:&nbsp;</span>
			{/if}
			<span class={valueClass(data)}>{formatValue(data)}</span>
		</span>
	{/if}
</div>

<style>
	.json-node {
		font-family: ui-monospace, SFMono-Regular, 'SF Mono', Menlo, Consolas, monospace;
		font-size: 0.75rem;
		line-height: 1.65;
	}

	.json-node--root {
		padding: 0.75rem 1rem;
		background: hsl(var(--muted) / 0.4);
		max-height: 400px;
		overflow-y: auto;
	}

	/* ── Toggle button ─────────────────────────────────── */

	.json-toggle {
		display: inline-flex;
		align-items: center;
		gap: 0;
		background: none;
		border: none;
		padding: 0;
		margin: 0;
		cursor: pointer;
		font: inherit;
		color: inherit;
		text-align: left;
	}

	.json-toggle:hover {
		background: hsl(var(--muted) / 0.5);
		border-radius: 2px;
	}

	:global(.json-chevron) {
		width: 0.875rem;
		height: 0.875rem;
		flex-shrink: 0;
		color: hsl(var(--muted-foreground));
		transition: transform 0.15s ease;
	}

	:global(.json-chevron--open) {
		transform: rotate(90deg);
	}

	/* ── Children indentation ──────────────────────────── */

	.json-children {
		padding-left: 1.25rem;
		border-left: 1px solid hsl(var(--border) / 0.5);
		margin-left: 0.375rem;
	}

	/* ── Syntax highlighting ───────────────────────────── */

	.json-key {
		color: hsl(var(--primary));
	}

	.json-colon {
		color: hsl(var(--muted-foreground));
	}

	.json-bracket {
		color: hsl(var(--muted-foreground));
		font-weight: 600;
	}

	.json-bracket--close {
		margin-left: 0.375rem;
	}

	.json-summary {
		color: hsl(var(--muted-foreground));
		font-style: italic;
		font-size: 0.6875rem;
		margin: 0 0.25rem;
	}

	.json-leaf {
		display: block;
	}

	:global(.json-string) {
		color: hsl(142 76% 36%);
	}

	:global(.json-number) {
		color: hsl(221 83% 53%);
	}

	:global(.json-boolean) {
		color: hsl(262 83% 58%);
	}

	:global(.json-null) {
		color: hsl(var(--muted-foreground));
		font-style: italic;
	}
</style>
