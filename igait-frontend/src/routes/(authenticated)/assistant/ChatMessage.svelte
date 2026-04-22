<script lang="ts">
	import type { AssistantMessage } from '$lib/api/types';
	import type { Job } from '$lib/api/types';
	import Markdown from '$lib/components/Markdown.svelte';
	import * as Avatar from '$lib/components/ui/avatar';
	import { Badge } from '$lib/components/ui/badge';
	import { Bot, User, Info, Table, TriangleAlert } from '@lucide/svelte';

	let { message }: { message: AssistantMessage } = $props();

	/**
	 * Format a Jobs-type message as a Markdown table
	 * matching the old PoC format exactly.
	 */
	function formatJobsTable(jobs: Job[]): string {
		let body = '| Date | Status | Status Message | Age | Height | Weight | Sex |';
		body += '\n| --- | --- | --- | --- | --- | --- | --- |';

		for (const job of jobs) {
			const date = new Date(job.timestamp * 1000);
			body += `\n| ${date.toDateString()} | ${job.status.code} | ${job.status.value} | ${job.age} | ${job.height} | ${job.weight} | ${job.sex} |`;
		}

		return body;
	}

	const displayContent = $derived(
		message.type === 'Jobs' && Array.isArray(message.content)
			? formatJobsTable(message.content as Job[])
			: (message.content as string)
	);

	const _isBot = $derived(
		message.type === 'Message' || message.type === 'Typing' || message.type === 'Jobs'
	);
	const isUser = $derived(message.type === 'You');
	const isInfo = $derived(message.type === 'Info');
	const isError = $derived(message.type === 'Error');
</script>

