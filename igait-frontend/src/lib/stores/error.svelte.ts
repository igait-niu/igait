/**
 * Global error store for managing application-wide errors
 * Uses Svelte 5 runes for reactivity
 */

import { AppError, type Option, Some, None } from '$lib/result';

/**
 * Global error state - when set, authenticated pages should show error UI
 * and prevent rendering potentially broken content
 */
class ErrorStore {
	#error = $state<Option<AppError>>(None());

	/**
	 * Get the current error, if any
	 */
	get current(): Option<AppError> {
		return this.#error;
	}

	/**
	 * Check if there's an active error
	 */
	get hasError(): boolean {
		return this.#error.isSome();
	}

	/**
	 * Set a new error - this will trigger error UI in authenticated layouts
	 */
	setError(error: AppError): void {
		this.#error = Some(error);
	}

	/**
	 * Set error from any error-like value with optional context
	 */
	setErrorFrom(error: unknown, context?: string): void {
		const appError = AppError.from(error);
		this.#error = Some(context ? appError.withContext(context) : appError);
	}

	/**
	 * Clear the current error - allows the page to render again
	 */
	clearError(): void {
		this.#error = None();
	}
}

/**
 * Singleton error store instance
 */
export const errorStore = new ErrorStore();
