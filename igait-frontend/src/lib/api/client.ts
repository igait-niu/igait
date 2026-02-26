/**
 * Core API client with Result-based error handling
 * All API calls return Result<T, AppError> - never throw!
 */

import { type Result, Ok, Err, AppError, tryAsync } from '$lib/result';
import { authStore } from '$lib/stores';
import { API_ENDPOINTS, DEFAULT_TIMEOUT_MS } from './config';
import type { RerunResponse, JobFilesResponse, UpdateCyclesResponse, GaitCycle } from './types';
import type { ContributionRequest, ProgressCallback, ResearchContributionRequest } from './types';
import { validateVideoFile, validateRequired, validateEmail } from './validation';

/**
 * Validate all submission fields
 */
function validateSubmission(request: ContributionRequest): Result<void, AppError> {
	// Validate email
	const emailResult = validateEmail(request.email);
	if (emailResult.isErr()) {
		return Err(emailResult.error.withContext('Invalid submission'));
	}

	// Validate age
	if (request.age < 1 || request.age > 115) {
		return Err(new AppError('Age must be between 1 and 115').withContext('Invalid submission'));
	}

	// Validate weight
	if (request.weight < 1 || request.weight > 500) {
		return Err(
			new AppError('Weight must be between 1 and 500 lbs').withContext('Invalid submission')
		);
	}

	// Validate height
	if (request.heightFeet < 1 || request.heightFeet > 8) {
		return Err(
			new AppError('Height (feet) must be between 1 and 8').withContext('Invalid submission')
		);
	}
	if (request.heightInches < 0 || request.heightInches > 11) {
		return Err(
			new AppError('Height (inches) must be between 0 and 11').withContext('Invalid submission')
		);
	}

	// Validate videos
	const frontVideoResult = validateVideoFile(request.frontVideo, 'front video');
	if (frontVideoResult.isErr()) {
		return Err(frontVideoResult.error.withContext('Invalid submission'));
	}

	const sideVideoResult = validateVideoFile(request.sideVideo, 'side video');
	if (sideVideoResult.isErr()) {
		return Err(sideVideoResult.error.withContext('Invalid submission'));
	}

	return Ok(undefined);
}

/**
 * Submit a gait analysis contribution
 */
export async function submitContribution(
	request: ContributionRequest,
	onProgress?: ProgressCallback
): Promise<Result<string, AppError>> {
	// Validate all fields first
	const validationResult = validateSubmission(request);
	if (validationResult.isErr()) {
		return validationResult as unknown as Result<string, AppError>;
	}

	// Get auth token for Bearer authentication
	const tokenResult = await authStore.getIdToken();
	if (tokenResult.isErr()) {
		return Err(tokenResult.error.withContext('Authentication required for submission'));
	}

	onProgress?.(5);

	// Build form data matching backend UploadRequestArguments
	const formData = new FormData();
	formData.append('age', request.age.toString());
	formData.append('ethnicity', request.ethnicity);
	formData.append('sex', request.sex);
	formData.append('height', `${request.heightFeet}'${request.heightInches}`);
	formData.append('weight', request.weight.toString());
	formData.append('email', request.email);
	if (request.requiresApproval) {
		formData.append('requires_approval', 'true');
	}
	formData.append('fileuploadfront', request.frontVideo);
	formData.append('fileuploadside', request.sideVideo);

	onProgress?.(5);

	// Use XMLHttpRequest for real upload progress tracking
	const result = await new Promise<Result<string, AppError>>((resolve) => {
		const xhr = new XMLHttpRequest();

		// Inactivity timeout — resets whenever progress is made
		let timeoutId = setTimeout(() => xhr.abort(), DEFAULT_TIMEOUT_MS);
		const resetTimeout = () => {
			clearTimeout(timeoutId);
			timeoutId = setTimeout(() => xhr.abort(), DEFAULT_TIMEOUT_MS);
		};

		xhr.upload.addEventListener('progress', (e) => {
			resetTimeout();
			if (e.lengthComputable) {
				// Map upload progress to 5–90% range
				const uploadPercent = 5 + Math.round((e.loaded / e.total) * 85);
				onProgress?.(uploadPercent);
			}
		});

		xhr.addEventListener('load', () => {
			clearTimeout(timeoutId);
			if (xhr.status >= 200 && xhr.status < 300) {
				onProgress?.(100);
				resolve(
					Ok(
						'Your submission has been received! You will receive an email with your results shortly.'
					)
				);
			} else {
				const errorText = xhr.responseText || 'Unknown error';
				resolve(
					Err(
						new AppError(`Server error (${xhr.status}): ${errorText}`).withContext(
							'Submission failed'
						)
					)
				);
			}
		});

		xhr.addEventListener('error', () => {
			clearTimeout(timeoutId);
			resolve(
				Err(
					new AppError('Network error. Please check your internet connection.').withContext(
						'Submission failed'
					)
				)
			);
		});

		xhr.addEventListener('abort', () => {
			clearTimeout(timeoutId);
			resolve(
				Err(
					new AppError(
						'Request timed out. Your files might be too large or your connection is slow.'
					).withContext('Submission failed')
				)
			);
		});

		xhr.open('POST', API_ENDPOINTS.upload);
		xhr.setRequestHeader('Authorization', `Bearer ${tokenResult.value}`);
		xhr.send(formData);
	});

	return result;
}

/**
 * Validate research contribution fields
 */
