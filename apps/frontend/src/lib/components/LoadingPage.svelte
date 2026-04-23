<script lang="ts">
	import { Loader2 } from '@lucide/svelte';

	interface Props {
		message?: string;
	}

	let { message = 'Loading...' }: Props = $props();
</script>

<div class="flex min-h-[60vh] flex-col items-center justify-center gap-4">
	<Loader2 class="h-8 w-8 animate-spin text-primary" />
	<p class="text-sm text-muted-foreground">{message}</p>
</div>
