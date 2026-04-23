<script lang="ts">
	import { AlertCircle, ChevronDown, ChevronUp, X } from '@lucide/svelte';
	import { Button } from '$lib/components/ui/button';
	import { errorStore } from '$lib/stores';
	import { cn } from '$lib/utils';

	let showDetails = $state(false);

	const error = $derived(errorStore.current);
	const hasError = $derived(errorStore.hasError);

	function handleDismiss() {
		errorStore.clearError();
		showDetails = false;
	}

	function toggleDetails() {
		showDetails = !showDetails;
	}
</script>

{#if hasError && error.isSome()}
	{@const appError = error.value}
	<div
		class={cn(
			'fixed top-0 right-0 left-0 z-50',
			'text-destructive-foreground bg-destructive',
			'shadow-lg transition-all duration-300 ease-in-out',
			'animate-in slide-in-from-top-full'
		)}
		role="alert"
		aria-live="assertive"
	>
		<div class="mx-auto max-w-7xl px-4 py-3">
			<div class="flex items-center justify-between gap-4">
				<!-- Error Icon and Message -->
				<div class="flex min-w-0 flex-1 items-center gap-3">
					<AlertCircle class="h-5 w-5 flex-shrink-0" />
					<p class="truncate font-medium">
						{appError.displayMessage}
					</p>
				</div>

				<!-- Actions -->
				<div class="flex flex-shrink-0 items-center gap-2">
					{#if appError.hasContext}
						<Button
							variant="ghost"
							size="sm"
							class="text-destructive-foreground hover:bg-destructive-foreground/10"
							onclick={toggleDetails}
						>
							{showDetails ? 'Hide' : 'Show'} Details
							{#if showDetails}
								<ChevronUp class="ml-1 h-4 w-4" />
							{:else}
								<ChevronDown class="ml-1 h-4 w-4" />
							{/if}
						</Button>
					{/if}
					<Button
						variant="ghost"
						size="icon"
						class="text-destructive-foreground hover:bg-destructive-foreground/10"
						onclick={handleDismiss}
						aria-label="Dismiss error"
					>
						<X class="h-4 w-4" />
					</Button>
				</div>
			</div>

			<!-- Error Chain Details -->
			{#if showDetails && appError.hasContext}
				<div
					class={cn(
						'border-destructive-foreground/20 mt-3 border-t pt-3',
						'animate-in slide-in-from-top-2'
					)}
				>
					<p class="mb-2 text-sm font-medium">Error Chain:</p>
					<div class="flex flex-wrap items-center gap-2 text-sm">
						{#each appError.contextChain as context, i (i)}
							<span class="bg-destructive-foreground/10 rounded px-2 py-1">
								{context}
							</span>
							{#if i < appError.contextChain.length - 1}
								<span class="text-destructive-foreground/60">→</span>
							{/if}
						{/each}
						<span class="text-destructive-foreground/60">→</span>
						<span class="bg-destructive-foreground/20 rounded px-2 py-1 font-mono text-xs">
							{appError.rootCause}
						</span>
					</div>
				</div>
			{/if}
		</div>
	</div>
{/if}
