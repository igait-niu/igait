<script lang="ts">
	import * as Card from '$lib/components/ui/card';
	import { Badge } from '$lib/components/ui/badge';
	import { Separator } from '$lib/components/ui/separator';
	import { GraduationCap, Building2 } from '@lucide/svelte';

	const universities = [
		{
			name: 'Northern Illinois University (NIU)',
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
			name: 'Southern Illinois University Edwardsville (SIUE)',
			graduates: ['Amit Shrestha', 'Prashanna Pandit'],
			undergrads: ['Cody Schaefer', 'Vinhhy Pham']
		}
	];
</script>

<section class="content-section">
	<div class="section-header">
		<div class="section-icon">
			<GraduationCap class="h-5 w-5 text-primary" />
		</div>
		<h2 class="section-title">Student Research Team</h2>
	</div>

	<div class="team-grid">
		{#each universities as uni}
			<Card.Root>
				<Card.Header>
					<Card.Title class="university-title">
						<Building2 class="h-5 w-5 text-muted-foreground" />
						{uni.name}
					</Card.Title>
				</Card.Header>
				<Card.Content class="team-content">
					<div>
						<h4 class="team-category">Graduate Students</h4>
						<div class="team-list">
							{#each uni.graduates as name}
								<Badge variant="secondary">{name}</Badge>
							{/each}
						</div>
					</div>
					<Separator />
					<div>
						<h4 class="team-category">Undergraduate Students</h4>
						<div class="team-list">
							{#each uni.undergrads as name}
								<Badge variant="outline">{name}</Badge>
							{/each}
						</div>
					</div>
				</Card.Content>
			</Card.Root>
		{/each}
	</div>
</section>

<style>
	.content-section {
		margin-bottom: var(--spacing-xl);
	}

	.section-header {
		display: flex;
		align-items: center;
		gap: var(--spacing-xs);
		margin-bottom: var(--spacing-lg);
	}

	.section-icon {
		display: inline-flex;
		height: 2.5rem;
		width: 2.5rem;
		align-items: center;
		justify-content: center;
		border-radius: var(--radius-md);
		background-color: hsl(var(--primary) / 0.1);
	}

	.section-title {
		font-size: 1.875rem;
		font-weight: 600;
	}

	.team-grid {
		display: grid;
		gap: var(--grid-gap);
		grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
	}

	:global(.university-title) {
		display: flex;
		align-items: center;
		gap: var(--spacing-xs);
	}

	:global(.team-content) {
		display: flex;
		flex-direction: column;
		gap: var(--spacing-md);
	}

	.team-category {
		font-weight: 600;
		font-size: 0.875rem;
		color: hsl(var(--muted-foreground));
		text-transform: uppercase;
		letter-spacing: 0.05em;
		margin-bottom: var(--spacing-sm);
	}

	.team-list {
		display: flex;
		flex-wrap: wrap;
		gap: var(--spacing-xs);
	}
</style>
