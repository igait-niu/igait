<script lang="ts">
	type Props = {
		timestamp: number;
	};

	let { timestamp }: Props = $props();

	const formattedDate = $derived.by(() => {
		const date = new Date(timestamp * 1000);
		return date.toLocaleDateString('en-US', {
			year: 'numeric',
			month: 'short',
			day: 'numeric',
			hour: '2-digit',
			minute: '2-digit'
		});
	});
</script>

<div class="text-sm">
	{formattedDate}
</div>
