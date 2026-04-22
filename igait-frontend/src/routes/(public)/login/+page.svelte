<script lang="ts">
	import { goto } from '$app/navigation';
	import { authStore } from '$lib/stores';
	import { validateEmail, validatePassword } from '$lib/api';
	import { Button } from '$lib/components/ui/button';
	import * as Card from '$lib/components/ui/card';
	import { Input } from '$lib/components/ui/input';
	import { Label } from '$lib/components/ui/label';
	import { Separator } from '$lib/components/ui/separator';
	import { Alert, AlertDescription } from '$lib/components/ui/alert';
	import { Loader2, AlertCircle } from '@lucide/svelte';
	import { type Option, None, Some, type AppError } from '$lib/result';

	let email = $state('');
	let password = $state('');
	let isLoading = $state(false);
	let error: Option<AppError> = $state(None());

	// Emulator-only credential hint. `VITE_FIREBASE_USE_EMULATOR` is inlined
	// at build time, so this whole branch (including the literal creds) is
	// treeshaken out of the prod bundle. Values must stay in sync with the
	// `firebase-bootstrap` service in docker-compose.yml — if you rotate one,
	// rotate the other.
	const IS_EMULATOR = import.meta.env.VITE_FIREBASE_USE_EMULATOR === 'true';
	const DEV_ADMIN_EMAIL = 'admin@igait.local';
	const DEV_ADMIN_PASSWORD = 'admin123';

	function fillDevAdmin() {
		email = DEV_ADMIN_EMAIL;
		password = DEV_ADMIN_PASSWORD;
	}

	async function handleEmailLogin(e: Event) {
		e.preventDefault();
		error = None();

		// Validate inputs
		const emailResult = validateEmail(email);
		if (emailResult.isErr()) {
			error = Some(emailResult.error);
			return;
		}

		const passwordResult = validatePassword(password);
		if (passwordResult.isErr()) {
			error = Some(passwordResult.error);
			return;
		}

		isLoading = true;

		const result = await authStore.signInWithEmail(email, password);

		if (result.isErr()) {
			error = Some(result.error);
			isLoading = false;
		} else {
			// Success! The auth state change will trigger redirect
			goto('/dashboard');
		}
	}

	async function handleGoogleLogin() {
		error = None();
		isLoading = true;

		const result = await authStore.signInWithGoogle();

		if (result.isErr()) {
			error = Some(result.error);
			isLoading = false;
		} else {
			goto('/dashboard');
		}
	}
</script>

<svelte:head>
	<title>Log In - iGait</title>
</svelte:head>

