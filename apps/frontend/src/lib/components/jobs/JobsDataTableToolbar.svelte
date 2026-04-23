<script lang="ts">
	import { Input } from '$lib/components/ui/input';
	import { Button } from '$lib/components/ui/button';
	import { Badge } from '$lib/components/ui/badge';
	import * as DropdownMenu from '$lib/components/ui/dropdown-menu';
	import { X, ListFilter, Search } from '@lucide/svelte';

	interface Props {
		statusFilter: string;
		searchQuery: string;
		totalCount: number;
		filteredCount: number;
		onStatusFilterChange: (value: string) => void;
		onSearchChange: (value: string) => void;
		onReset: () => void;
		hasActiveFilters: boolean;
		placeholder?: string;
	}

	let {
		statusFilter,
		searchQuery,
		totalCount,
		filteredCount,
		onStatusFilterChange,
		onSearchChange,
		onReset,
		hasActiveFilters,
		placeholder = 'Filter jobs...'
	}: Props = $props();

	function getStatusLabel(value: string): string {
		switch (value) {
			case 'completed':
				return 'Completed';
			case 'processing':
				return 'Processing';
			case 'error':
				return 'Error';
			default:
				return 'All Status';
		}
	}
</script>

<div class="toolbar">
	<div class="toolbar-left">
		<div class="search-wrapper">
			<Search class="search-icon" />
			<Input
				{placeholder}
				value={searchQuery}
				oninput={(e) => onSearchChange(e.currentTarget.value)}
				class="search-input"
			/>
		</div>
		<DropdownMenu.Root>
			<DropdownMenu.Trigger>
				<Button variant="outline" size="sm" class="filter-btn">
					<ListFilter class="filter-icon" />
					{getStatusLabel(statusFilter)}
					{#if statusFilter !== 'all'}
						<span class="filter-badge">1</span>
					{/if}
				</Button>
			</DropdownMenu.Trigger>
			<DropdownMenu.Content align="start">
				<DropdownMenu.Label>Filter by status</DropdownMenu.Label>
				<DropdownMenu.Separator />
				<DropdownMenu.Item onclick={() => onStatusFilterChange('all')}>
					All Status
				</DropdownMenu.Item>
				<DropdownMenu.Item onclick={() => onStatusFilterChange('completed')}>
					Completed
				</DropdownMenu.Item>
				<DropdownMenu.Item onclick={() => onStatusFilterChange('processing')}>
					Processing
				</DropdownMenu.Item>
				<DropdownMenu.Item onclick={() => onStatusFilterChange('error')}>Error</DropdownMenu.Item>
			</DropdownMenu.Content>
		</DropdownMenu.Root>
		{#if hasActiveFilters}
			<Button variant="ghost" size="sm" onclick={onReset} class="reset-btn">
				Reset
				<X class="reset-icon" />
			</Button>
		{/if}
	</div>
	<Badge variant="secondary" class="count-badge">
		{filteredCount} of {totalCount}
	</Badge>
</div>

<style>
	.toolbar {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 0.75rem;
		flex-wrap: wrap;
	}

	.toolbar-left {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		flex-wrap: wrap;
	}

	.search-wrapper {
		position: relative;
	}

	:global(.search-icon) {
		position: absolute;
		left: 0.625rem;
		top: 50%;
		transform: translateY(-50%);
		width: 0.875rem;
		height: 0.875rem;
		color: hsl(var(--muted-foreground));
		pointer-events: none;
	}

	:global(.search-input) {
		padding-left: 2rem !important;
		height: 2rem !important;
		font-size: 0.8125rem !important;
		width: 180px;
	}

	:global(.filter-btn) {
		height: 2rem !important;
		font-size: 0.8125rem !important;
		border-style: dashed !important;
	}

	:global(.filter-icon) {
		width: 0.875rem;
		height: 0.875rem;
		margin-right: 0.375rem;
	}

	.filter-badge {
		margin-left: 0.375rem;
		padding: 0 0.25rem;
		border-radius: 0.25rem;
		background: hsl(var(--primary));
		color: hsl(var(--primary-foreground));
		font-size: 0.6875rem;
		font-weight: 600;
	}

	:global(.reset-btn) {
		height: 2rem !important;
		padding: 0 0.5rem !important;
		font-size: 0.8125rem !important;
	}

	:global(.reset-icon) {
		width: 0.875rem;
		height: 0.875rem;
		margin-left: 0.25rem;
	}

	:global(.count-badge) {
		font-size: 0.75rem;
	}

	@media (max-width: 640px) {
		.toolbar {
			flex-direction: column;
			align-items: stretch;
		}

		:global(.search-input) {
			width: 100% !important;
		}

		.search-wrapper {
			flex: 1;
		}
	}
</style>
