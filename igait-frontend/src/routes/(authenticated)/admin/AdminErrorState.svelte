<script lang="ts">
	import { AlertCircle } from '@lucide/svelte';

	interface Props {
		message: string;
	}

	let { message }: Props = $props();
</script>

<div class="error-state">
	<AlertCircle class="error-icon" />
	<p>{message}</p>
</div>

<style>
	.error-state {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 0.75rem;
		padding: 2.5rem 1.5rem;
		color: hsl(var(--muted-foreground));
	}

	.error-state p {
		font-size: 0.8125rem;
		margin: 0;
		text-align: center;
	}

	:global(.error-icon) {
		width: 1.5rem;
		height: 1.5rem;
		color: hsl(var(--destructive));
	}
</style>
