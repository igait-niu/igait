/**
 * Firebase configuration and initialization
 * Import this file once in your root layout to initialize Firebase
 */

import { initializeApp, getApps, type FirebaseApp } from 'firebase/app';
import { getAuth, connectAuthEmulator, type Auth } from 'firebase/auth';
import { getDatabase, connectDatabaseEmulator, type Database } from 'firebase/database';
import { type Option, Some, None } from '$lib/result';

/**
 * Firebase configuration
 * These values are safe to expose - security is handled by Firebase Rules
 */
const firebaseConfig = {
	apiKey: import.meta.env.VITE_FIREBASE_API_KEY,
	authDomain: import.meta.env.VITE_FIREBASE_AUTH_DOMAIN,
	projectId: import.meta.env.VITE_FIREBASE_PROJECT_ID,
	storageBucket: import.meta.env.VITE_FIREBASE_STORAGE_BUCKET,
	messagingSenderId: import.meta.env.VITE_FIREBASE_MESSAGING_SENDER_ID,
	appId: import.meta.env.VITE_FIREBASE_APP_ID,
	databaseURL: import.meta.env.VITE_FIREBASE_DATABASE_URL
};

let firebaseApp: Option<FirebaseApp> = None();
let emulatorsConnected = false;

/**
 * Wire SDK instances to the local Firebase emulator suite. Idempotent —
 * connect* calls no-op once done, but we still guard so subsequent
 * getAuth/getDatabase callers don't try to re-connect (which warns).
 * Gated by VITE_FIREBASE_USE_EMULATOR so prod builds stay untouched.
 */
function connectEmulatorsOnce(auth: Auth, db: Database) {
	if (emulatorsConnected) return;
	if (import.meta.env.VITE_FIREBASE_USE_EMULATOR !== 'true') return;
	connectAuthEmulator(auth, 'http://localhost:9099', { disableWarnings: true });
	connectDatabaseEmulator(db, 'localhost', 9000);
	emulatorsConnected = true;
}

/**
 * Initialize Firebase - safe to call multiple times
 */
export function initializeFirebase(): FirebaseApp {
	// Return existing app if already initialized
	if (firebaseApp.isSome()) {
		return firebaseApp.value;
	}

	// Check if already initialized by another part of the app
	const existingApps = getApps();
	if (existingApps.length > 0) {
		firebaseApp = Some(existingApps[0]);
		return existingApps[0];
	}

	// Initialize new app
	const app = initializeApp(firebaseConfig);
	firebaseApp = Some(app);
	return app;
}

/**
 * Get Firebase Auth instance
 */
export function getFirebaseAuth() {
	initializeFirebase();
	const auth = getAuth();
	connectEmulatorsOnce(auth, getDatabase());
	return auth;
}

/**
 * Get Firebase Realtime Database instance
 */
export function getFirebaseDatabase(): Database {
	initializeFirebase();
	const db = getDatabase();
	connectEmulatorsOnce(getAuth(), db);
	return db;
}
