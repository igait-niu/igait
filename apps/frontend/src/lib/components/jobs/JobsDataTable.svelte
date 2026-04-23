<script lang="ts">
	import * as Table from '$lib/components/ui/table';
	import { Badge } from '$lib/components/ui/badge';
	import { Button } from '$lib/components/ui/button';
	import { ArrowUpDown, ShieldCheck } from '@lucide/svelte';
	import { SvelteSet } from 'svelte/reactivity';
	import JobsDataTableToolbar from './JobsDataTableToolbar.svelte';
	import type { Job } from '../../../types/Job';
	import type { JobStatus } from '../../../types/JobStatus';
	import { registryStore, isRegistryLoaded } from '$lib/stores';

	type JobWithId = Job & { id: string };

	interface Props {
		data: (Job | JobWithId)[];
		uid?: string;
		showEmail?: boolean;
		initialStatusFilter?: string;
		selectedId?: string | null;
		selectable?: boolean;
		onRowClick?: (job: JobWithId) => void;
		onApprove?: (jobIds: string[]) => void | Promise<void>;
	}

	let {
		data,
		uid = '',
		showEmail = false,
		initialStatusFilter = 'all',
		selectedId = null,
		selectable = false,
		onRowClick,
		onApprove
	}: Props = $props();

	let selectedIds = new SvelteSet<string>();
	let approving = $state(false);

	function toggleSelect(id: string) {
		const job = jobsWithIds.find((j) => j.id === id);
		if (job?.approved) return;
		if (selectedIds.has(id)) selectedIds.delete(id);
		else selectedIds.add(id);
	}

	async function handleApprove() {
		if (!onApprove || selectedIds.size === 0 || approving) return;
		const ids = [...selectedIds];
		approving = true;
		try {
			await onApprove(ids);
			// Only clear selection on success — preserves the user's selection
			// so they can retry after an error without re-picking rows.
			selectedIds.clear();
		} finally {
			approving = false;
		}
	}

	// Create jobs with IDs - use existing id if present, otherwise generate from uid_index
	const jobsWithIds = $derived(
		data.map((job, index) => {
			if ('id' in job && job.id) {
				return job as JobWithId;
			}
			return { ...job, id: `${uid}_${index}` } as JobWithId;
		})
	);

	let sortColumn: 'date' | 'status' | null = $state(null);
	let sortDirection: 'asc' | 'desc' = $state('desc');
	let searchQuery = $state('');
	// eslint-disable-next-line svelte/prefer-writable-derived -- statusFilter is user-editable via UI; the effect only syncs the initial/prop-change value.
	let statusFilter = $state('all');

	// Update status filter when prop changes
	$effect(() => {
		statusFilter = initialStatusFilter;
	});

	// Helper to get status display info
	function getStatusInfo(status: JobStatus) {
		switch (status.code) {
			case 'Complete':
				return {
					label: status.asd ? 'ASD Indicators' : 'No ASD Indicators',
					variant: 'secondary' as const
				};
			case 'Error':
				return { label: 'Error', variant: 'destructive' as const };
			case 'Processing': {
				const registryState = registryStore.state;
				const stages = isRegistryLoaded(registryState) ? registryState.stages : [];
				const spec = stages.find((s) => s.key === status.stage);
				return {
					label: spec?.display_name ?? status.stage,
					variant: 'secondary' as const
				};
			}
			case 'Submitted':
			default:
				return { label: 'Submitted', variant: 'outline' as const };
		}
	}

	// Filter and sort data
	const filteredData = $derived.by(() => {
		let filtered = jobsWithIds;

		// Apply status filter
		if (statusFilter !== 'all') {
			filtered = filtered.filter((job) => {
				if (statusFilter === 'completed') return job.status.code === 'Complete';
				if (statusFilter === 'error') return job.status.code === 'Error';
				if (statusFilter === 'processing')
					return job.status.code === 'Processing' || job.status.code === 'Submitted';
				return true;
			});
		}

		// Apply search filter
		if (searchQuery) {
			const query = searchQuery.toLowerCase();
			filtered = filtered.filter((job) => {
				const matchesBasic =
					job.id.toLowerCase().includes(query) ||
					job.status.value.toLowerCase().includes(query) ||
					getStatusInfo(job.status).label.toLowerCase().includes(query) ||
					new Date(job.timestamp * 1000).toLocaleDateString().includes(query);

				if (showEmail) {
					return matchesBasic || job.email.toLowerCase().includes(query);
				}
				return matchesBasic;
			});
		}

		// Apply sorting - use toSorted to avoid mutation
		if (sortColumn) {
			filtered = filtered.toSorted((a, b) => {
				let aVal: number, bVal: number;

				if (sortColumn === 'date') {
					aVal = a.timestamp;
					bVal = b.timestamp;
				} else {
					// Sort order: Error > Processing > Submitted > Complete
					const statusOrder = { Error: 0, Processing: 1, Submitted: 2, Complete: 3 };
					aVal = statusOrder[a.status.code] ?? 2;
					bVal = statusOrder[b.status.code] ?? 2;
				}

				return sortDirection === 'asc' ? aVal - bVal : bVal - aVal;
			});
		}

		return filtered;
	});

	const selectableFilteredData = $derived(filteredData.filter((j) => !j.approved));

	const allFilteredSelected = $derived(
		selectableFilteredData.length > 0 && selectableFilteredData.every((j) => selectedIds.has(j.id))
	);

	function toggleSelectAll() {
		if (allFilteredSelected) {
			selectedIds.clear();
		} else {
			for (const j of selectableFilteredData) selectedIds.add(j.id);
		}
	}

	function toggleSort(column: 'date' | 'status') {
		if (sortColumn === column) {
			sortDirection = sortDirection === 'asc' ? 'desc' : 'asc';
		} else {
			sortColumn = column;
			sortDirection = 'desc';
		}
	}

	function handleStatusFilterChange(value: string) {
		statusFilter = value;
	}

	function handleSearchChange(value: string) {
		searchQuery = value;
	}

	function resetFilters() {
		statusFilter = 'all';
		searchQuery = '';
		sortColumn = null;
	}

	function formatJobId(id: string): string {
		const parts = id.split('_');
		if (parts.length >= 2) {
			const uid = parts.slice(0, -1).join('_');
			const index = parts[parts.length - 1];
			return `${uid.slice(0, 6)}…#${index}`;
		}
		return id.slice(0, 10) + '…';
	}

	function formatDate(timestamp: number): string {
		return new Date(timestamp * 1000).toLocaleDateString('en-US', {
			month: 'short',
			day: 'numeric',
			year: '2-digit'
		});
	}

	const hasActiveFilters = $derived(statusFilter !== 'all' || searchQuery !== '');
	const colCount = $derived((showEmail ? 5 : 4) + (selectable ? 1 : 0));
