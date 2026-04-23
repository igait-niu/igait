<script lang="ts">
	import { Crosshair, Brain, Accessibility } from '@lucide/svelte';
</script>

<section class="mission-section">
	<div class="mission-header">
		<Crosshair class="section-icon" />
		<h2 class="section-title">Our Mission</h2>
	</div>
	<p class="mission-lead">
		iGait makes gait-based autism screening accessible to every family — regardless of location,
		income, or proximity to specialists.
	</p>

	<div class="mission-points">
		<div class="mission-point">
			<div class="point-icon-wrap">
				<Brain class="point-icon" />
			</div>
			<div>
				<h3 class="point-title">AI-Powered Analysis</h3>
				<p class="point-desc">
					Our pipeline uses computer vision and machine learning to detect subtle biomechanical
					markers in walking patterns that may indicate autism spectrum characteristics.
				</p>
			</div>
		</div>

		<div class="mission-point">
			<div class="point-icon-wrap">
				<Accessibility class="point-icon" />
			</div>
			<div>
				<h3 class="point-title">Accessible to All</h3>
				<p class="point-desc">
					Families record short walking videos at home with a smartphone. No travel to specialists,
					no long waitlists — just fast, preliminary screening results.
				</p>
			</div>
		</div>
	</div>
</section>

<style>
	.mission-section {
		margin-bottom: 3rem;
	}

	.mission-header {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		margin-bottom: 0.75rem;
	}

	:global(.section-icon) {
		width: 1.125rem;
		height: 1.125rem;
		color: var(--primary);
		flex-shrink: 0;
	}

	.section-title {
		font-size: 1.25rem;
		font-weight: 600;
		margin: 0;
	}

	.mission-lead {
		font-size: 1rem;
		color: var(--muted-foreground);
		line-height: 1.7;
		max-width: 42rem;
		margin-bottom: 1.5rem;
	}

	.mission-points {
		display: flex;
		flex-direction: column;
		gap: 1.25rem;
	}

	.mission-point {
		display: flex;
		gap: 1rem;
		align-items: flex-start;
		padding: 1rem 1.25rem;
		border: 1px solid var(--border);
		border-radius: var(--radius-md);
		background: oklch(from var(--muted) l c h / 0.4);
	}

	.point-icon-wrap {
		flex-shrink: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		width: 2.25rem;
		height: 2.25rem;
		border-radius: var(--radius-sm);
		background: oklch(from var(--primary) l c h / 0.08);
		margin-top: 0.125rem;
	}

	:global(.point-icon) {
		width: 1rem;
		height: 1rem;
		color: var(--primary);
	}

	.point-title {
		font-size: 0.9375rem;
		font-weight: 600;
		margin: 0 0 0.25rem;
	}

	.point-desc {
		font-size: 0.875rem;
		color: var(--muted-foreground);
		line-height: 1.6;
		margin: 0;
	}

	@media (min-width: 640px) {
		.mission-points {
			flex-direction: row;
		}

		.mission-point {
			flex: 1;
		}
	}
</style>
