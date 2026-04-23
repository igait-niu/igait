/**
 * Form validation utilities
 * Returns Result types for consistent error handling
 */

import { type Result, Ok, Err, AppError } from '$lib/result';
import { MAX_VIDEO_SIZE_BYTES, MAX_VIDEO_SIZE_MB, VALID_VIDEO_EXTENSIONS } from './config';

/**
 * Validate an email address
 */
export function validateEmail(email: string): Result<string, AppError> {
	const trimmed = email.trim();

	if (trimmed.length === 0) {
		return Err(new AppError('Email is required'));
	}

	// Basic email regex
	const emailRegex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
	if (!emailRegex.test(trimmed)) {
		return Err(new AppError('Invalid email format'));
	}

	return Ok(trimmed);
}

/**
 * Validate a required string field
 */
export function validateRequired(value: string, fieldName: string): Result<string, AppError> {
	const trimmed = value.trim();

	if (trimmed.length === 0) {
		return Err(new AppError(`${fieldName} is required`));
	}

	return Ok(trimmed);
}

/**
 * Validate a video file
 */
export function validateVideoFile(file: File, fieldName: string): Result<File, AppError> {
	// Check if file exists and has content
	if (file.size === 0) {
		return Err(new AppError(`The ${fieldName} file is empty`));
	}

	// Check file size
	if (file.size > MAX_VIDEO_SIZE_BYTES) {
		const sizeMB = (file.size / (1024 * 1024)).toFixed(1);
		return Err(
			new AppError(
				`The ${fieldName} file is too large (${sizeMB}MB). Maximum size is ${MAX_VIDEO_SIZE_MB}MB`
			)
		);
	}

	// Check MIME type
	const hasValidMimeType = file.type.startsWith('video/');

	// Check extension
	const fileName = file.name.toLowerCase();
	const hasValidExtension = VALID_VIDEO_EXTENSIONS.some((ext) => fileName.endsWith(ext));

	if (!hasValidMimeType && !hasValidExtension) {
		return Err(
			new AppError(
				`The ${fieldName} file doesn't appear to be a valid video. ` +
					`Supported formats: ${VALID_VIDEO_EXTENSIONS.join(', ')}`
			)
		);
	}

	return Ok(file);
}

/**
 * Validate a password meets minimum requirements
 */
export function validatePassword(password: string): Result<string, AppError> {
	if (password.length < 6) {
		return Err(new AppError('Password must be at least 6 characters'));
	}

	return Ok(password);
}

/**
 * Validate password confirmation matches
 */
export function validatePasswordMatch(
	password: string,
	confirmation: string
): Result<string, AppError> {
	if (password !== confirmation) {
		return Err(new AppError('Passwords do not match'));
	}

	return Ok(password);
}