</script>

<div class="data-table-wrapper">
	<JobsDataTableToolbar
		{statusFilter}
		{searchQuery}
		totalCount={data.length}
		filteredCount={filteredData.length}
		onStatusFilterChange={handleStatusFilterChange}
		onSearchChange={handleSearchChange}
		onReset={resetFilters}
		{hasActiveFilters}
		placeholder={showEmail ? 'Filter by ID, email, status...' : 'Filter submissions...'}
	/>

	{#if selectable && selectedIds.size > 0}
		<div class="selection-bar">
			<span class="selection-count">{selectedIds.size} selected</span>
			<Button
				variant="default"
				size="sm"
				class="approve-selected-btn"
				onclick={handleApprove}
				disabled={approving}
			>
				<ShieldCheck class="approve-icon" />
				{approving ? 'Approving…' : 'Approve Selected'}
			</Button>
		</div>
	{/if}

	<div class="table-container">
		<Table.Root class="compact-table">
			<Table.Header>
				<Table.Row>
					{#if selectable}
						<Table.Head class="col-check">
							<input
								type="checkbox"
								checked={allFilteredSelected}
								onchange={toggleSelectAll}
								disabled={selectableFilteredData.length === 0}
								class="row-checkbox"
							/>
						</Table.Head>
					{/if}
					<Table.Head class="col-id">Job ID</Table.Head>
					{#if showEmail}
						<Table.Head class="col-email">Email</Table.Head>
					{/if}
					<Table.Head class="col-desc">Description</Table.Head>
					<Table.Head class="col-status">
						<Button variant="ghost" size="sm" class="sort-btn" onclick={() => toggleSort('status')}>
							Status
							<ArrowUpDown class="sort-icon" />
						</Button>
					</Table.Head>
					<Table.Head class="col-date">
						<Button variant="ghost" size="sm" class="sort-btn" onclick={() => toggleSort('date')}>
							Date
							<ArrowUpDown class="sort-icon" />
						</Button>
					</Table.Head>
				</Table.Row>
			</Table.Header>
			<Table.Body>
				{#if filteredData.length > 0}
					{#each filteredData as job (job.id)}
						{@const statusInfo = getStatusInfo(job.status)}
						<Table.Row
							class="data-row {selectedId === job.id ? 'row-selected' : ''} {onRowClick
								? 'row-clickable'
								: ''} {selectable && job.approved ? 'row-approved' : ''}"
							onclick={() => onRowClick?.(job)}
						>
							{#if selectable}
								<Table.Cell>
									{#if job.approved}
										<span
											class="approved-indicator"
											title="Already approved"
											onclick={(e) => e.stopPropagation()}
											role="presentation"
										>
											<ShieldCheck class="approved-icon" />
										</span>
									{:else}
										<input
											type="checkbox"
											checked={selectedIds.has(job.id)}
											onchange={() => toggleSelect(job.id)}
											onclick={(e) => e.stopPropagation()}
											class="row-checkbox"
										/>
									{/if}
								</Table.Cell>
							{/if}
							<Table.Cell>
								<span class="job-id" title={job.id}>{formatJobId(job.id)}</span>
							</Table.Cell>
							{#if showEmail}
								<Table.Cell>
									<span class="email">{job.email}</span>
								</Table.Cell>
							{/if}
							<Table.Cell>
								<span class="description">{job.status.value}</span>
							</Table.Cell>
							<Table.Cell>
								<Badge variant={statusInfo.variant} class="status-badge">
									{statusInfo.label}
								</Badge>
							</Table.Cell>
							<Table.Cell>
								<span class="date">{formatDate(job.timestamp)}</span>
							</Table.Cell>
						</Table.Row>
					{/each}
				{:else}
					<Table.Row>
						<Table.Cell colspan={colCount} class="empty-cell">No results found</Table.Cell>
					</Table.Row>
				{/if}
			</Table.Body>
		</Table.Root>
	</div>
</div>

<style>
	.data-table-wrapper {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}

	.table-container {
		border: 1px solid hsl(var(--border));
		border-radius: 0.5rem;
		overflow: hidden;
	}

	:global(.compact-table) {
		font-size: 0.8125rem;
	}

	:global(.compact-table th),
	:global(.compact-table td) {
		padding: 0.5rem 0.75rem !important;
	}

	:global(.col-id) {
		width: 100px;
	}

	:global(.col-email) {
		width: 180px;
	}

	:global(.col-desc) {
		min-width: 150px;
	}

	:global(.col-status) {
		width: 120px;
	}

	:global(.col-date) {
		width: 100px;
	}

	:global(.sort-btn) {
		margin-left: -0.75rem;
		height: 2rem !important;
		font-size: 0.8125rem !important;
		font-weight: 500 !important;
	}

	:global(.sort-icon) {
		width: 0.875rem;
		height: 0.875rem;
		margin-left: 0.25rem;
	}

	:global(.data-row) {
		transition: background-color 0.1s ease;
	}

	:global(.data-row:hover) {
		background-color: hsl(var(--muted) / 0.4);
	}

	:global(.row-clickable) {
		cursor: pointer;
	}

	:global(.row-selected) {
		background-color: hsl(var(--primary) / 0.08) !important;
		border-left: 2px solid hsl(var(--primary));
	}

	.job-id {
		font-family: ui-monospace, monospace;
		font-size: 0.75rem;
		color: hsl(var(--muted-foreground));
	}

	.email {
		font-size: 0.8125rem;
		color: hsl(var(--foreground));
	}

	.description {
		font-size: 0.8125rem;
		font-weight: 500;
	}

	:global(.status-badge) {
		font-size: 0.6875rem;
		padding: 0.125rem 0.5rem;
	}

	.date {
		font-size: 0.75rem;
		color: hsl(var(--muted-foreground));
		white-space: nowrap;
	}

	:global(.empty-cell) {
		text-align: center;
		padding: 2rem !important;
		color: hsl(var(--muted-foreground));
	}

	:global(.col-check) {
		width: 36px;
	}

	.row-checkbox {
		cursor: pointer;
		width: 0.875rem;
		height: 0.875rem;
		accent-color: hsl(var(--primary));
	}

	.row-checkbox:disabled {
		cursor: not-allowed;
		opacity: 0.4;
	}

	:global(.data-row.row-approved) {
		opacity: 0.65;
		background-color: hsl(142 76% 36% / 0.04);
	}

	:global(.data-row.row-approved:hover) {
		opacity: 0.85;
	}

	.approved-indicator {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		color: hsl(142 76% 36%);
		cursor: default;
	}

	:global(.approved-icon) {
		width: 0.9375rem;
		height: 0.9375rem;
	}

	.selection-bar {
		display: flex;
		align-items: center;
		gap: 0.75rem;
		padding: 0.5rem 0.75rem;
		background: hsl(var(--primary) / 0.06);
		border: 1px solid hsl(var(--primary) / 0.2);
		border-radius: var(--radius-sm, 0.375rem);
	}

	.selection-count {
		font-size: 0.8125rem;
		font-weight: 500;
		color: hsl(var(--primary));
	}

	:global(.approve-selected-btn) {
		height: 1.75rem !important;
		font-size: 0.75rem !important;
	}

	:global(.approve-icon) {
		width: 0.875rem;
		height: 0.875rem;
		margin-right: 0.25rem;
	}
</style>
