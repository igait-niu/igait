<script lang="ts">
	import { AlertTriangle, Home, RefreshCw } from '@lucide/svelte';
	import { Button } from '$lib/components/ui/button';
	import * as Card from '$lib/components/ui/card';
	import { errorStore } from '$lib/stores';
	import { goto } from '$app/navigation';

	interface Props {
		showHomeButton?: boolean;
		showRetryButton?: boolean;
		onRetry?: () => void;
	}

	let { showHomeButton = true, showRetryButton = false, onRetry }: Props = $props();

	function handleGoHome() {
		errorStore.clearError();
		goto('/');
	}

	function handleRetry() {
		errorStore.clearError();
		onRetry?.();
	}
</script>

<div class="flex min-h-[60vh] items-center justify-center p-4">
	<Card.Root class="w-full max-w-md text-center">
		<Card.Header class="pb-4">
			<div
				class="mx-auto mb-4 flex h-16 w-16 items-center justify-center rounded-full bg-destructive/10"
			>
				<AlertTriangle class="h-8 w-8 text-destructive" />
			</div>
			<Card.Title class="text-2xl">Oh no!</Card.Title>
			<Card.Description class="text-base">
				Something went wrong and we couldn't load this page safely.
			</Card.Description>
		</Card.Header>
		<Card.Content>
			<p class="text-sm text-muted-foreground">
				Don't worry though - the error has been captured in the banner above. You can view the
				details there or try one of the options below.
			</p>
		</Card.Content>
		<Card.Footer class="flex justify-center gap-3 pt-2">
			{#if showRetryButton && onRetry}
				<Button variant="outline" onclick={handleRetry}>
					<RefreshCw class="mr-2 h-4 w-4" />
					Try Again
				</Button>
			{/if}
			{#if showHomeButton}
				<Button onclick={handleGoHome}>
					<Home class="mr-2 h-4 w-4" />
					Go Home
				</Button>
			{/if}
		</Card.Footer>
	</Card.Root>
</div>
