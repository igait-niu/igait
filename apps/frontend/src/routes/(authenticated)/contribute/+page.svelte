<script lang="ts">
	import { getUser } from '$lib/hooks';
	import {
		submitResearchContribution,
		type ResearchContributionRequest,
		type ProgressCallback
	} from '$lib/api';
	import { Button } from '$lib/components/ui/button';
	import * as Card from '$lib/components/ui/card';
	import { Alert, AlertDescription } from '$lib/components/ui/alert';
	import { Loader2, HeartHandshake, FlaskConical } from '@lucide/svelte';
	import { toast } from 'svelte-sonner';
	import { goto } from '$app/navigation';
	import { type Option, None, Some, AppError } from '$lib/result';
	import VideoUploadArea from '../submit/VideoUploadArea.svelte';

	const user = getUser();

	// Video fields
	let frontVideo: File | undefined = $state(undefined);
	let sideVideo: File | undefined = $state(undefined);

	// Form state
	let isSubmitting = $state(false);
	let progress = $state(0);
	let error: Option<AppError> = $state(None());

	function handleFileChange(e: Event, type: 'front' | 'side') {
		const input = e.target as HTMLInputElement;
		const file = input.files?.[0];
		if (file) {
			if (type === 'front') {
				frontVideo = file;
			} else {
				sideVideo = file;
			}
		}
	}

	const isFormValid = $derived(frontVideo !== undefined && sideVideo !== undefined);

	async function handleSubmit(e: Event) {
		e.preventDefault();
		error = None();

		if (!frontVideo || !sideVideo) {
			error = Some(new AppError('Please select both front and side videos'));
			return;
		}

		isSubmitting = true;
		progress = 0;

		const request: ResearchContributionRequest = {
			name: user.displayName,
			email: user.email,
			frontVideo,
			sideVideo
		};

		const onProgress: ProgressCallback = (p) => {
			progress = p;
		};

		const result = await submitResearchContribution(request, onProgress);

		if (result.isErr()) {
			error = Some(result.error);
			isSubmitting = false;
			progress = 0;
		} else {
			toast.success(result.value);
			setTimeout(() => goto('/dashboard'), 1500);
		}
	}
</script>

<svelte:head>
	<title>Contribute - iGait</title>
</svelte:head>