function validateResearchContribution(
	request: ResearchContributionRequest
): Result<void, AppError> {
	const nameResult = validateRequired(request.name, 'Name');
	if (nameResult.isErr()) {
		return Err(nameResult.error.withContext('Invalid contribution'));
	}

	const emailResult = validateEmail(request.email);
	if (emailResult.isErr()) {
		return Err(emailResult.error.withContext('Invalid contribution'));
	}

	const frontVideoResult = validateVideoFile(request.frontVideo, 'front video');
	if (frontVideoResult.isErr()) {
		return Err(frontVideoResult.error.withContext('Invalid contribution'));
	}

	const sideVideoResult = validateVideoFile(request.sideVideo, 'side video');
	if (sideVideoResult.isErr()) {
		return Err(sideVideoResult.error.withContext('Invalid contribution'));
	}

	return Ok(undefined);
}

/**
 * Submit a research contribution (videos only, no demographic data)
 */
export async function submitResearchContribution(
	request: ResearchContributionRequest,
	onProgress?: ProgressCallback
): Promise<Result<string, AppError>> {
	const validationResult = validateResearchContribution(request);
	if (validationResult.isErr()) {
		return validationResult as unknown as Result<string, AppError>;
	}

	// Get auth token for Bearer authentication
	const tokenResult = await authStore.getIdToken();
	if (tokenResult.isErr()) {
		return Err(tokenResult.error.withContext('Authentication required for contribution'));
	}

	onProgress?.(5);

	const formData = new FormData();
	formData.append('name', request.name);
	formData.append('email', request.email);
	formData.append('fileuploadfront', request.frontVideo);
	formData.append('fileuploadside', request.sideVideo);

	onProgress?.(15);

	const controller = new AbortController();
	const timeoutId = setTimeout(() => controller.abort(), DEFAULT_TIMEOUT_MS);

	const result = await tryAsync(async () => {
		const response = await fetch(API_ENDPOINTS.contribute, {
			method: 'POST',
			body: formData,
			signal: controller.signal,
			headers: {
				Authorization: `Bearer ${tokenResult.value}`
			}
		});

		onProgress?.(80);

		if (!response.ok) {
			const errorText = await response.text().catch(() => 'Unknown error');
			throw new Error(`Server error (${response.status}): ${errorText}`);
		}

		onProgress?.(100);
		return 'Thank you for your contribution! Your videos have been submitted for research.';
	}, 'Failed to submit research contribution');

	clearTimeout(timeoutId);

	if (result.isErr()) {
		const error = result.error;

		if (error.rootCause.includes('aborted') || error.rootCause.includes('abort')) {
			return Err(
				new AppError(
					'Request timed out. Your files might be too large or your connection is slow.'
				).withContext('Contribution failed')
			);
		}

		if (error.rootCause.includes('NetworkError') || error.rootCause.includes('fetch')) {
			return Err(
				new AppError('Network error. Please check your internet connection.').withContext(
					'Contribution failed'
				)
			);
		}
	}

	return result;
}

/**
 * Generic authenticated API request helper
 */
export async function authenticatedFetch<T>(
	url: string,
	options: RequestInit = {}
): Promise<Result<T, AppError>> {
	// Get auth token
	const tokenResult = await authStore.getIdToken();
	if (tokenResult.isErr()) {
		return tokenResult as unknown as Result<T, AppError>;
	}

	const headers = new Headers(options.headers);
	headers.set('Authorization', `Bearer ${tokenResult.value}`);

	return tryAsync(async () => {
		const response = await fetch(url, {
			...options,
			headers
		});

		if (!response.ok) {
			const errorText = await response.text().catch(() => 'Unknown error');
			throw new Error(`API error (${response.status}): ${errorText}`);
		}

		return response.json() as Promise<T>;
	}, 'API request failed');
}

/**
 * Re-run a job from a specific stage.
 * Calls the backend /rerun endpoint which cleans S3 outputs and re-queues.
 */
export async function rerunJob(
	userIdJobIndex: string,
	stage: number
): Promise<Result<RerunResponse, AppError>> {
	const lastUnderscore = userIdJobIndex.lastIndexOf('_');

	if (lastUnderscore === -1) {
		return Err(
			new AppError('Invalid job identifier format. Expected format: userId_jobIndex').withContext(
				'Failed to rerun job'
			)
		);
	}

	const userId = userIdJobIndex.slice(0, lastUnderscore);
	const jobIndexStr = userIdJobIndex.slice(lastUnderscore + 1);
	const jobIndex = parseInt(jobIndexStr, 10);

	if (Number.isNaN(jobIndex) || jobIndex < 0) {
		return Err(
			new AppError('Invalid job index in job identifier').withContext('Failed to rerun job')
		);
	}

	return authenticatedFetch<RerunResponse>(API_ENDPOINTS.rerun, {
		method: 'POST',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({ user_id: userId, job_index: jobIndex, stage })
	});
}

/**
 * Fetch presigned download URLs for all files belonging to a job.
 * Returns files grouped by stage.
 */
export async function getJobFiles(
	jobId: string
): Promise<Result<JobFilesResponse, AppError>> {
	return authenticatedFetch<JobFilesResponse>(API_ENDPOINTS.files(jobId));
}

/**
 * Update gait cycles for a stage 5 output file.
 * Admin-only: replaces the gait_cycles array in the specified JSON file.
 */
export async function updateCycles(
	jobId: string,
	fileName: string,
	gaitCycles: GaitCycle[]
): Promise<Result<UpdateCyclesResponse, AppError>> {
	return authenticatedFetch<UpdateCyclesResponse>(API_ENDPOINTS.cycles(jobId), {
		method: 'PUT',
		headers: { 'Content-Type': 'application/json' },
		body: JSON.stringify({ file_name: fileName, gait_cycles: gaitCycles })
	});
}
