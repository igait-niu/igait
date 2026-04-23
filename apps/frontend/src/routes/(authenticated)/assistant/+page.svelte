<script lang="ts">
	import { getUser } from '$lib/hooks';
	import { authStore } from '$lib/stores';
	import { API_ENDPOINTS } from '$lib/api/config';
	import type { AssistantMessage, Job } from '$lib/api/types';
	import { Button } from '$lib/components/ui/button';
	import { Input } from '$lib/components/ui/input';
	import * as Card from '$lib/components/ui/card';
	import { ScrollArea } from '$lib/components/ui/scroll-area';
	import { Separator } from '$lib/components/ui/separator';
	import * as Tooltip from '$lib/components/ui/tooltip';
	import { Badge } from '$lib/components/ui/badge';
	import { Alert, AlertDescription } from '$lib/components/ui/alert';
	import { SendHorizontal, RefreshCw, TriangleAlert, Bot, Loader2, WifiOff } from '@lucide/svelte';
	import ChatMessage from './ChatMessage.svelte';

	const _user = getUser();

	// ----- State -----
	let messages: AssistantMessage[] = $state([
		{ type: 'Message', content: 'Hello! How can I help you today?' },
		{
			type: 'Info',
			content:
				'You can ask me about:\n* Your past submissions\n* How the iGait systems work\n* Next steps with your pre-screening'
		}
	]);
	let inputMessage = $state('');
	let waitingStatus = $state('');
	let isClosed = $state(false);
	let scrollViewportRef: HTMLElement | null = $state(null);

	// ----- WebSocket management -----
	let ws = $state<WebSocket | null>(null);
	let authSent = false;
	let heartbeatInterval: ReturnType<typeof setInterval> | null = null;
	let pongTimeout: ReturnType<typeof setTimeout> | null = null;

	/**
	 * Opens the WebSocket connection and wires up all event handlers.
	 */
	async function connect() {
		const tokenResult = await authStore.getIdToken();
		if (tokenResult.isErr()) {
			messages = [
				...messages,
				{ type: 'Error', content: `Authentication error: ${tokenResult.error.displayMessage}` }
			];
			return;
		}
		const token = tokenResult.value;

		isClosed = false;
		authSent = false;

		const socket = new WebSocket(API_ENDPOINTS.assistant);

		socket.addEventListener('open', () => {
			if (!authSent) {
				socket.send(token);
				authSent = true;
			}
			startHeartbeat(socket);
		});

		socket.addEventListener('message', (event) => {
			const data = event.data as string;

			if (data === 'pong') {
				clearPongTimeout();
				return;
			}

			try {
				const parsed: { type: string; content: unknown } = JSON.parse(data);

				switch (parsed.type) {
					case 'Jobs': {
						const jobs = parsed.content as Job[];
						let body = '| Date | Status | Status Message | Age | Height | Weight | Sex |';
						body += '\n| --- | --- | --- | --- | --- | --- | --- |';
						for (const job of jobs) {
							const d = new Date(job.timestamp * 1000);
							body += `\n| ${d.toDateString()} | ${job.status.code} | ${job.status.value} | ${job.age} | ${job.height} | ${job.weight} | ${job.sex} |`;
						}
						messages = [...messages, { type: 'Jobs', content: body }];
						break;
					}
					case 'Waiting':
						waitingStatus = parsed.content as string;
						break;
					case 'Message':
						messages = [
							...messages.filter((m) => m.type !== 'Typing'),
							{ type: 'Message', content: parsed.content as string }
						];
						waitingStatus = '';
						break;
					case 'Error':
						messages = [...messages, { type: 'Error', content: parsed.content as string }];
						waitingStatus = '';
						break;
				}
			} catch {
				console.error('Failed to parse WebSocket message:', data);
			}

			scrollToBottom();
		});

		socket.addEventListener('close', () => {
			isClosed = true;
			stopHeartbeat();
		});

		socket.addEventListener('error', (e) => {
			console.error('WebSocket error:', e);
			isClosed = true;
			stopHeartbeat();
		});

		ws = socket;
	}

	// ---- Heartbeat (ping/pong) ----
	function startHeartbeat(socket: WebSocket) {
		stopHeartbeat();
		heartbeatInterval = setInterval(() => {
			if (socket.readyState === WebSocket.OPEN) {
				socket.send('ping');
				pongTimeout = setTimeout(() => {
					console.warn('Pong not received in time — closing socket');
					socket.close();
				}, 15_000);
			}
		}, 5_000);
	}

	function clearPongTimeout() {
		if (pongTimeout) {
			clearTimeout(pongTimeout);
			pongTimeout = null;
		}
	}

	function stopHeartbeat() {
		if (heartbeatInterval) {
			clearInterval(heartbeatInterval);
			heartbeatInterval = null;
		}
		clearPongTimeout();
	}

	// ---- Sending messages ----
	function sendMessage() {
		if (!inputMessage.trim() || !ws || ws.readyState !== WebSocket.OPEN || waitingStatus) return;

		const text = inputMessage.trim();
		ws.send(text);

		messages = [
			...messages,
			{ type: 'You', content: text },
			{ type: 'Typing', content: 'Typing...' }
		];

		waitingStatus = 'Processing your request...';
		inputMessage = '';
		scrollToBottom();
	}

	function handleKeyDown(e: KeyboardEvent) {
		if (e.key === 'Enter' && !e.shiftKey) {
			e.preventDefault();
			sendMessage();
		}
	}

	function handleRefresh() {
		if (ws) ws.close();
		messages = [
			{ type: 'Message', content: 'Hello! How can I help you today?' },
			{
				type: 'Info',
				content:
					'You can ask me about:\n* Your past submissions\n* How the iGait systems work\n* Next steps with your pre-screening'
			}
		];
		waitingStatus = '';
		connect();
	}

	function scrollToBottom() {
		requestAnimationFrame(() => {
			if (scrollViewportRef) {
				scrollViewportRef.scrollTop = scrollViewportRef.scrollHeight;
			}
		});
	}

	// Derived state
	const canSend = $derived(
		!!inputMessage.trim() && !waitingStatus && !isClosed && ws?.readyState === WebSocket.OPEN
	);
	const inputDisabled = $derived(!!waitingStatus || isClosed);

	// ---- Lifecycle ----
	$effect(() => {
		connect();
		return () => {
			stopHeartbeat();
			if (ws && ws.readyState === WebSocket.OPEN) {
				ws.close();
			}
		};
	});
