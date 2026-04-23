<script lang="ts">
	import { ArrowRight, Upload, MessageSquare, History, HeartHandshake } from '@lucide/svelte';

	const actions = [
		{
			title: 'New Submission',
			description: 'Upload walking videos for gait analysis',
			href: '/submit',
			icon: Upload,
			primary: true
		},
		{
			title: 'Assistant',
			description: 'Chat with our AI about your results',
			href: '/assistant',
			icon: MessageSquare,
			primary: false
		},
		{
			title: 'View Submissions',
			description: 'See your past submissions and results',
			href: '/submissions',
			icon: History,
			primary: false
		},
		{
			title: 'Contribute',
			description: 'Donate walking videos for research',
			href: '/contribute',
			icon: HeartHandshake,
			primary: false
		}
	];
</script>

<section>
	<h2 class="section-heading">Quick Actions</h2>
	<div class="actions-grid">
		{#each actions as action (action.href)}
			<a href={action.href} class="action-item" class:primary={action.primary}>
				<div class="action-icon-wrap" class:primary-icon={action.primary}>
					<svelte:component this={action.icon} class="action-icon" />
				</div>
				<div class="action-text">
					<span class="action-title">{action.title}</span>
					<span class="action-desc">{action.description}</span>
				</div>
				<ArrowRight class="action-arrow" />
			</a>
		{/each}
	</div>
</section>

<style>
	.section-heading {
		font-size: 1.125rem;
		font-weight: 600;
		margin-bottom: 0.75rem;
	}

	.actions-grid {
		display: grid;
		gap: 0.5rem;
		grid-template-columns: 1fr;
	}

	@media (min-width: 640px) {
		.actions-grid {
			grid-template-columns: 1fr 1fr;
		}
	}

	.action-item {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		padding: 0.875rem 1rem;
		border: 1px solid var(--border);
		border-radius: var(--radius-md);
		background: var(--card);
		text-decoration: none;
		color: inherit;
		transition: border-color 0.15s ease;
	}

	.action-item:hover {
		border-color: oklch(from var(--primary) l c h / 0.4);
	}

	.action-item.primary {
		border-color: oklch(from var(--primary) l c h / 0.3);
		background: oklch(from var(--primary) l c h / 0.04);
	}

	.action-item.primary:hover {
		border-color: oklch(from var(--primary) l c h / 0.5);
	}

	.action-icon-wrap {
		flex-shrink: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		width: 2.25rem;
		height: 2.25rem;
		border-radius: var(--radius-sm);
		background: oklch(from var(--muted) l c h / 0.5);
	}

	.action-icon-wrap.primary-icon {
		background: oklch(from var(--primary) l c h / 0.1);
	}

	:global(.action-icon) {
		width: 1rem;
		height: 1rem;
		color: var(--muted-foreground);
	}

	.primary-icon :global(.action-icon) {
		color: var(--primary);
	}

	.action-text {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 0.125rem;
	}

	.action-title {
		font-size: 0.875rem;
		font-weight: 600;
	}

	.action-desc {
		font-size: 0.75rem;
		color: var(--muted-foreground);
		line-height: 1.4;
	}

	:global(.action-arrow) {
		width: 0.875rem;
		height: 0.875rem;
		color: var(--muted-foreground);
		flex-shrink: 0;
	}
</style>
