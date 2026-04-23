<script lang="ts">
	import type { Snippet, Component } from 'svelte';

	interface Props {
		icon: Component<{ class?: string }>;
		title: string;
		description: string;
		variant?: 'default' | 'error' | 'loading';
		children?: Snippet;
	}

	let { icon: Icon, title, description, variant = 'default', children }: Props = $props();
</script>

<div class="empty-state">
	<div
		class="empty-state__icon"
		class:empty-state__icon--error={variant === 'error'}
		class:empty-state__icon--loading={variant === 'loading'}
	>
		<Icon class="icon" />
	</div>
	<h3 class="empty-state__title">{title}</h3>
	<p class="empty-state__description">{description}</p>
	{#if children}
		<div class="empty-state__actions">
			{@render children()}
		</div>
	{/if}
</div>

<style>
	.empty-state {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		padding: var(--spacing-xl) var(--spacing-md);
		text-align: center;
	}

	.empty-state__icon {
		margin-bottom: 1rem;
		color: hsl(var(--muted-foreground));
	}

	.empty-state__icon--error {
		color: hsl(var(--destructive));
	}

	.empty-state__icon--loading :global(.icon) {
		animation: spin 1s linear infinite;
	}

	.empty-state__icon :global(.icon) {
		height: 3rem;
		width: 3rem;
	}

	.empty-state__title {
		font-weight: 600;
		margin-bottom: 0.25rem;
	}

	.empty-state__description {
		font-size: 0.875rem;
		color: hsl(var(--muted-foreground));
		max-width: 280px;
	}

	.empty-state__actions {
		margin-top: 1rem;
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
