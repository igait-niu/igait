<script lang="ts">
	import { Badge } from '$lib/components/ui/badge';
	import type { JobStatus } from '../../../types/JobStatus';
	import { CheckCircle2, Clock, XCircle, Activity } from '@lucide/svelte';

	type Props = {
		status: JobStatus;
	};

	let { status }: Props = $props();

	function getStatusVariant(
		code: JobStatus['code']
	): 'default' | 'secondary' | 'destructive' | 'outline' {
		switch (code) {
			case 'Complete':
				return 'default';
			case 'Processing':
				return 'secondary';
			case 'Error':
				return 'destructive';
			case 'Submitted':
			default:
				return 'outline';
		}
	}

	function getStatusIcon(code: JobStatus['code']) {
		switch (code) {
			case 'Complete':
				return CheckCircle2;
			case 'Processing':
				return Activity;
			case 'Error':
				return XCircle;
			case 'Submitted':
			default:
				return Clock;
		}
	}

	const variant = $derived(getStatusVariant(status.code));
	const StatusIcon = $derived(getStatusIcon(status.code));
</script>

<Badge {variant} class="flex w-fit items-center gap-1.5">
	<StatusIcon class="h-3 w-3" />
	{status.value}
</Badge>
