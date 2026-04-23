<script lang="ts">
	import './layout.css';
	import favicon from '$lib/assets/favicon.svg';
	import { Toaster } from '$lib/components/ui/sonner';
	import { initializeFirebase } from '$lib/firebase';
	import { authStore } from '$lib/stores';
	import { onMount } from 'svelte';

	let { children } = $props();

	onMount(() => {
		// Initialize Firebase and auth listener
		initializeFirebase();
		authStore.initialize();

		return () => {
			authStore.destroy();
		};
	});
</script>

<svelte:head><link rel="icon" href={favicon} /></svelte:head>

<div class="min-h-screen bg-background text-foreground antialiased">
	{@render children()}
</div>

<Toaster richColors position="top-right" />
