/**
 * Hook for accessing the authenticated user in child components
 * Must be used within the (authenticated) route group
 */

import { getContext } from 'svelte';
import { type User, USER_CONTEXT_KEY } from '$lib/types';

/**
 * Get the authenticated user from context
 * This will always return a User (not Option<User>) because
 * the authenticated layout guarantees the user exists
 *
 * @throws Error if used outside of authenticated routes
 */
export function getUser(): User {
	const user = getContext<User>(USER_CONTEXT_KEY);

	if (!user) {
		throw new Error(
			'getUser() must be called within an authenticated route. ' +
				'Make sure this component is rendered inside the (authenticated) route group.'
		);
	}

	return user;
}
