/**
 * Custom hooks for the iGait application
 */

export { getUser } from './user';
export {
	subscribeToJobs,
	isJobsLoading,
	isJobsError,
	isJobsLoaded,
	type JobsState,
	type JobWithId
} from './jobs';
export {
	subscribeToQueues,
	subscribeToAllJobs,
	subscribeToQueueConfigs,
	subscribeToJob,
	isQueuesLoading,
	isQueuesError,
	isQueuesLoaded,
	isQueueConfigLoaded,
	setQueueRequiresApproval,
	approveQueueItem,
	queueItemToJob,
	type QueueItem,
	type FinalizeQueueItem,
	type QueuesData,
	type QueuesState,
	type AdminJob,
	type AllJobsState,
	type QueueConfigItem,
	type QueueConfigData,
	type QueueConfigState,
	type SingleJobState
} from './admin';
