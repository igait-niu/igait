<script lang="ts">
	import { getUser } from '$lib/hooks';
	import { submitContribution, type ContributionRequest, type ProgressCallback } from '$lib/api';
	import { Button } from '$lib/components/ui/button';
	import * as Card from '$lib/components/ui/card';
	import { Input } from '$lib/components/ui/input';
	import { Label } from '$lib/components/ui/label';
	import { Alert, AlertDescription } from '$lib/components/ui/alert';
	import { Loader2, Upload } from '@lucide/svelte';
	import { Switch } from '$lib/components/ui/switch';
	import { toast } from 'svelte-sonner';
	import { goto } from '$app/navigation';
	import { type Option, None, Some, AppError } from '$lib/result';
	import VideoUploadArea from './VideoUploadArea.svelte';
	import FormSelect from './FormSelect.svelte';
	import type { Ethnicity, Sex, UserRole } from '$lib/api/types';

	const user = getUser();

	// Demographic fields
	let age = $state('');
	let sex = $state<Sex | ''>('');
	let ethnicity = $state<Ethnicity | ''>('');
	let heightFeet = $state('');
	let heightInches = $state('');
	let weight = $state('');
	let role = $state<UserRole | ''>('');

	// Video fields
	let frontVideo: File | undefined = $state(undefined);
	let sideVideo: File | undefined = $state(undefined);

	// Approval
	let requiresApproval = $state(false);

	// Form state
	let isSubmitting = $state(false);
	let progress = $state(0);
	let statusMessage = $state('');
	let error: Option<AppError> = $state(None());

	// Select options
	const sexOptions = [
		{ value: 'M', label: 'Male' },
		{ value: 'F', label: 'Female' },
		{ value: 'O', label: 'Other' }
	];

	const ethnicityOptions = [
		{ value: 'africanAmerican', label: 'African American/Black' },
		{ value: 'nativeAmerican', label: 'Native American/American Indian' },
		{ value: 'asian', label: 'Asian' },
		{ value: 'hispanic', label: 'Hispanic/Latino' },
		{ value: 'caucasian', label: 'Caucasian/White' },
		{ value: 'pacificIslander', label: 'Pacific Islander' }
	];

	const roleOptions = [
		{ value: 'parent', label: 'Parent' },
		{ value: 'doctor', label: 'Medical Professional' },
		{ value: 'schoolOfficial', label: 'School Official' },
		{ value: 'sibling', label: 'Sibling' },
		{ value: 'grandparent', label: 'Grandparent' },
		{ value: 'self', label: 'Self' }
	];

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

	// Validation helpers
	const isFormValid = $derived(
		age !== '' &&
			sex !== '' &&
			ethnicity !== '' &&
			heightFeet !== '' &&
			heightInches !== '' &&
			weight !== '' &&
			role !== '' &&
			frontVideo !== undefined &&
			sideVideo !== undefined
	);

	async function handleSubmit(e: Event) {
		e.preventDefault();
		error = None();

		if (!frontVideo || !sideVideo) {
			error = Some(new AppError('Please select both front and side videos'));
			return;
		}

		if (!sex || !ethnicity || !role) {
			error = Some(new AppError('Please fill out all required fields'));
			return;
		}

		isSubmitting = true;
		progress = 0;
		statusMessage = 'Preparing upload...';

		const request: ContributionRequest = {
			email: user.email,
			age: parseInt(age, 10),
			sex: sex as Sex,
			ethnicity: ethnicity as Ethnicity,
			heightFeet: parseInt(heightFeet, 10),
			heightInches: parseInt(heightInches, 10),
			weight: parseInt(weight, 10),
			role: role as UserRole,
			frontVideo,
			sideVideo,
			requiresApproval
		};

		const onProgress: ProgressCallback = (p) => {
			progress = p;
			if (p < 10) {
				statusMessage = 'Preparing upload...';
			} else if (p < 90) {
				statusMessage = 'Uploading videos...';
			} else if (p < 100) {
				statusMessage = 'Processing on server...';
			} else {
				statusMessage = 'Submission complete!';
			}
		};

		try {
			const result = await submitContribution(request, onProgress);

			if (result.isErr()) {
				error = Some(result.error);
				isSubmitting = false;
				progress = 0;
				statusMessage = '';
			} else {
				statusMessage = 'Submission complete! Redirecting...';
				toast.success(result.value);
				setTimeout(() => goto('/dashboard'), 1500);
			}
		} catch (err) {
			error = Some(
				new AppError(err instanceof Error ? err.message : 'An unexpected error occurred')
			);
			isSubmitting = false;
			progress = 0;
			statusMessage = '';
		}
	}
