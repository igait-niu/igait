import tailwindcss from '@tailwindcss/vite';
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

if (process.env.NODE_ENV === 'production' && !process.env.VITE_API_BASE_URL) {
	throw new Error('VITE_API_BASE_URL must be set for production build');
}

export default defineConfig({
	plugins: [tailwindcss(), sveltekit()],
	server: {
		allowedHosts: ['igaitapp.com', 'www.igaitapp.com']
	}
});
