<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import * as Card from '$lib/components/ui/card';
	import { ArrowRight } from '@lucide/svelte';
	import { authStore } from '$lib/stores/auth.svelte';

	const authState = $derived(authStore.state);
	const isLoggedIn = $derived(authState.status === 'authenticated');

	const teamMembers = [
		{
			name: 'Sinan Onal, PhD',
			photo: '/Images/Team%20Photos/sonal.jpg',
			url: 'http://www.sinanonal.com/'
		},
		{
			name: 'Ziteng Wang, PhD',
			photo: '/Images/Team%20Photos/wang.jpg',
			url: 'https://www.wang-zt.com/'
		},
		{
			name: 'Allison Gladfelter, PhD, CCC-SLP',
			photo: '/Images/Team%20Photos/gladfelter.jpg',
			url: 'https://www.chhs.niu.edu/about/staff/gladfelter.shtml'
		},
		{
			name: 'Milijana Buac, PhD, CCC-SLP',
			photo: '/Images/Team%20Photos/buac.jpg',
			url: 'https://www.chhs.niu.edu/about/staff/buac.shtml'
		}
	];
</script>

<!-- Team Section -->
<section class="team-section">
	<div class="page-container">
		<h2 class="section-title">Meet the Team</h2>
		<div class="team-grid">
			{#each teamMembers as member, i (i)}
				<Card.Root class="team-card">
					<Card.Header>
						<div class="member-photo-wrapper">
							<img src={member.photo} alt={member.name} class="member-photo" />
						</div>
						<Card.Title class="member-name">
							<a href={member.url} target="_blank" rel="noopener noreferrer">
								{member.name}
							</a>
						</Card.Title>
					</Card.Header>
				</Card.Root>
			{/each}
		</div>
		<p class="team-acknowledgment">
			This research would be impossible without the talent and endeavors of the
			<a href="/about#student-team" class="link-accent">student team</a>. We thank their
			contributions and hard work!
		</p>
	</div>
</section>

<!-- CTA Section -->
<section class="cta-section">
	<div class="page-container">
		<div class="cta-card">
			<h2 class="cta-title">Ready to Get Started?</h2>
			<p class="cta-description">
				{#if isLoggedIn}
					Start screening your child today with our AI-powered gait analysis tool.
				{:else}
					Join thousands of families using iGAIT for early autism screening. Fast, free, and
					accessible from home.
				{/if}
			</p>
			<div class="cta-buttons">
				{#if isLoggedIn}
					<Button size="lg" href="/home">
						Try iGAIT Now
						<ArrowRight class="ml-2 h-4 w-4" />
					</Button>
					<Button size="lg" variant="outline" href="/about">Learn More</Button>
				{:else}
					<Button size="lg" href="/signup">
						Sign Up Today
						<ArrowRight class="ml-2 h-4 w-4" />
					</Button>
					<Button size="lg" variant="outline" href="/login">Log In</Button>
				{/if}
			</div>
		</div>
	</div>
</section>

<style>
	/* Team Section */
	.team-section {
		padding: var(--section-padding-y) 0;
		background-color: hsl(var(--muted) / 0.3);
	}

	.section-title {
		font-size: 2.25rem;
		font-weight: bold;
		text-align: center;
		margin-bottom: var(--spacing-xl);
	}

	.team-grid {
		display: grid;
		gap: var(--grid-gap);
		grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
		margin-bottom: var(--spacing-xl);
	}

	:global(.team-card) {
		text-align: center;
		transition:
			transform 0.2s,
			box-shadow 0.2s;
	}

	:global(.team-card:hover) {
		transform: translateY(-4px);
		box-shadow: 0 10px 25px -5px hsl(var(--foreground) / 0.1);
	}

	.member-photo-wrapper {
		display: flex;
		justify-content: center;
		margin-bottom: var(--spacing-md);
	}

	.member-photo {
		width: 10rem;
		height: 10rem;
		border-radius: 50%;
		object-fit: cover;
		border: 4px solid hsl(var(--primary) / 0.1);
	}

	:global(.team-card .member-name a) {
		color: hsl(var(--foreground));
		text-decoration: none;
		transition: color 0.2s;
	}

	:global(.team-card .member-name a:hover) {
		color: hsl(var(--primary));
	}

	.team-acknowledgment {
		text-align: center;
		font-size: 1.125rem;
		color: hsl(var(--muted-foreground));
		max-width: 42rem;
		margin: 0 auto;
		line-height: 1.6;
	}

	.link-accent {
		color: hsl(var(--primary));
		text-decoration: none;
		transition: opacity 0.2s;
	}

	.link-accent:hover {
		opacity: 0.8;
	}

	/* CTA Section */
	.cta-section {
		padding: var(--section-padding-y) 0;
	}

	.cta-card {
		background: linear-gradient(135deg, hsl(var(--primary) / 0.1), hsl(var(--primary) / 0.05));
		border-radius: var(--radius-lg);
		padding: var(--spacing-2xl);
		text-align: center;
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: var(--spacing-md);
		transition:
			transform 0.2s,
			box-shadow 0.2s;
	}

	.cta-card:hover {
		transform: translateY(-2px);
		box-shadow: 0 10px 25px -5px hsl(var(--primary) / 0.2);
	}

	.cta-title {
		font-size: 2rem;
		font-weight: bold;
	}

	.cta-description {
		font-size: 1.125rem;
		color: hsl(var(--muted-foreground));
		max-width: 36rem;
		line-height: 1.6;
	}

	.cta-buttons {
		display: flex;
		gap: var(--spacing-md);
		margin-top: var(--spacing-sm);
		flex-wrap: wrap;
		justify-content: center;
	}

	/* Responsive adjustments */
	@media (max-width: 768px) {
		.section-title {
			font-size: 1.875rem;
		}

		.cta-card {
			padding: var(--spacing-xl);
		}

		.cta-title {
			font-size: 1.5rem;
		}

		.cta-description,
		.team-acknowledgment {
			font-size: 1rem;
		}

		.member-photo {
			width: 8rem;
			height: 8rem;
		}
	}
</style>