<div class="auth-container">
	<Card.Root class="auth-card">
		<Card.Header class="auth-header">
			<Card.Title class="auth-title">Welcome Back</Card.Title>
			<Card.Description>Sign in to your account to continue</Card.Description>
		</Card.Header>
		<Card.Content>
			{#if IS_EMULATOR}
				<div class="dev-banner" role="note" aria-label="Development mode credentials">
					<div class="dev-banner-title">
						<span class="dev-banner-badge">DEV</span>
						<span>Seeded admin account</span>
					</div>
					<dl class="dev-banner-creds">
						<dt>Email</dt>
						<dd><code>{DEV_ADMIN_EMAIL}</code></dd>
						<dt>Password</dt>
						<dd><code>{DEV_ADMIN_PASSWORD}</code></dd>
					</dl>
					<button type="button" class="dev-banner-fill" onclick={fillDevAdmin}> Fill in </button>
				</div>
			{/if}

			<!-- Error Alert -->
			{#if error.isSome()}
				<Alert variant="destructive" class="error-alert">
					<AlertCircle class="h-4 w-4" />
					<AlertDescription>
						{error.value.displayMessage}
					</AlertDescription>
				</Alert>
			{/if}

			<!-- Google Sign In -->
			<Button
				variant="outline"
				class="social-button"
				onclick={handleGoogleLogin}
				disabled={isLoading}
			>
				{#if isLoading}
					<Loader2 class="mr-2 h-4 w-4 animate-spin" />
				{:else}
					<svg class="mr-2 h-4 w-4" viewBox="0 0 24 24">
						<path
							fill="currentColor"
							d="M22.56 12.25c0-.78-.07-1.53-.2-2.25H12v4.26h5.92c-.26 1.37-1.04 2.53-2.21 3.31v2.77h3.57c2.08-1.92 3.28-4.74 3.28-8.09z"
						/>
						<path
							fill="currentColor"
							d="M12 23c2.97 0 5.46-.98 7.28-2.66l-3.57-2.77c-.98.66-2.23 1.06-3.71 1.06-2.86 0-5.29-1.93-6.16-4.53H2.18v2.84C3.99 20.53 7.7 23 12 23z"
						/>
						<path
							fill="currentColor"
							d="M5.84 14.09c-.22-.66-.35-1.36-.35-2.09s.13-1.43.35-2.09V7.07H2.18C1.43 8.55 1 10.22 1 12s.43 3.45 1.18 4.93l2.85-2.22.81-.62z"
						/>
						<path
							fill="currentColor"
							d="M12 5.38c1.62 0 3.06.56 4.21 1.64l3.15-3.15C17.45 2.09 14.97 1 12 1 7.7 1 3.99 3.47 2.18 7.07l3.66 2.84c.87-2.6 3.3-4.53 6.16-4.53z"
						/>
					</svg>
				{/if}
				Continue with Google
			</Button>

			<div class="auth-divider">
				<div class="divider-line">
					<Separator class="w-full" />
				</div>
				<div class="divider-text">
					<span class="bg-card px-2 text-muted-foreground">Or continue with email</span>
				</div>
			</div>

			<!-- Email/Password Form -->
			<form onsubmit={handleEmailLogin} class="auth-form">
				<div class="form-field">
					<Label for="email">Email</Label>
					<Input
						id="email"
						type="email"
						placeholder="you@example.com"
						bind:value={email}
						disabled={isLoading}
						required
					/>
				</div>

				<div class="form-field">
					<Label for="password">Password</Label>
					<Input
						id="password"
						type="password"
						placeholder="••••••••"
						bind:value={password}
						disabled={isLoading}
						required
					/>
				</div>

				<Button type="submit" class="submit-button" disabled={isLoading}>
					{#if isLoading}
						<Loader2 class="mr-2 h-4 w-4 animate-spin" />
					{/if}
					Sign In
				</Button>
			</form>
		</Card.Content>
		<Card.Footer class="auth-footer">
			<p class="footer-text">
				Don't have an account?
				<a href="/signup" class="footer-link"> Sign up </a>
			</p>
		</Card.Footer>
	</Card.Root>
</div>

<style>
	.auth-container {
		display: flex;
		min-height: calc(100vh - 12rem);
		align-items: center;
		justify-content: center;
		padding-top: var(--spacing-lg);
		padding-bottom: var(--spacing-lg);
	}

	:global(.auth-card) {
		width: 100%;
		max-width: 28rem;
	}

	:global(.auth-header) {
		text-align: center;
	}

	:global(.auth-title) {
		font-size: 1.5rem;
	}

	:global(.error-alert) {
		margin-bottom: 1.5rem;
	}

	:global(.social-button) {
		width: 100%;
	}

	.auth-divider {
		position: relative;
		margin-top: 1.5rem;
		margin-bottom: 1.5rem;
	}

	.divider-line {
		position: absolute;
		inset: 0;
		display: flex;
		align-items: center;
	}

	.divider-text {
		position: relative;
		display: flex;
		justify-content: center;
		font-size: 0.75rem;
		text-transform: uppercase;
	}

	.auth-form {
		display: flex;
		flex-direction: column;
		gap: var(--stack-sm);
	}

	.form-field {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}

	:global(.submit-button) {
		width: 100%;
	}

	:global(.submit-button:focus-visible),
	:global(.social-button:focus-visible) {
		outline: 2px solid hsl(var(--ring));
		outline-offset: 2px;
	}

	:global(.auth-footer) {
		display: flex;
		flex-direction: column;
		gap: var(--stack-sm);
	}

	.footer-text {
		text-align: center;
		font-size: 0.875rem;
		color: hsl(var(--muted-foreground));
	}

	.footer-link {
		color: hsl(var(--primary));
		text-underline-offset: 4px;
	}

	.footer-link:hover {
		text-decoration: underline;
	}

	/* Loud dev-only banner. Treeshaken out of prod via VITE_FIREBASE_USE_EMULATOR. */
	.dev-banner {
		display: grid;
		grid-template-columns: 1fr auto;
		align-items: center;
		gap: 0.75rem 1rem;
		margin-bottom: 1.5rem;
		padding: 0.875rem 1rem;
		border: 2px dashed #f59e0b;
		background: repeating-linear-gradient(45deg, #fef3c7, #fef3c7 10px, #fde68a 10px, #fde68a 20px);
		color: #78350f;
		border-radius: 0.5rem;
		font-size: 0.875rem;
	}

	.dev-banner-title {
		grid-column: 1 / -1;
		display: flex;
		align-items: center;
		gap: 0.5rem;
		font-weight: 600;
	}

	.dev-banner-badge {
		background: #f59e0b;
		color: #fff;
		padding: 0.125rem 0.5rem;
		border-radius: 0.25rem;
		font-size: 0.7rem;
		letter-spacing: 0.05em;
	}

	.dev-banner-creds {
		display: grid;
		grid-template-columns: auto 1fr;
		column-gap: 0.5rem;
		row-gap: 0.125rem;
		margin: 0;
	}

	.dev-banner-creds dt {
		font-weight: 500;
		opacity: 0.85;
	}

	.dev-banner-creds dd {
		margin: 0;
	}

	.dev-banner-creds code {
		background: rgba(255, 255, 255, 0.6);
		padding: 0.0625rem 0.375rem;
		border-radius: 0.25rem;
		font-size: 0.8125rem;
	}

	.dev-banner-fill {
		background: #f59e0b;
		color: #fff;
		border: none;
		padding: 0.5rem 0.875rem;
		border-radius: 0.375rem;
		font-weight: 600;
		cursor: pointer;
		font-size: 0.8125rem;
	}

	.dev-banner-fill:hover {
		background: #d97706;
	}

	.dev-banner-fill:focus-visible {
		outline: 2px solid #78350f;
		outline-offset: 2px;
	}
</style>
