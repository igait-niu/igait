<script lang="ts">
	import { Button } from '$lib/components/ui/button';
	import { ArrowRight } from '@lucide/svelte';
</script>

<section class="hero-section">
	<div class="page-container">
		<div class="hero-content">
			<h1 class="hero-title">
				2 Years
				<span class="text-primary"> Too Late</span>
			</h1>
			<p class="hero-description">
				Children wait 27+ months for autism diagnosis after parents first notice signs. iGAIT
				screens in minutes, giving families answers when they need them most.
			</p>
			<div class="hero-actions">
				<Button size="lg" href="/signup">
					Screen Your Child Today
					<ArrowRight class="ml-2 h-4 w-4" />
				</Button>
				<Button size="lg" variant="outline" href="/about">Learn More</Button>
			</div>
		</div>
	</div>
</section>

<style>
	.hero-section {
		padding: var(--section-padding-y) 0;
		text-align: center;
	}

	.hero-content {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: var(--spacing-lg);
		max-width: 48rem;
		margin: 0 auto;
	}

	.hero-title {
		font-size: 3rem;
		font-weight: bold;
		letter-spacing: -0.025em;
		line-height: 1.2;
	}

	.hero-description {
		font-size: 1.25rem;
		color: hsl(var(--muted-foreground));
		max-width: 42rem;
	}

	.hero-actions {
		display: flex;
		gap: var(--spacing-md);
		flex-direction: column;
	}

	@media (min-width: 640px) {
		.hero-actions {
			flex-direction: row;
		}
	}

	@media (max-width: 768px) {
		.hero-title {
			font-size: 1.875rem;
		}

		.hero-description {
			font-size: 1rem;
		}
	}
</style>
