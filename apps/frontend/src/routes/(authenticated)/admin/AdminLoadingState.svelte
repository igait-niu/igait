<script lang="ts">
	import { Loader2 } from '@lucide/svelte';

	interface Props {
		message?: string;
	}

	let { message = 'Loading...' }: Props = $props();
</script>

<div class="loading-state">
	<Loader2 class="spinner" />
	<p>{message}</p>
</div>

<style>
	.loading-state {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 0.75rem;
		padding: 2.5rem 1.5rem;
		color: hsl(var(--muted-foreground));
	}

	.loading-state p {
		font-size: 0.8125rem;
		margin: 0;
	}

	:global(.spinner) {
		width: 1.5rem;
		height: 1.5rem;
		animation: spin 1s linear infinite;
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
