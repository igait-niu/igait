<script lang="ts" module>
	// Module exports moved to $lib/types for proper import handling
</script>

<script lang="ts">
	import { setContext } from 'svelte';
	import { goto } from '$app/navigation';
	import { authStore, errorStore, registryStore } from '$lib/stores';
	import { isAuthenticated, isLoading, USER_CONTEXT_KEY } from '$lib/types';
	import { ErrorBanner, ErrorPage, Footer, LoadingPage } from '$lib/components';
	import { Button } from '$lib/components/ui/button';
	import * as Avatar from '$lib/components/ui/avatar';
	import * as DropdownMenu from '$lib/components/ui/dropdown-menu';
	import {
		Home,
		Upload,
		MessageSquare,
		History,
		LogOut,
		Menu,
		Settings,
		X,
		Shield,
		HeartHandshake
	} from '@lucide/svelte';
	import type { Component } from 'svelte';

	let { children } = $props();

	let mobileMenuOpen = $state(false);

	const authState = $derived(authStore.state);
	const hasError = $derived(errorStore.hasError);

	// Helper function that sets context and returns user
	// Called via {@const} in template to ensure context is set when user exists
	function setUserContext(user: import('$lib/types').User) {
		setContext(USER_CONTEXT_KEY, user);
		return user;
	}

	// Redirect unauthenticated users to login
	$effect(() => {
		if (!isLoading(authState) && !isAuthenticated(authState)) {
			goto('/login');
		}
	});

	// Subscribe to the stage registry once the user is authenticated
	// (reads require auth != null per the RTDB rules).
	$effect(() => {
		if (isAuthenticated(authState)) {
			registryStore.initialize();
		}
	});

	async function handleSignOut() {
		const result = await authStore.signOut();
		if (result.isErr()) {
			errorStore.setError(result.error);
		} else {
			goto('/');
		}
	}

	function closeMobileMenu() {
		mobileMenuOpen = false;
	}

	interface NavItem {
		href: string;
		label: string;
		icon: Component<{ class?: string }>;
		adminOnly?: boolean;
	}

	const navItems: NavItem[] = [
		{ href: '/dashboard', label: 'Dashboard', icon: Home },
		{ href: '/submit', label: 'Submit', icon: Upload },
		{ href: '/assistant', label: 'Assistant', icon: MessageSquare },
		{ href: '/submissions', label: 'Submissions', icon: History },
		{ href: '/contribute', label: 'Contribute', icon: HeartHandshake },
		{ href: '/admin', label: 'Admin', icon: Shield, adminOnly: true }
	];
</script>

<!--
	Authenticated layout with:
	1. Error banner (shows when errorStore has error)
	2. Auth guard (redirects if not authenticated)
	3. User context (provides User to all child pages)
	4. Error boundary (shows ErrorPage instead of children when error exists)
-->

{#if isLoading(authState)}
	<LoadingPage message="Checking authentication..." />
{:else if isAuthenticated(authState)}
	{@const user = setUserContext(authState.user)}

	<!-- Error Banner - always visible at top when there's an error -->
	<ErrorBanner />

	<div class="flex min-h-screen flex-col" class:pt-14={hasError}>
		<!-- Navigation Header -->
		<header
			class="sticky top-0 z-40 border-b bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60"
		>
			<div class="mx-auto flex h-16 max-w-7xl items-center justify-between px-4">
				<!-- Logo -->
				<a href="/dashboard" class="flex items-center gap-2 text-xl font-bold">
					<span class="text-primary">iGait</span>
				</a>

				<!-- Desktop Navigation -->
				<nav class="hidden items-center gap-6 md:flex">
					{#each navItems.filter((item) => !item.adminOnly || user.administrator) as item (item.href)}
						<a
							href={item.href}
							class="flex items-center gap-2 text-sm text-muted-foreground transition-colors hover:text-foreground"
							class:admin-link={item.adminOnly}
						>
							<item.icon class="h-4 w-4" />
							{item.label}
						</a>
					{/each}
				</nav>

				<!-- User Menu -->
				<div class="flex items-center gap-4">
					<DropdownMenu.Root>
						<DropdownMenu.Trigger>
							<Button variant="ghost" class="relative h-10 w-10 rounded-full">
								<Avatar.Root class="h-10 w-10">
									{#if user.photoURL}
										<Avatar.Image src={user.photoURL} alt={user.displayName} />
									{/if}
									<Avatar.Fallback>
										{user.displayName.charAt(0).toUpperCase()}
									</Avatar.Fallback>
								</Avatar.Root>
							</Button>
						</DropdownMenu.Trigger>
						<DropdownMenu.Content class="w-56" align="end">
							<DropdownMenu.Label>
								<div class="flex flex-col space-y-1">
									<p class="text-sm font-medium">{user.displayName}</p>
									<p class="text-xs text-muted-foreground">{user.email}</p>
								</div>
							</DropdownMenu.Label>
							<DropdownMenu.Separator />
							<a href="/settings">
								<DropdownMenu.Item>
									<Settings class="mr-2 h-4 w-4" />
									Settings
								</DropdownMenu.Item>
							</a>
							<DropdownMenu.Separator />
							<DropdownMenu.Item onclick={handleSignOut}>
								<LogOut class="mr-2 h-4 w-4" />
								Sign Out
							</DropdownMenu.Item>
						</DropdownMenu.Content>
					</DropdownMenu.Root>

					<!-- Mobile menu button -->
					<Button
						variant="ghost"
						size="icon"
						class="md:hidden"
						onclick={() => (mobileMenuOpen = !mobileMenuOpen)}
					>
						{#if mobileMenuOpen}
							<X class="h-5 w-5" />
						{:else}
							<Menu class="h-5 w-5" />
						{/if}
					</Button>
				</div>
			</div>

			<!-- Mobile Navigation -->
			{#if mobileMenuOpen}
				<nav class="border-t bg-background px-4 py-4 md:hidden">
					<div class="flex flex-col gap-2">
						{#each navItems.filter((item) => !item.adminOnly || user.administrator) as item (item.href)}
							<a
								href={item.href}
								class="flex items-center gap-3 rounded-md px-3 py-2 text-sm text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
								class:admin-link-mobile={item.adminOnly}
								onclick={closeMobileMenu}
							>
								<item.icon class="h-4 w-4" />
								{item.label}
							</a>
						{/each}
					</div>
				</nav>
			{/if}
		</header>

		<!-- Main Content -->
		<main class="flex-1">
			<div class="mx-auto max-w-7xl px-4 py-8">
				{#if hasError}
					<!-- Error boundary - show error page instead of content -->
					<ErrorPage
						showHomeButton={true}
						showRetryButton={true}
						onRetry={() => errorStore.clearError()}
					/>
				{:else}
					<!-- Normal content - user is available in context -->
					{@render children()}
				{/if}
			</div>
		</main>

		<!-- Footer -->
		<Footer />
	</div>
{:else}
	<!-- Redirect happening, show loading -->
	<LoadingPage message="Redirecting to login..." />
{/if}

<style>
	.admin-link {
		color: hsl(38 92% 50%) !important;
	}

	.admin-link:hover {
		color: hsl(38 92% 40%) !important;
	}

	.admin-link-mobile {
		color: hsl(38 92% 50%) !important;
		background-color: hsl(38 92% 50% / 0.1);
	}

	.admin-link-mobile:hover {
		background-color: hsl(38 92% 50% / 0.15) !important;
	}
</style>
