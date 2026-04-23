/**
 * API configuration and base URL
 */

const apiBase = import.meta.env.VITE_API_BASE_URL;
if (!apiBase) {
	throw new Error('VITE_API_BASE_URL must be set at build time');
}
export const API_BASE_URL: string = apiBase;

const V1 = `${API_BASE_URL}/api/v1`;

export const API_ENDPOINTS = {
	contribute: `${V1}/contribute`,
	upload: `${V1}/upload`,
	assistant: `${V1.replace(/^http/, 'ws')}/assistant_proxied`,
	rerun: `${V1}/rerun`,
	files: (jobId: string) => `${V1}/files/${jobId}`,
	cycles: (jobId: string) => `${V1}/cycles/${jobId}`,
	videoEdit: (jobId: string) => `${V1}/video-edit/${jobId}`
} as const;

/**
 * Default timeout for API requests (60 seconds for large video files)
 */
export const DEFAULT_TIMEOUT_MS = 60_000;

/**
 * Maximum file sizes
 */
export const MAX_VIDEO_SIZE_BYTES = 500 * 1024 * 1024; // 500MB
export const MAX_VIDEO_SIZE_MB = 500;

/**
 * Supported video extensions
 */
export const VALID_VIDEO_EXTENSIONS = [
	'.mp4',
	'.mov',
	'.avi',
	'.mkv',
	'.webm',
	'.m4v',
	'.wmv',
	'.flv'
] as const;