</script>

<svelte:head>
	<title>New Submission - iGait</title>
</svelte:head>

<div class="submit-page">
	<section class="page-header">
		<h1 class="page-title">New Submission</h1>
		<p class="page-description">Upload your walking videos for gait analysis</p>
	</section>

	<Card.Root class="submit-card">
		<Card.Header class="compact-header">
			<Card.Title>Patient & Video Information</Card.Title>
			<Card.Description>
				Please provide patient details and upload both front and side view videos.
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

			<form onsubmit={handleSubmit} class="submit-form">
				<!-- Patient Information Section -->
				<fieldset class="form-section">
					<legend class="form-section__title">Patient Information</legend>

					<div class="form-grid">
						<div class="form-group">
							<Label for="age">Age (years) *</Label>
							<Input
								id="age"
								type="number"
								min="1"
								max="115"
								placeholder="Enter age"
								bind:value={age}
								disabled={isSubmitting}
								required
							/>
						</div>

						<FormSelect
							label="Sex"
							id="sex"
							bind:value={sex}
							options={sexOptions}
							placeholder="Select sex"
							required
							disabled={isSubmitting}
						/>

						<FormSelect
							label="Ethnicity"
							id="ethnicity"
							bind:value={ethnicity}
							options={ethnicityOptions}
							placeholder="Select ethnicity"
							required
							disabled={isSubmitting}
						/>

						<FormSelect
							label="Your Role"
							id="role"
							bind:value={role}
							options={roleOptions}
							placeholder="Your relationship"
							required
							disabled={isSubmitting}
						/>

						<div class="form-group height-group">
							<Label>Height (ft / in) *</Label>
							<div class="height-inputs">
								<Input
									id="heightFeet"
									type="number"
									min="1"
									max="8"
									placeholder="Feet"
									bind:value={heightFeet}
									disabled={isSubmitting}
									required
								/>
								<Input
									id="heightInches"
									type="number"
									min="0"
									max="11"
									placeholder="Inches"
									bind:value={heightInches}
									disabled={isSubmitting}
									required
								/>
							</div>
						</div>

						<div class="form-group">
							<Label for="weight">Weight (lbs) *</Label>
							<Input
								id="weight"
								type="number"
								min="1"
								max="500"
								placeholder="Enter weight"
								bind:value={weight}
								disabled={isSubmitting}
								required
							/>
						</div>
					</div>
				</fieldset>

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

				<!-- Options Section -->
				<fieldset class="form-section">
					<legend class="form-section__title">Options</legend>

					<label class="approval-option">
						<div class="approval-text">
							<span class="approval-label">Request Manual Approval</span>
							<span class="approval-description"
								>When enabled, an administrator must manually review your submission before it is
								processed.</span
							>
						</div>
						<Switch
							checked={requiresApproval}
							onCheckedChange={(v) => (requiresApproval = v)}
							disabled={isSubmitting}
						/>
					</label>
				</fieldset>

				<div class="submit-button-area">
					<div class="submit-button-container">
						{#if isSubmitting}
							<div class="button-progress-fill" style="width: {Math.min(progress, 100)}%"></div>
						{/if}
						<Button
							type="submit"
							class="submit-button {isSubmitting ? 'is-uploading' : ''}"
							disabled={isSubmitting || !isFormValid}
						>
							{#if isSubmitting}
								<Loader2 class="mr-2 h-4 w-4 animate-spin" />
								{statusMessage} ({Math.min(progress, 100)}%)
							{:else}
								<Upload class="mr-2 h-4 w-4" />
								Submit for Analysis
							{/if}
						</Button>
					</div>
					{#if isSubmitting}
						<p class="upload-hint">
							Please don't close this page — large videos may take a moment to upload.
						</p>
					{/if}
				</div>
			</form>
		</Card.Content>
	</Card.Root>
</div>

<style>
	.submit-page {
		max-width: 1000px;
		margin: 0 auto;
		padding: 1.5rem 1rem;
	}

	.page-header {
		margin-bottom: 1.5rem;
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
		color: var(--muted-foreground);
	}

	:global(.compact-header) {
		padding-bottom: 1rem !important;
	}

	.submit-form {
		display: flex;
		flex-direction: column;
		gap: 1.5rem;
	}

	.submit-form :global(input[type='number']) {
		-moz-appearance: textfield;
		appearance: textfield;
	}

	.submit-form :global(input[type='number']::-webkit-outer-spin-button),
	.submit-form :global(input[type='number']::-webkit-inner-spin-button) {
		-webkit-appearance: none;
		margin: 0;
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
		color: var(--foreground);
		padding-bottom: 0.5rem;
		border-bottom: 1px solid var(--border);
		margin: 0 0 0.75rem 0;
	}

	.form-grid {
		display: grid;
		grid-template-columns: repeat(2, 1fr);
		gap: 1rem;
	}

	.form-group {
		display: flex;
		flex-direction: column;
		gap: 0.375rem;
	}

	.height-group {
		grid-column: span 1;
	}

	.height-inputs {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 0.5rem;
	}

	.video-grid {
		display: grid;
		grid-template-columns: repeat(2, 1fr);
		gap: 1rem;
	}

	/* Approval toggle */
	.approval-option {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 1rem;
		padding: 0.875rem 1rem;
		background: color-mix(in oklch, var(--muted) 40%, transparent);
		border: 1px solid var(--border);
		border-radius: var(--radius-md);
		cursor: pointer;
	}

	.approval-text {
		display: flex;
		flex-direction: column;
		gap: 0.125rem;
	}

	.approval-label {
		font-size: 0.875rem;
		font-weight: 500;
		color: var(--foreground);
	}

	.approval-description {
		font-size: 0.75rem;
		color: var(--muted-foreground);
		line-height: 1.4;
	}

	/* Error details styling */
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
		color: color-mix(in oklch, var(--destructive-foreground) 80%, transparent);
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
		background-color: color-mix(in oklch, var(--destructive-foreground) 10%, transparent);
		padding: 0.5rem;
		border-radius: var(--radius-sm);
	}

	/* Submit button with integrated progress */
	.submit-button-area {
		margin-top: 0.5rem;
	}

	.submit-button-container {
		position: relative;
		border-radius: var(--radius-md);
		overflow: hidden;
	}

	.button-progress-fill {
		position: absolute;
		top: 0;
		left: 0;
		height: 100%;
		background-color: color-mix(in oklch, var(--primary) 30%, transparent);
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
		background-color: color-mix(in oklch, var(--primary) 80%, transparent);
	}

	.upload-hint {
		text-align: center;
		font-size: 0.8125rem;
		color: var(--muted-foreground);
		margin-top: 0.5rem;
	}

	@media (max-width: 768px) {
		.submit-page {
			padding: 1rem 0.75rem;
		}

		.page-title {
			font-size: 1.5rem;
		}

		.form-grid {
			grid-template-columns: 1fr;
		}

		.video-grid {
			grid-template-columns: 1fr;
		}

		.height-group {
			grid-column: span 1;
		}
	}
</style>
