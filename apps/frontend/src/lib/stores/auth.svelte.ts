/**
 * Authentication store for Firebase Auth state management
 * Uses Svelte 5 runes for reactivity
 */

import {
	getAuth,
	onAuthStateChanged,
	signInWithEmailAndPassword,
	createUserWithEmailAndPassword,
	signInWithPopup,
	GoogleAuthProvider,
	signOut as firebaseSignOut,
	type User as FirebaseUser
} from 'firebase/auth';
import { ref, get } from 'firebase/database';
import { getFirebaseDatabase } from '$lib/firebase';
import type { AuthState, User } from '$lib/types';
import { type Result, Err, AppError, tryAsync } from '$lib/result';

/**
 * Fetch administrator status from RTDB
 */
async function fetchAdminStatus(uid: string): Promise<boolean> {
	try {
		const db = getFirebaseDatabase();
		const adminRef = ref(db, `users/${uid}/administrator`);
		const snapshot = await get(adminRef);
		return snapshot.val() === true;
	} catch (e) {
		console.error('Failed to fetch admin status:', e);
		return false;
	}
}

/**
 * Convert Firebase user to our User type
 */
async function toUser(firebaseUser: FirebaseUser): Promise<User> {
	const administrator = await fetchAdminStatus(firebaseUser.uid);
	return {
		uid: firebaseUser.uid,
		email: firebaseUser.email ?? '',
		displayName: firebaseUser.displayName ?? firebaseUser.email ?? 'User',
		photoURL: firebaseUser.photoURL ?? '',
		emailVerified: firebaseUser.emailVerified,
		administrator
	};
}

/**
 * Authentication store managing Firebase auth state
 */
class AuthStore {
	#state = $state<AuthState>({ status: 'loading' });
	#unsubscribe: (() => void) | undefined;

	/**
	 * Get the current auth state
	 */
	get state(): AuthState {
		return this.#state;
	}

	/**
	 * Initialize the auth listener - call this once on app startup
	 */
	initialize(): void {
		if (this.#unsubscribe) return; // Already initialized

		const auth = getAuth();
		this.#unsubscribe = onAuthStateChanged(auth, async (firebaseUser) => {
			if (firebaseUser) {
				const user = await toUser(firebaseUser);
				this.#state = {
					status: 'authenticated',
					user
				};
			} else {
				this.#state = { status: 'unauthenticated' };
			}
		});
	}

	/**
	 * Clean up the auth listener
	 */
	destroy(): void {
		this.#unsubscribe?.();
		this.#unsubscribe = undefined;
	}

	/**
	 * Sign in with email and password
	 */
	async signInWithEmail(email: string, password: string): Promise<Result<User, AppError>> {
		const auth = getAuth();

		const result = await tryAsync(async () => {
			const credential = await signInWithEmailAndPassword(auth, email, password);
			return await toUser(credential.user);
		}, 'Failed to sign in');

		return result;
	}

	/**
	 * Create a new account with email and password
	 */
	async signUpWithEmail(email: string, password: string): Promise<Result<User, AppError>> {
		const auth = getAuth();

		const result = await tryAsync(async () => {
			const credential = await createUserWithEmailAndPassword(auth, email, password);
			return await toUser(credential.user);
		}, 'Failed to create account');

		return result;
	}

	/**
	 * Sign in with Google popup
	 */
	async signInWithGoogle(): Promise<Result<User, AppError>> {
		const auth = getAuth();
		const provider = new GoogleAuthProvider();

		const result = await tryAsync(async () => {
			const credential = await signInWithPopup(auth, provider);
			return await toUser(credential.user);
		}, 'Failed to sign in with Google');

		return result;
	}

	/**
	 * Sign out the current user
	 */
	async signOut(): Promise<Result<void, AppError>> {
		const auth = getAuth();

		return tryAsync(() => firebaseSignOut(auth), 'Failed to sign out');
	}

	/**
	 * Get the current user's ID token for API calls
	 */
	async getIdToken(): Promise<Result<string, AppError>> {
		const auth = getAuth();
		const user = auth.currentUser;

		if (!user) {
			return Err(new AppError('No authenticated user'));
		}

		return tryAsync(() => user.getIdToken(), 'Failed to get authentication token');
	}
}

/**
 * Singleton auth store instance
 */
export const authStore = new AuthStore();
