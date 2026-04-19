<script lang="ts">
	import { Label } from '$lib/components/ui/label';

	interface SelectOption {
		value: string;
		label: string;
	}

	interface Props {
		label: string;
		id: string;
		value: string;
		options: SelectOption[];
		placeholder?: string;
		required?: boolean;
		disabled?: boolean;
		error?: string;
	}

	let {
		label,
		id,
		value = $bindable(),
		options,
		placeholder = 'Select an option',
		required = false,
		disabled = false,
		error
	}: Props = $props();

	function handleChange(e: Event) {
		const target = e.target as HTMLSelectElement;
		value = target.value;
	}
</script>

<div class="form-field">
	<Label for={id}>{label}{required ? ' *' : ''}</Label>
	<select
		{id}
		class="select-input"
		class:select-input--error={error}
		{disabled}
		{required}
		onchange={handleChange}
	>
		<option value="" disabled selected={!value}>{placeholder}</option>
		{#each options as option}
			<option value={option.value} selected={value === option.value}>
				{option.label}
			</option>
		{/each}
	</select>
	{#if error}
		<p class="form-field__error">{error}</p>
	{/if}
</div>

<style>
	.form-field {
		display: flex;
		flex-direction: column;
		gap: 0.375rem;
	}

	.select-input {
		height: 2.25rem;
		width: 100%;
		border-radius: var(--radius-md);
		border: 1px solid var(--input);
		background-color: var(--background);
		padding: 0 0.75rem;
		font-size: 0.875rem;
		font-weight: 500;
		color: var(--foreground);
		box-shadow: var(--shadow-xs);
		transition:
			box-shadow 0.2s,
			border-color 0.2s;
		outline: none;
		cursor: pointer;
		appearance: none;
		background-image: url("data:image/svg+xml,%3csvg xmlns='http://www.w3.org/2000/svg' fill='none' viewBox='0 0 20 20'%3e%3cpath stroke='%236b7280' stroke-linecap='round' stroke-linejoin='round' stroke-width='1.5' d='M6 8l4 4 4-4'/%3e%3c/svg%3e");
		background-position: right 0.5rem center;
		background-repeat: no-repeat;
		background-size: 1.5rem 1.5rem;
		padding-right: 2.5rem;
	}

	.select-input:focus {
		border-color: var(--ring);
		box-shadow: 0 0 0 2px color-mix(in oklch, var(--ring) 20%, transparent);
	}

	.select-input:disabled {
		cursor: not-allowed;
		opacity: 0.5;
	}

	.select-input--error {
		border-color: var(--destructive);
	}

	.select-input--error:focus {
		box-shadow: 0 0 0 2px color-mix(in oklch, var(--destructive) 20%, transparent);
	}

	.form-field__error {
		font-size: 0.75rem;
		color: var(--destructive);
	}
</style>
