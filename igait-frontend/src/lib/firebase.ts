/**
 * Firebase configuration and initialization
 * Import this file once in your root layout to initialize Firebase
 */

import { initializeApp, getApps, type FirebaseApp } from 'firebase/app';
import { getAuth } from 'firebase/auth';
import { getDatabase, type Database } from 'firebase/database';
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
	return getAuth();
}

/**
 * Get Firebase Realtime Database instance
 */
export function getFirebaseDatabase(): Database {
	initializeFirebase();
	return getDatabase();
}
