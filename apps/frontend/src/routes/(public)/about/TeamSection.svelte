<script lang="ts">
	import { GraduationCap, Building2 } from '@lucide/svelte';

	const universities = [
		{
			name: 'Northern Illinois University',
			abbreviation: 'NIU',
			graduates: [
				'Gabriela Ibarra',
				'Mahesh Raju',
				'Ricardo Torres',
				'Alicia LaRouech',
				'Viviana Cortes',
				'Noelle Veome',
				'Hope Larson',
				'Regan Alzueta',
				'Nico Edwards-Testolin',
				'Ipsita Priyadarshinee',
				'Natalie Quinlan',
				'Harshitha Maartha'
			],
			undergrads: [
				'John White',
				'Shaivil Patel',
				'Michael Sensenbrenner',
				'Luke Ali',
				'Angelica Sanyal',
				'Zoey Proper',
				'Rachel Conolly'
			]
		},
		{
			name: 'Southern Illinois University Edwardsville',
			abbreviation: 'SIUE',
			graduates: ['Amit Shrestha', 'Prashanna Pandit'],
			undergrads: ['Cody Schaefer', 'Vinhhy Pham']
		}
	];
</script>

<section class="team-section">
	<div class="team-header">
		<GraduationCap class="section-icon" />
		<h2 class="section-title">Student Research Team</h2>
	</div>
	<p class="team-lead">
		A cross-university collaboration between researchers, engineers, and healthcare professionals.
	</p>

	<div class="university-list">
		{#each universities as uni (uni.abbreviation)}
			<div class="university-card">
				<div class="uni-header">
					<Building2 class="uni-icon" />
					<div>
						<h3 class="uni-name">{uni.name}</h3>
						<span class="uni-abbr">{uni.abbreviation}</span>
					</div>
				</div>

				<div class="uni-groups">
					<div class="student-group">
						<h4 class="group-label">Graduate Students</h4>
						<ul class="name-list">
							{#each uni.graduates as name (name)}
								<li>{name}</li>
							{/each}
						</ul>
					</div>

					<div class="student-group">
						<h4 class="group-label">Undergraduate Students</h4>
						<ul class="name-list">
							{#each uni.undergrads as name (name)}
								<li>{name}</li>
							{/each}
						</ul>
					</div>
				</div>
			</div>
		{/each}
	</div>
</section>

<style>
	.team-section {
		margin-bottom: 3rem;
	}

	.team-header {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		margin-bottom: 0.75rem;
	}

	:global(.team-header .section-icon) {
		width: 1.125rem;
		height: 1.125rem;
		color: var(--primary);
	}

	.section-title {
		font-size: 1.25rem;
		font-weight: 600;
		margin: 0;
	}

	.team-lead {
		font-size: 1rem;
		color: var(--muted-foreground);
		line-height: 1.7;
		max-width: 42rem;
		margin-bottom: 1.5rem;
	}

	.university-list {
		display: grid;
		gap: 1rem;
		grid-template-columns: 1fr;
	}

	@media (min-width: 640px) {
		.university-list {
			grid-template-columns: 1fr 1fr;
		}
	}

	.university-card {
		border: 1px solid var(--border);
		border-radius: var(--radius-md);
		background: var(--card);
		padding: 1.25rem;
	}

	.uni-header {
		display: flex;
		align-items: center;
		gap: 0.625rem;
		margin-bottom: 1rem;
		padding-bottom: 0.75rem;
		border-bottom: 1px solid var(--border);
	}

	:global(.uni-icon) {
		width: 1rem;
		height: 1rem;
		color: var(--muted-foreground);
		flex-shrink: 0;
	}

	.uni-name {
		font-size: 0.875rem;
		font-weight: 600;
		margin: 0;
		line-height: 1.3;
	}

	.uni-abbr {
		font-size: 0.75rem;
		color: var(--muted-foreground);
	}

	.uni-groups {
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}

	.group-label {
		font-size: 0.6875rem;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.05em;
		color: var(--muted-foreground);
		margin: 0 0 0.5rem;
	}

	.name-list {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-wrap: wrap;
		gap: 0.25rem 0.75rem;
	}

	.name-list li {
		font-size: 0.8125rem;
		color: var(--foreground);
		line-height: 1.6;
	}
</style>
