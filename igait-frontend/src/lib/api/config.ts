/**
 * API configuration and base URL
 */

const apiBase = import.meta.env.VITE_API_BASE_URL;
if (!apiBase) {
	throw new Error('VITE_API_BASE_URL must be set at build time');
}
export const API_BASE_URL: string = apiBase;

export const API_ENDPOINTS = {
	contribute: `${API_BASE_URL}/contribute`,
	upload: `${API_BASE_URL}/upload`,
	assistant: `${API_BASE_URL.replace(/^http/, 'ws')}/assistant_proxied`,
	rerun: `${API_BASE_URL}/rerun`,
	files: (jobId: string) => `${API_BASE_URL}/files/${jobId}`,
	cycles: (jobId: string) => `${API_BASE_URL}/cycles/${jobId}`,
	videoEdit: (jobId: string) => `${API_BASE_URL}/video-edit/${jobId}`
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
