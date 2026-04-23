<script lang="ts">
	import { authStore } from '$lib/stores';
	import { isAuthenticated, isLoading } from '$lib/types';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { Footer, LoadingPage } from '$lib/components';

	let { children } = $props();

	const authState = $derived(authStore.state);

	// Redirect authenticated users away from login/signup
	$effect(() => {
		if (isAuthenticated(authState)) {
			const currentPath = page.url.pathname;
			// If on login or signup, redirect to dashboard
			if (currentPath.startsWith('/login') || currentPath.startsWith('/signup')) {
				goto('/dashboard');
			}
		}
	});
</script>

<!--
	Public layout - no auth required
	Shows a minimal header/footer for public pages
-->

{#if isLoading(authState)}
	<LoadingPage message="Loading..." />
{:else}
	<div class="flex min-h-screen flex-col">
		<!-- Simple header for public pages -->
		<header
			class="border-b bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60"
		>
			<div class="mx-auto flex h-16 max-w-7xl items-center justify-between px-4">
				<a href="/" class="flex items-center gap-2 text-xl font-bold">
					<span class="text-primary">iGait</span>
				</a>

				<nav class="flex items-center gap-4">
					<a
						href="/about"
						class="text-sm text-muted-foreground transition-colors hover:text-foreground"
					>
						About
					</a>
					{#if !isAuthenticated(authState)}
						<a
							href="/login"
							class="text-sm text-muted-foreground transition-colors hover:text-foreground"
						>
							Log In
						</a>
						<a
							href="/signup"
							class="rounded-md bg-primary px-4 py-2 text-sm font-medium text-primary-foreground transition-colors hover:bg-primary/90"
						>
							Sign Up
						</a>
					{:else}
						<a
							href="/dashboard"
							class="rounded-md bg-primary px-4 py-2 text-sm font-medium text-primary-foreground transition-colors hover:bg-primary/90"
						>
							Dashboard
						</a>
					{/if}
				</nav>
			</div>
		</header>

		<!-- Main content -->
		<main class="flex-1">
			{@render children()}
		</main>

		<!-- Footer -->
		<Footer />
	</div>
{/if}
