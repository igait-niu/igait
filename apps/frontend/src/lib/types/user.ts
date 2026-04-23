/**
 * Core user types for the iGait application
 * These are used throughout the app for type-safe user handling
 */

/**
 * Context key for accessing the authenticated user in child components
 * Used with Svelte's setContext/getContext
 */
export const USER_CONTEXT_KEY = Symbol('authenticated-user');

/**
 * Represents an authenticated Firebase user
 * This is the concrete type used in authenticated routes - no optionals!
 */
export interface User {
	readonly uid: string;
	readonly email: string;
	readonly displayName: string;
	readonly photoURL: string;
	readonly emailVerified: boolean;
	readonly administrator: boolean;
}

/**
 * Authentication state that the auth store manages
 */
export type AuthState =
	| { readonly status: 'loading' }
	| { readonly status: 'unauthenticated' }
	| { readonly status: 'authenticated'; readonly user: User };

/**
 * Helper to check if auth state is authenticated
 */
export function isAuthenticated(
	state: AuthState
): state is { status: 'authenticated'; user: User } {
	return state.status === 'authenticated';
}

/**
 * Helper to check if auth state is loading
 */
export function isLoading(state: AuthState): state is { status: 'loading' } {
	return state.status === 'loading';
}
