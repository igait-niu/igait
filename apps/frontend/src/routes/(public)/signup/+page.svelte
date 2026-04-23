<script lang="ts">
	import { goto } from '$app/navigation';
	import { authStore } from '$lib/stores';
	import { validateEmail, validatePassword, validatePasswordMatch } from '$lib/api';
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
	let confirmPassword = $state('');
	let isLoading = $state(false);
	let error: Option<AppError> = $state(None());

	async function handleEmailSignup(e: Event) {
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

		const matchResult = validatePasswordMatch(password, confirmPassword);
		if (matchResult.isErr()) {
			error = Some(matchResult.error);
			return;
		}

		isLoading = true;

		const result = await authStore.signUpWithEmail(email, password);

		if (result.isErr()) {
			error = Some(result.error);
			isLoading = false;
		} else {
			// Success! Redirect to dashboard
			goto('/dashboard');
		}
	}

	async function handleGoogleSignup() {
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
	<title>Sign Up - iGait</title>
</svelte:head>

<div class="auth-container">
	<Card.Root class="auth-card">
		<Card.Header class="auth-header">
			<Card.Title class="auth-title">Create an Account</Card.Title>
			<Card.Description>Get started with iGait gait analysis</Card.Description>
		</Card.Header>
		<Card.Content>
			<!-- Error Alert -->
			{#if error.isSome()}
				<Alert variant="destructive" class="error-alert">
					<AlertCircle class="h-4 w-4" />
					<AlertDescription>
						{error.value.displayMessage}
					</AlertDescription>
				</Alert>
			{/if}

			<!-- Google Sign Up -->
			<Button
				variant="outline"
				class="social-button"
				onclick={handleGoogleSignup}
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
			<form onsubmit={handleEmailSignup} class="auth-form">
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
					<p class="text-xs text-muted-foreground">Must be at least 6 characters</p>
				</div>

				<div class="form-field">
					<Label for="confirmPassword">Confirm Password</Label>
					<Input
						id="confirmPassword"
						type="password"
						placeholder="••••••••"
						bind:value={confirmPassword}
						disabled={isLoading}
						required
					/>
				</div>

				<Button type="submit" class="submit-button" disabled={isLoading}>
					{#if isLoading}
						<Loader2 class="mr-2 h-4 w-4 animate-spin" />
					{/if}
					Create Account
				</Button>
			</form>

			<p class="auth-terms">
				By signing up, you agree to our
				<a href="/terms" class="underline underline-offset-4 hover:text-primary">Terms of Service</a
				>
				and
				<a href="/policy" class="underline underline-offset-4 hover:text-primary">Privacy Policy</a>
			</p>
		</Card.Content>
		<Card.Footer class="auth-footer">
			<p class="footer-text">
				Already have an account?
				<a href="/login" class="footer-link"> Sign in </a>
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

	.auth-terms {
		margin-top: 1rem;
		text-align: center;
		font-size: 0.75rem;
		color: hsl(var(--muted-foreground));
	}

	.auth-terms a {
		color: hsl(var(--foreground));
		text-decoration: underline;
		text-underline-offset: 4px;
		transition: color 0.2s;
	}

	.auth-terms a:hover {
		color: hsl(var(--primary));
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
</style>
