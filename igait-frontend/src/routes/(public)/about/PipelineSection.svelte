<script lang="ts">
	import { Workflow, Video, ScanEye, Bone, ArrowRight } from '@lucide/svelte';
</script>

<section class="pipeline-section">
	<div class="pipeline-header">
		<Workflow class="section-icon" />
		<h2 class="section-title">How It Works</h2>
	</div>
	<p class="pipeline-lead">
		A short smartphone video goes through our multi-stage AI pipeline — from raw footage to
		screening results.
	</p>

	<div class="pipeline-stages">
		<div class="stage">
			<div class="stage-visual">
				<img src="/Videos/110.gif" alt="Raw video of child walking" />
			</div>
			<div class="stage-info">
				<div class="stage-label">
					<Video class="stage-label-icon" />
					<span class="stage-number">Step 1</span>
				</div>
				<h3 class="stage-name">Video Capture</h3>
				<p class="stage-desc">Record your child walking naturally with any smartphone camera</p>
			</div>
		</div>

		<div class="stage-arrow">
			<ArrowRight class="arrow-icon" />
		</div>

		<div class="stage">
			<div class="stage-visual">
				<img src="/Videos/9S.gif" alt="AI processing with tracking points" />
			</div>
			<div class="stage-info">
				<div class="stage-label">
					<ScanEye class="stage-label-icon" />
					<span class="stage-number">Step 2</span>
				</div>
				<h3 class="stage-name">Pose Estimation</h3>
				<p class="stage-desc">Computer vision tracks 30+ skeletal landmarks frame by frame</p>
			</div>
		</div>

		<div class="stage-arrow">
			<ArrowRight class="arrow-icon" />
		</div>

		<div class="stage">
			<div class="stage-visual">
				<img src="/Videos/114.gif" alt="Skeleton view of gait analysis" />
			</div>
			<div class="stage-info">
				<div class="stage-label">
					<Bone class="stage-label-icon" />
					<span class="stage-number">Step 3</span>
				</div>
				<h3 class="stage-name">Gait Analysis</h3>
				<p class="stage-desc">ML models analyze gait cycles to detect autism-associated patterns</p>
			</div>
		</div>
	</div>
</section>

<style>
	.pipeline-section {
		margin-bottom: 3rem;
	}

	.pipeline-header {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		margin-bottom: 0.75rem;
	}

	:global(.pipeline-header .section-icon) {
		width: 1.125rem;
		height: 1.125rem;
		color: var(--primary);
	}

	.section-title {
		font-size: 1.25rem;
		font-weight: 600;
		margin: 0;
	}

	.pipeline-lead {
		font-size: 1rem;
		color: var(--muted-foreground);
		line-height: 1.7;
		max-width: 42rem;
		margin-bottom: 1.5rem;
	}

	.pipeline-stages {
		display: flex;
		flex-direction: column;
		gap: 1rem;
		align-items: stretch;
	}

	@media (min-width: 768px) {
		.pipeline-stages {
			flex-direction: row;
			align-items: flex-start;
		}
	}

	.stage {
		flex: 1;
		border: 1px solid var(--border);
		border-radius: var(--radius-md);
		overflow: hidden;
		background: var(--card);
	}

	.stage-visual {
		background: var(--muted);
		overflow: hidden;
		height: 14rem;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.stage-visual img {
		width: 100%;
		height: 100%;
		object-fit: cover;
		display: block;
	}

	.stage-info {
		padding: 1rem 1.25rem;
	}

	.stage-label {
		display: flex;
		align-items: center;
		gap: 0.375rem;
		margin-bottom: 0.375rem;
	}

	:global(.stage-label-icon) {
		width: 0.8125rem;
		height: 0.8125rem;
		color: var(--primary);
	}

	.stage-number {
		font-size: 0.6875rem;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.05em;
		color: var(--primary);
	}

	.stage-name {
		font-size: 0.9375rem;
		font-weight: 600;
		margin: 0 0 0.25rem;
	}

	.stage-desc {
		font-size: 0.8125rem;
		color: var(--muted-foreground);
		line-height: 1.5;
		margin: 0;
	}

	.stage-arrow {
		display: none;
	}

	@media (min-width: 768px) {
		.stage-arrow {
			display: flex;
			align-items: center;
			justify-content: center;
			flex-shrink: 0;
			/* Center on the image area (14rem) plus top border */
			height: 14rem;
		}

		:global(.arrow-icon) {
			width: 1.25rem;
			height: 1.25rem;
			color: oklch(from var(--muted-foreground) l c h / 0.4);
		}
	}
</style>
