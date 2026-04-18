/**
 * Central store exports for the iGait application
 */

export { authStore } from './auth.svelte';
export { errorStore } from './error.svelte';
export {
	registryStore,
	queuePathSegment,
	stageLogsKey,
	stageFilesKey,
	stageNumber,
	isRegistryLoading,
	isRegistryError,
	isRegistryLoaded,
	type StageSpec,
	type StagePanel,
	type RegistryState
} from './registry.svelte';