</script>

<svelte:head>
	<title>Assistant - iGait</title>
</svelte:head>

<div class="assistant-page">
	<!-- Header -->
	<section class="page-header">
		<div class="header-row">
			<div>
				<h1 class="page-title">Assistant</h1>
				<p class="page-description">
					Ask questions about your submissions, next steps, or how iGait works
				</p>
			</div>
			<Badge variant={isClosed ? 'destructive' : 'secondary'} class="status-badge">
				{#if isClosed}
					<WifiOff class="mr-1 h-3 w-3" />
					Disconnected
				{:else if waitingStatus}
					<Loader2 class="mr-1 h-3 w-3 animate-spin" />
					Thinking
				{:else}
					<Bot class="mr-1 h-3 w-3" />
					Online
				{/if}
			</Badge>
		</div>
	</section>

	<!-- Connection closed banner -->
	{#if isClosed}
		<Alert variant="destructive" class="connection-alert">
			<TriangleAlert class="h-4 w-4" />
			<AlertDescription class="connection-alert-content">
				<span>Connection lost. Please reconnect to continue chatting.</span>
				<Button variant="outline" size="sm" onclick={handleRefresh}>
					<RefreshCw class="mr-2 h-3.5 w-3.5" />
					Reconnect
				</Button>
			</AlertDescription>
		</Alert>
	{/if}

	<!-- Chat Card -->
	<Card.Root class="chat-card">
		<Card.Content class="chat-card-content">
			<!-- Messages area -->
			<ScrollArea
				class="message-scroll-area"
				bind:viewportRef={scrollViewportRef}
				orientation="vertical"
			>
				<div class="message-list">
					{#each messages as message (message)}
						<ChatMessage {message} />
					{/each}
				</div>
			</ScrollArea>

			<Separator />

			<!-- Input area -->
			<div class="input-area">
				{#if waitingStatus}
					<div class="waiting-indicator">
						<Loader2 class="h-3.5 w-3.5 animate-spin text-muted-foreground" />
						<span class="waiting-text">{waitingStatus}</span>
					</div>
				{/if}

				<div class="input-wrapper">
					<Input
						type="text"
						placeholder={isClosed ? 'Reconnect to continue...' : 'Type your message...'}
						bind:value={inputMessage}
						onkeydown={handleKeyDown}
						disabled={inputDisabled}
						class="chat-input"
					/>

					<Tooltip.Provider>
						<Tooltip.Root>
							<Tooltip.Trigger>
								<Button
									variant="default"
									size="icon"
									onclick={sendMessage}
									disabled={!canSend}
									class="send-button"
								>
									<SendHorizontal class="h-4 w-4" />
								</Button>
							</Tooltip.Trigger>
							<Tooltip.Content>
								<p>Send message</p>
							</Tooltip.Content>
						</Tooltip.Root>
					</Tooltip.Provider>
				</div>
			</div>
		</Card.Content>
	</Card.Root>
</div>

<style>
	.assistant-page {
		display: flex;
		flex-direction: column;
		height: calc(100vh - 4rem);
		overflow: hidden;
	}

	/* Header */
	.page-header {
		padding: 1rem 0 0.75rem;
		flex-shrink: 0;
	}

	.header-row {
		display: flex;
		align-items: flex-start;
		justify-content: space-between;
		gap: 1rem;
	}

	.page-title {
		font-size: 1.5rem;
		font-weight: 700;
		line-height: 1.2;
		letter-spacing: -0.025em;
	}

	@media (min-width: 640px) {
		.page-title {
			font-size: 1.875rem;
		}
	}

	.page-description {
		margin-top: 0.25rem;
		color: var(--muted-foreground);
		font-size: 0.875rem;
	}

	:global(.status-badge) {
		flex-shrink: 0;
		margin-top: 0.25rem;
	}

	/* Connection alert */
	:global(.connection-alert) {
		flex-shrink: 0;
		margin-bottom: 0.75rem;
	}

	:global(.connection-alert-content) {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 0.75rem;
	}

	/* Chat card — fills remaining vertical space */
	:global(.chat-card) {
		flex: 1;
		min-height: 0;
		display: flex;
		flex-direction: column;
		margin-bottom: 0.5rem;
	}

	:global(.chat-card-content) {
		flex: 1;
		display: flex;
		flex-direction: column;
		min-height: 0;
		padding: 0 !important;
	}

	/* Scroll area fills the available space */
	:global(.message-scroll-area) {
		flex: 1;
		min-height: 0;
	}

	.message-list {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
		padding: 1rem 1.25rem;
	}

	/* Input area — pinned at bottom */
	.input-area {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		flex-shrink: 0;
		padding: 0.75rem 1.25rem;
	}

	.waiting-indicator {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		font-size: 0.8125rem;
		color: var(--muted-foreground);
	}

	.waiting-text {
		font-weight: 500;
	}

	.input-wrapper {
		display: flex;
		gap: 0.5rem;
		align-items: center;
	}

	:global(.chat-input) {
		flex: 1;
	}

	:global(.send-button) {
		flex-shrink: 0;
	}
</style>