<div class="contribute-page">
	<section class="page-header">
		<h1 class="page-title">Contribute to Research</h1>
		<p class="page-description">
			Help improve gait analysis by contributing your walking videos for research
		</p>
	</section>

	<!-- Info Card -->
	<Card.Root class="info-card">
		<Card.Content class="info-content">
			<div class="info-icon-wrapper">
				<FlaskConical class="info-icon" />
			</div>
			<div class="info-text">
				<h3 class="info-title">How your contribution helps</h3>
				<p class="info-description">
					Your walking videos are used to train and improve our gait analysis models. Contributed
					videos are stored securely and used solely for research purposes. No demographic or
					personal health data is collected — just your videos, name, and email so we can thank you
					for your contribution!
				</p>
			</div>
		</Card.Content>
	</Card.Root>

	<Card.Root class="contribute-card">
		<Card.Header class="compact-header">
			<Card.Title>Upload Videos</Card.Title>
			<Card.Description>
				Please upload both a front view and side view video of the subject walking.
			</Card.Description>
		</Card.Header>
		<Card.Content>
			{#if error.isSome()}
				<Alert variant="destructive" class="form-error">
					<AlertDescription>
						<p class="error-message">{error.value.displayMessage}</p>
						{#if error.value.hasContext}
							<details class="error-details">
								<summary>Show full error</summary>
								<p class="error-full">{error.value.fullMessage}</p>
							</details>
						{/if}
					</AlertDescription>
				</Alert>
			{/if}

			<form onsubmit={handleSubmit} class="contribute-form">
				<!-- Video Uploads Section -->
				<fieldset class="form-section">
					<legend class="form-section__title">Video Uploads</legend>

					<div class="video-grid">
						<VideoUploadArea
							label="Front View"
							id="frontVideo"
							bind:file={frontVideo}
							disabled={isSubmitting}
							onchange={(e) => handleFileChange(e, 'front')}
						/>

						<VideoUploadArea
							label="Side View"
							id="sideVideo"
							bind:file={sideVideo}
							disabled={isSubmitting}
							onchange={(e) => handleFileChange(e, 'side')}
						/>
					</div>
				</fieldset>

				<div class="submit-button-container">
					{#if isSubmitting}
						<div class="button-progress-fill" style="width: {progress}%"></div>
					{/if}
					<Button
						type="submit"
						class="submit-button {isSubmitting ? 'is-uploading' : ''}"
						disabled={isSubmitting || !isFormValid}
					>
						{#if isSubmitting}
							<Loader2 class="mr-2 h-4 w-4 animate-spin" />
							Uploading... {progress}%
						{:else}
							<HeartHandshake class="mr-2 h-4 w-4" />
							Contribute Videos
						{/if}
					</Button>
				</div>
			</form>
		</Card.Content>
	</Card.Root>
</div>

<style>
	.contribute-page {
		max-width: 1000px;
		margin: 0 auto;
		padding: 1.5rem 1rem;
		display: flex;
		flex-direction: column;
		gap: 1.5rem;
	}

	.page-header {
		margin-bottom: 0;
	}

	.page-title {
		font-size: 1.875rem;
		font-weight: 700;
		line-height: 1.2;
		letter-spacing: -0.025em;
	}

	.page-description {
		margin-top: 0.375rem;
		font-size: 0.875rem;
		color: hsl(var(--muted-foreground));
	}

	/* Info card */
	:global(.info-card) {
		border-color: hsl(var(--primary) / 0.2);
		background: hsl(var(--primary) / 0.04);
	}

	:global(.info-content) {
		display: flex;
		gap: 1rem;
		align-items: flex-start;
		padding: 1.25rem !important;
	}

	.info-icon-wrapper {
		display: flex;
		align-items: center;
		justify-content: center;
		height: 2.5rem;
		width: 2.5rem;
		min-width: 2.5rem;
		border-radius: var(--radius-md);
		background: hsl(var(--primary) / 0.1);
	}

	:global(.info-icon) {
		height: 1.25rem;
		width: 1.25rem;
		color: hsl(var(--primary));
	}

	.info-text {
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
	}

	.info-title {
		font-size: 0.9375rem;
		font-weight: 600;
		color: hsl(var(--foreground));
	}

	.info-description {
		font-size: 0.8125rem;
		color: hsl(var(--muted-foreground));
		line-height: 1.5;
	}

	/* Form */
	:global(.compact-header) {
		padding-bottom: 1rem !important;
	}

	.contribute-form {
		display: flex;
		flex-direction: column;
		gap: 1.5rem;
	}

	:global(.form-error) {
		margin-bottom: 1rem;
	}

	.form-section {
		border: none;
		padding: 0;
		margin: 0;
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}

	.form-section__title {
		font-size: 0.9375rem;
		font-weight: 600;
		color: hsl(var(--foreground));
		padding-bottom: 0.5rem;
		border-bottom: 1px solid hsl(var(--border));
		margin: 0 0 0.75rem 0;
	}

	.video-grid {
		display: grid;
		grid-template-columns: repeat(2, 1fr);
		gap: 1rem;
	}

	/* Error styling */
	.error-message {
		font-weight: 500;
		margin: 0;
	}

	.error-details {
		margin-top: 0.5rem;
	}

	.error-details summary {
		cursor: pointer;
		font-size: 0.8125rem;
		color: hsl(var(--destructive-foreground) / 0.8);
	}

	.error-details summary:hover {
		text-decoration: underline;
	}

	.error-full {
		margin-top: 0.375rem;
		font-size: 0.8125rem;
		font-family: ui-monospace, monospace;
		word-break: break-word;
		white-space: pre-wrap;
		background-color: hsl(var(--destructive-foreground) / 0.1);
		padding: 0.5rem;
		border-radius: var(--radius-sm);
	}

	/* Submit button with progress */
	.submit-button-container {
		position: relative;
		margin-top: 0.5rem;
		border-radius: var(--radius-md);
		overflow: hidden;
	}

	.button-progress-fill {
		position: absolute;
		top: 0;
		left: 0;
		height: 100%;
		background-color: hsl(var(--primary) / 0.3);
		transition: width 0.3s ease-out;
		pointer-events: none;
		z-index: 0;
	}

	:global(.submit-button) {
		width: 100%;
		position: relative;
		z-index: 1;
	}

	:global(.submit-button.is-uploading) {
		background-color: hsl(var(--primary) / 0.8);
	}

	@media (max-width: 768px) {
		.contribute-page {
			padding: 1rem 0.75rem;
		}

		.page-title {
			font-size: 1.5rem;
		}

		.video-grid {
			grid-template-columns: 1fr;
		}

		:global(.info-content) {
			flex-direction: column;
		}
	}
</style>
