<script lang="ts">
	/**
	 * Generic pipeline-stage tab button. Rendered in a loop over the
	 * registry stages. The ONLY component that renders a stage tab —
	 * there is no per-stage variant.
	 *
	 * Optional props make it work in both contexts we have today:
	 *   - job detail page: `status` drives the status icon + color
	 *   - admin queues page: `count` shows a queue-depth badge
	 */
	import type { StageSpec } from '$lib/stores';
	import type { StageStatus } from '../../types/StageStatus';
	import { Badge } from '$lib/components/ui/badge';
	import { CheckCircle2, Loader2, XCircle, Clock } from '@lucide/svelte';

	type Props = {
		stage: StageSpec;
		active: boolean;
		status?: StageStatus;
		count?: number;
		onclick: () => void;
	};

	let { stage, active, status, count, onclick }: Props = $props();
</script>

<button
	type="button"
	class="stage-tab"
	class:active
	class:status-complete={status === 'complete'}
	class:status-running={status === 'running'}
	class:status-error={status === 'error'}
	class:status-idle={status === 'not_started'}
	title={stage.description}
	{onclick}
>
	{#if status !== undefined}
		<span class="status-icon">
			{#if status === 'complete'}
				<CheckCircle2 class="icon" />
			{:else if status === 'running'}
				<Loader2 class="icon spin" />
			{:else if status === 'error'}
				<XCircle class="icon" />
			{:else}
				<Clock class="icon" />
			{/if}
		</span>
	{/if}
	<span class="label">{stage.display_name}</span>
	{#if count !== undefined && count > 0}
		<Badge variant="default" class="stage-count-badge">{count}</Badge>
	{/if}
</button>

<style>
	.stage-tab {
		position: relative;
		display: inline-flex;
		align-items: center;
		gap: 0.375rem;
		padding: 0.5rem 0.875rem;
		border: 1px solid transparent;
		background: none;
		cursor: pointer;
		color: hsl(var(--muted-foreground));
		transition: all 0.15s ease;
		white-space: nowrap;
		border-radius: var(--radius-sm);
		font-size: 0.8125rem;
		font-weight: 500;
	}

	.stage-tab:hover {
		color: hsl(var(--foreground));
		background-color: hsl(var(--primary) / 0.06);
	}

	.stage-tab.active {
		background-color: hsl(var(--background));
		box-shadow: 0 1px 3px hsl(var(--primary) / 0.1);
		font-weight: 600;
	}

	/* Status-driven color variants (job detail page). */
	.stage-tab.status-complete {
		color: hsl(142 76% 36%);
	}
	.stage-tab.status-complete.active {
		border-color: hsl(142 76% 36% / 0.4);
	}
	.stage-tab.status-running {
		color: hsl(var(--primary));
	}
	.stage-tab.status-running.active {
		border-color: hsl(var(--primary) / 0.4);
	}
	.stage-tab.status-error {
		color: hsl(var(--destructive));
	}
	.stage-tab.status-error.active {
		border-color: hsl(var(--destructive) / 0.4);
	}
	.stage-tab.status-idle {
		color: hsl(var(--muted-foreground));
		opacity: 0.7;
	}
	.stage-tab.status-idle.active {
		border-color: hsl(var(--border));
		opacity: 1;
	}

	/* No-status variant picks up the primary color when active (admin queues). */
	.stage-tab:not(.status-complete):not(.status-running):not(.status-error):not(.status-idle).active {
		color: hsl(var(--primary));
		border-color: hsl(var(--primary) / 0.3);
	}

	.status-icon {
		display: inline-flex;
		align-items: center;
	}

	:global(.stage-tab .icon) {
		width: 0.8125rem;
		height: 0.8125rem;
	}

	:global(.stage-tab .icon.spin) {
		animation: spin 1s linear infinite;
	}

	.label {
		font-size: 0.8125rem;
	}

	:global(.stage-count-badge) {
		position: absolute;
		top: -0.3rem;
		right: -0.3rem;
		font-size: 0.5625rem !important;
		padding: 0 0.3rem !important;
		height: 1rem !important;
		min-width: 1rem !important;
		line-height: 1rem !important;
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
