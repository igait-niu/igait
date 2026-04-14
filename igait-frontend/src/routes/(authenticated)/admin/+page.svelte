<script lang="ts">
	import { goto } from '$app/navigation';

	// Redirect to queues by default
	$effect(() => {
		goto('/admin/queues');
	});
</script>

<p>Redirecting...</p>
