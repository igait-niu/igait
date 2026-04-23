<script lang="ts">
	import { getUser } from '$lib/hooks';
	import * as Card from '$lib/components/ui/card';
	import { Button } from '$lib/components/ui/button';
	const user = getUser();
</script>

<svelte:head>
	<title>Settings - iGait</title>
</svelte:head>

<div class="settings-page stack-lg">
	<section>
		<h1 class="page-title">Settings</h1>
		<p class="page-description">Manage your account preferences</p>
	</section>

	<!-- Coming Soon Placeholder -->
	<Card.Root class="coming-soon-card">
		<Card.Content>
			<div class="account-info">
				<h3 class="account-title">Your Current Account</h3>
				<p class="account-detail">{user.email}</p>

				<h3 class="account-title" style="margin-top: 1rem;">Display Name</h3>
				<p class="account-detail">{user.displayName}</p>
			</div>

			<Button variant="outline" href="/dashboard" class="back-button">Back to Dashboard</Button>
		</Card.Content>
	</Card.Root>
</div>

<style>
	.page-title {
		font-size: 1.5rem;
		font-weight: 700;
		line-height: 1.2;
		letter-spacing: -0.025em;
	}

	@media (min-width: 640px) {
		.page-title {
			font-size: 1.875rem;
		}
	}

	.page-description {
		margin-top: 0.5rem;
		color: hsl(var(--muted-foreground));
	}

	:global(.coming-soon-card) {
		text-align: center;
	}

	@keyframes pulse {
		0%,
		100% {
			opacity: 1;
		}
		50% {
			opacity: 0.7;
		}
	}

	.account-info {
		margin-top: 1.5rem;
		border-radius: var(--radius-lg);
		background-color: hsl(var(--muted));
		padding: 1rem;
	}

	.account-title {
		font-weight: 600;
	}

	.account-detail {
		font-size: 0.875rem;
		color: hsl(var(--muted-foreground));
	}

	:global(.back-button) {
		margin-top: 1.5rem;
	}
</style>
