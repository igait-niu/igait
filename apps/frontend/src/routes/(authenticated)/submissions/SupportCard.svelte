<script lang="ts">
	import * as Card from '$lib/components/ui/card';
	import { Button } from '$lib/components/ui/button';
	import { Mail, Send } from '@lucide/svelte';

	interface Props {
		userEmail: string;
		userId: string;
	}

	let { userEmail, userId }: Props = $props();

	function handleEmailRequest() {
		const subject = encodeURIComponent('Request for Historical Submissions');
		const body = encodeURIComponent(
			`Hello iGAIT Support Team,\n\n` +
				`I would like to request my historical submissions from the iGAIT system.\n\n` +
				`User Email: ${userEmail}\n` +
				`User ID: ${userId}\n\n` +
				`Thank you!`
		);
		window.location.href = `mailto:gaitstudy@niu.edu?subject=${subject}&body=${body}`;
	}
</script>

<Card.Root>
	<Card.Content class="support-content">
		<div class="support-text">
			<div class="support-title">
				<Mail class="icon" />
				<span>Need More Help?</span>
			</div>
			<p class="support-description">Request detailed reports or assistance via email</p>
		</div>
		<Button onclick={handleEmailRequest}>
			<Send class="button-icon" />
			Email Support Team
		</Button>
	</Card.Content>
</Card.Root>

<style>
	:global(.support-content) {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 1.5rem;
		padding: 1rem 1.5rem !important;
	}

	.support-text {
		flex: 1;
		text-align: left;
	}

	.support-title {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		font-size: 1rem;
		font-weight: 600;
		margin-bottom: 0.25rem;
	}

	.support-title :global(.icon) {
		height: 1.25rem;
		width: 1.25rem;
		color: hsl(var(--primary));
	}

	.support-description {
		margin: 0;
		font-size: 0.875rem;
		color: hsl(var(--muted-foreground));
	}

	:global(.button-icon) {
		margin-right: 0.5rem;
		height: 1rem;
		width: 1rem;
	}

	@media (max-width: 640px) {
		:global(.support-content) {
			flex-direction: column;
			align-items: flex-start;
			gap: 1rem;
		}
	}
</style>