<!-- Info messages — full-width, centered, subtle -->
{#if isInfo}
	<div class="info-row">
		<div class="info-bubble">
			<Info class="info-icon" />
			<Markdown content={displayContent} />
		</div>
	</div>

	<!-- Error messages -->
{:else if isError}
	<div class="info-row">
		<div class="error-bubble">
			<TriangleAlert class="error-icon" />
			<Markdown content={displayContent} />
		</div>
	</div>

	<!-- User messages — right-aligned -->
{:else if isUser}
	<div class="message-row user-row">
		<div class="user-bubble">
			<Markdown content={displayContent} />
		</div>
		<Avatar.Root class="avatar avatar-user">
			<Avatar.Fallback class="avatar-fallback-user">
				<User class="avatar-icon" />
			</Avatar.Fallback>
		</Avatar.Root>
	</div>

	<!-- Bot messages (Message / Typing / Jobs) — left-aligned -->
{:else}
	<div class="message-row bot-row">
		<Avatar.Root class="avatar avatar-bot">
			<Avatar.Fallback class="avatar-fallback-bot">
				<Bot class="avatar-icon" />
			</Avatar.Fallback>
		</Avatar.Root>

		<div class="bot-bubble" class:jobs-bubble={message.type === 'Jobs'}>
			{#if message.type === 'Typing'}
				<div class="typing-content">
					<div class="typing-dots">
						<span></span>
						<span></span>
						<span></span>
					</div>
				</div>
			{:else}
				{#if message.type === 'Jobs'}
					<Badge variant="outline" class="jobs-badge">
						<Table class="mr-1 h-3 w-3" />
						Submissions
					</Badge>
				{/if}
				<Markdown content={displayContent} />
			{/if}
		</div>
	</div>
{/if}

<style>
	/* ---- Row layouts ---- */
	.message-row {
		display: flex;
		gap: 0.625rem;
		align-items: flex-end;
		animation: slideIn 0.3s ease-out;
	}

	.bot-row {
		justify-content: flex-start;
		padding-right: 3rem;
	}

	.user-row {
		justify-content: flex-end;
		padding-left: 3rem;
	}

	.info-row {
		display: flex;
		justify-content: center;
		animation: slideIn 0.3s ease-out;
	}

	@keyframes slideIn {
		from {
			opacity: 0;
			transform: translateY(8px);
		}
		to {
			opacity: 1;
			transform: translateY(0);
		}
	}

	/* ---- Avatar ---- */
	:global(.avatar) {
		flex-shrink: 0;
		width: 2rem;
		height: 2rem;
		box-shadow: 0 1px 3px color-mix(in oklch, var(--foreground) 8%, transparent);
	}

	:global(.avatar-fallback-bot) {
		background-color: var(--primary);
		color: var(--primary-foreground);
	}

	:global(.avatar-fallback-user) {
		background-color: var(--secondary);
		color: var(--secondary-foreground);
	}

	:global(.avatar-icon) {
		width: 1rem;
		height: 1rem;
	}

	/* ---- Chat bubbles ---- */
	.bot-bubble {
		position: relative;
		background: var(--muted);
		border: 1px solid var(--border);
		border-radius: 1rem 1rem 1rem 0.25rem;
		padding: 0.75rem 1rem;
		font-size: 0.875rem;
		line-height: 1.65;
		max-width: 100%;
		min-width: 0;
		box-shadow:
			0 1px 2px color-mix(in oklch, var(--foreground) 4%, transparent),
			0 2px 8px color-mix(in oklch, var(--foreground) 3%, transparent);
	}

	.user-bubble {
		position: relative;
		background: var(--primary);
		color: var(--primary-foreground);
		border-radius: 1rem 1rem 0.25rem 1rem;
		padding: 0.75rem 1rem;
		font-size: 0.875rem;
		line-height: 1.65;
		max-width: 100%;
		min-width: 0;
		box-shadow:
			0 1px 2px color-mix(in oklch, var(--primary) 15%, transparent),
			0 4px 12px color-mix(in oklch, var(--primary) 10%, transparent);
	}

	/* Markdown links inside user bubbles */
	.user-bubble :global(a) {
		color: var(--primary-foreground);
		text-decoration: underline;
		text-underline-offset: 2px;
	}

	/* Markdown prose inside bot bubbles */
	.bot-bubble :global(p:last-child),
	.user-bubble :global(p:last-child) {
		margin-bottom: 0;
	}

	/* ---- Info bubble ---- */
	.info-bubble {
		display: flex;
		align-items: flex-start;
		gap: 0.625rem;
		background: var(--accent);
		border: 1px solid var(--border);
		border-radius: 0.75rem;
		padding: 0.75rem 1rem;
		font-size: 0.8125rem;
		line-height: 1.65;
		color: var(--muted-foreground);
		max-width: 85%;
		box-shadow: 0 1px 4px color-mix(in oklch, var(--foreground) 3%, transparent);
	}

	:global(.info-icon) {
		flex-shrink: 0;
		width: 1rem;
		height: 1rem;
		margin-top: 0.1875rem;
		color: var(--muted-foreground);
	}

	/* ---- Error bubble ---- */
	.error-bubble {
		display: flex;
		align-items: flex-start;
		gap: 0.625rem;
		background: color-mix(in oklch, var(--destructive) 8%, transparent);
		border: 1px solid color-mix(in oklch, var(--destructive) 25%, transparent);
		border-radius: 0.75rem;
		padding: 0.75rem 1rem;
		font-size: 0.8125rem;
		line-height: 1.65;
		color: var(--destructive);
		max-width: 85%;
		box-shadow: 0 1px 4px color-mix(in oklch, var(--destructive) 6%, transparent);
	}

	:global(.error-icon) {
		flex-shrink: 0;
		width: 1rem;
		height: 1rem;
		margin-top: 0.1875rem;
	}

	/* ---- Jobs bubble — wider for tables ---- */
	.jobs-bubble {
		overflow-x: auto;
	}

	:global(.jobs-badge) {
		margin-bottom: 0.5rem;
	}

	/* ---- Typing indicator ---- */
	.typing-content {
		display: flex;
		align-items: center;
		padding: 0.25rem 0.125rem;
	}

	.typing-dots {
		display: flex;
		gap: 5px;
	}

	.typing-dots span {
		width: 7px;
		height: 7px;
		background: color-mix(in oklch, var(--muted-foreground) 50%, transparent);
		border-radius: 50%;
		animation: bounce 1.4s ease-in-out infinite;
	}

	.typing-dots span:nth-child(2) {
		animation-delay: 0.16s;
	}

	.typing-dots span:nth-child(3) {
		animation-delay: 0.32s;
	}

	@keyframes bounce {
		0%,
		80%,
		100% {
			transform: scale(0.5);
			opacity: 0.35;
		}
		40% {
			transform: scale(1);
			opacity: 1;
		}
	}

	/* ---- Responsive ---- */
	@media (max-width: 640px) {
		.bot-row {
			padding-right: 1.5rem;
		}
		.user-row {
			padding-left: 1.5rem;
		}
		.info-bubble,
		.error-bubble {
			max-width: 95%;
		}
	}
</style>
