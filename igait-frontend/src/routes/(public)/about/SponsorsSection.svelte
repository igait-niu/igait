<script lang="ts">
	import { Heart, Handshake } from '@lucide/svelte';

	const sponsors = [
		'Illinois Innovation Network',
		'NIH AIM-AHEAD',
		'NIU Office of Student Engagement and Experiential Learning',
		'NIU College of Health and Human Sciences',
		'SIUE Graduate School',
		'SIUE Undergraduate Research and Creative Activities'
	];

	const partners = [
		'NIU Center for Research Computing and Data',
		'NIU Center for the Interdisciplinary Study of Language and Literacy',
		'NIU Autism Caregiver Group',
		'NIU Speech-Language-Hearing Clinic',
		'NIU Division of Research and Innovation Partnerships',
		'NIU Foundation'
	];
</script>

<section class="sponsors-section">
	<div class="section-block">
		<div class="section-header">
			<Heart class="section-icon" />
			<h2 class="section-title">Sponsors</h2>
		</div>
		<p class="section-lead">We are grateful to the following sponsors for their funding support.</p>

		<ul class="org-list">
			{#each sponsors as sponsor (sponsor)}
				<li class="org-item">{sponsor}</li>
			{/each}
		</ul>
	</div>

	<div class="section-block">
		<div class="section-header">
			<Handshake class="section-icon" />
			<h2 class="section-title">Special Thanks</h2>
		</div>
		<p class="section-lead">We appreciate the continued support of the following partners.</p>

		<ul class="org-list">
			{#each partners as partner (partner)}
				<li class="org-item">{partner}</li>
			{/each}
		</ul>
	</div>
</section>

<style>
	.sponsors-section {
		display: flex;
		flex-direction: column;
		gap: 2.5rem;
	}

	.section-header {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		margin-bottom: 0.75rem;
	}

	:global(.sponsors-section .section-icon) {
		width: 1.125rem;
		height: 1.125rem;
		color: hsl(var(--primary));
	}

	.section-title {
		font-size: 1.25rem;
		font-weight: 600;
		margin: 0;
	}

	.section-lead {
		font-size: 1rem;
		color: hsl(var(--muted-foreground));
		line-height: 1.7;
		margin-bottom: 1rem;
	}

	.org-list {
		list-style: none;
		margin: 0;
		padding: 0;
		display: grid;
		gap: 0;
		grid-template-columns: 1fr;
	}

	@media (min-width: 640px) {
		.org-list {
			grid-template-columns: 1fr 1fr;
		}
	}

	.org-item {
		font-size: 0.875rem;
		color: hsl(var(--foreground));
		padding: 0.5rem 0.75rem;
		border-bottom: 1px solid hsl(var(--border) / 0.5);
		line-height: 1.5;
	}

	.org-item:last-child {
		border-bottom: none;
	}

	@media (min-width: 640px) {
		.org-item:nth-last-child(-n + 2) {
			border-bottom: none;
		}

		.org-item:nth-child(odd) {
			border-right: 1px solid hsl(var(--border) / 0.5);
		}
	}
</style>
