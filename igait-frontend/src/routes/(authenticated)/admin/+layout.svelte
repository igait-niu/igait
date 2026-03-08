<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/stores';
	import { getUser } from '$lib/hooks';
	import { Button } from '$lib/components/ui/button';
	import { Badge } from '$lib/components/ui/badge';
	import { Shield, Layers, Users, AlertTriangle } from '@lucide/svelte';

	let { children } = $props();

	const user = getUser();

	// Redirect non-admins
	$effect(() => {
		if (!user.administrator) {
			goto('/dashboard');
		}
	});

	// Navigation items for admin panel
	const adminNavItems = [
		{ href: '/admin/queues', label: 'Queue Overview', desc: 'Pipeline Queues', icon: Layers },
		{ href: '/admin/jobs', label: 'Job Overview', desc: 'All Submissions', icon: Users }
	];

	const currentPath = $derived($page.url.pathname);
</script>

{#if user.administrator}
	<div class="admin-panel">
		<!-- Header -->
		<header class="admin-header">
			<div class="admin-title-row">
				<Shield class="admin-shield-icon" />
				<h1 class="admin-title">Admin Panel</h1>
				<Badge variant="outline" class="admin-badge">Administrator</Badge>
			</div>
			<p class="admin-description">Monitor system queues and manage all user submissions</p>
		</header>

		<!-- Tab Navigation -->
		<div class="admin-nav-container">
			<nav class="admin-nav">
				{#each adminNavItems as item}
					<a href={item.href} class="admin-nav-tab" class:active={currentPath === item.href}>
						<item.icon class="admin-nav-icon" />
						<span class="admin-nav-label">{item.label}</span>
						<span class="admin-nav-desc">{item.desc}</span>
					</a>
				{/each}
			</nav>
		</div>

		<!-- Content -->
		<div class="admin-content">
			{@render children()}
		</div>
	</div>
{:else}
	<div class="access-denied">
		<AlertTriangle class="h-12 w-12 text-destructive" />
		<h2>Access Denied</h2>
		<p>You don't have permission to access this area.</p>
		<Button href="/dashboard">Return to Dashboard</Button>
	</div>
{/if}

<style>
	.admin-panel {
		display: flex;
		flex-direction: column;
		gap: 1.25rem;
	}

	/* ── Header ─────────────────────────────────────────── */

	.admin-header {
		display: flex;
		flex-direction: column;
		gap: 0.375rem;
	}

	.admin-title-row {
		display: flex;
		align-items: center;
		gap: 0.625rem;
	}

	:global(.admin-shield-icon) {
		width: 1.375rem;
		height: 1.375rem;
		color: oklch(0.795 0.184 86.047);
		flex-shrink: 0;
	}

	.admin-title {
		font-size: 1.125rem;
		font-weight: 600;
		margin: 0;
	}

	:global(.admin-badge) {
		background-color: oklch(0.795 0.184 86.047 / 0.1) !important;
		border-color: oklch(0.795 0.184 86.047 / 0.3) !important;
		color: oklch(0.795 0.184 86.047) !important;
		font-size: 0.6875rem !important;
	}

	.admin-description {
		font-size: 0.8125rem;
		color: var(--muted-foreground);
		margin: 0;
	}

	/* ── Tab Navigation ─────────────────────────────────── */

	.admin-nav-container {
		padding: 0.5rem;
	}

	.admin-nav {
		display: flex;
		gap: 0.25rem;
		justify-content: center;
	}

	.admin-nav-tab {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 0.125rem;
		padding: 0.625rem 1.5rem;
		border: 1px solid transparent;
		background: none;
		cursor: pointer;
		color: var(--muted-foreground);
		transition: all 0.15s ease;
		white-space: nowrap;
		border-radius: var(--radius-sm);
		text-decoration: none;
	}

	.admin-nav-tab:hover {
		color: var(--foreground);
		background-color: oklch(from var(--primary) l c h / 0.06);
	}

	.admin-nav-tab.active {
		color: var(--primary);
		background-color: var(--background);
		border-color: oklch(from var(--primary) l c h / 0.3);
		box-shadow: 0 1px 3px oklch(from var(--primary) l c h / 0.1);
	}

	:global(.admin-nav-icon) {
		width: 1rem;
		height: 1rem;
		margin-bottom: 0.125rem;
	}

	.admin-nav-label {
		font-size: 0.8125rem;
		font-weight: 600;
	}

	.admin-nav-tab.active .admin-nav-label {
		color: var(--primary);
	}

	.admin-nav-desc {
		font-size: 0.6875rem;
		font-weight: 400;
		opacity: 0.7;
	}

	.admin-nav-tab.active .admin-nav-desc {
		opacity: 1;
	}

	/* ── Content ────────────────────────────────────────── */

	.admin-content {
		min-height: 400px;
	}

	/* ── Access Denied ──────────────────────────────────── */

	.access-denied {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 1rem;
		padding: 4rem 2rem;
		text-align: center;
	}

	.access-denied h2 {
		font-size: 1.5rem;
		font-weight: 600;
	}

	.access-denied p {
		color: var(--muted-foreground);
	}
</style>
