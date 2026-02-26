/**
 * API response types
 */

// Re-export generated types from ts-rs
export type { Job } from '../../types/Job';
export type { JobStatus } from '../../types/JobStatus';
export type { Sex } from '../../types/Sex';
export type { Ethnicity } from '../../types/Ethnicity';
export type { UserRole } from '../../types/UserRole';
export type { User } from '../../types/User';

// Import types for use in this file
import type { Sex } from '../../types/Sex';
import type { Ethnicity } from '../../types/Ethnicity';
import type { UserRole } from '../../types/UserRole';
import type { Job } from '../../types/Job';

/**
 * Contribution/submission request - includes all demographic fields
 * required by the backend for gait analysis.
 */
export interface ContributionRequest {
	readonly email: string;
	readonly age: number;
	readonly sex: Sex;
	readonly ethnicity: Ethnicity;
	readonly heightFeet: number;
	readonly heightInches: number;
	readonly weight: number;
	readonly role: UserRole;
	readonly frontVideo: File;
	readonly sideVideo: File;
	readonly requiresApproval?: boolean;
}

/**
 * Assistant WebSocket message types
 */
export type AssistantMessageType =
	| 'Error'
	| 'Message'
	| 'Waiting'
	| 'You'
	| 'Typing'
	| 'Info'
	| 'Jobs';

export interface AssistantMessage {
	readonly type: AssistantMessageType;
	readonly content: string | Job[];
}

/**
 * Research contribution request - simplified form for contributing
 * walking videos to help improve iGait's models.
 */
export interface ResearchContributionRequest {
	readonly name: string;
	readonly email: string;
	readonly frontVideo: File;
	readonly sideVideo: File;
}

/**
 * Progress callback type
 */
export type ProgressCallback = (progress: number) => void;

/**
 * Response from the rerun endpoint
 */
export interface RerunResponse {
	readonly success: boolean;
	readonly message: string;
	readonly objects_deleted: number;
}

/**
 * A single file entry with name and presigned URL
 */
export interface FileEntry {
	readonly name: string;
	readonly url: string;
}

/**
 * Response from the files endpoint - stages mapped to their files
 */
export interface JobFilesResponse {
	readonly stages: Record<string, FileEntry[]>;
}

/**
 * A single gait cycle with start/end frame numbers and side.
 */
export interface GaitCycle {
	start: number;
	end: number;
	side: 'L' | 'R';
}

/**
 * Request body for updating cycle data
 */
export interface UpdateCyclesRequest {
	readonly file_name: string;
	readonly gait_cycles: GaitCycle[];
}

/**
 * Response from the cycles endpoint
 */
export interface UpdateCyclesResponse {
	readonly success: boolean;
	readonly message: string;
}
