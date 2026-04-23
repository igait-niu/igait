<script lang="ts">
	import MissionSection from './MissionSection.svelte';
	import PipelineSection from './PipelineSection.svelte';
	import TeamSection from './TeamSection.svelte';
	import SponsorsSection from './SponsorsSection.svelte';
</script>

<svelte:head>
	<title>About - iGait</title>
	<meta
		name="description"
		content="Learn about the iGait research team advancing early autism detection through innovative gait analysis technology."
	/>
</svelte:head>

<div class="about-page">
	<div class="page-container">
		<header class="page-header">
			<p class="page-label">About Us</p>
			<h1 class="page-title">About iGait</h1>
			<p class="page-subtitle">
				Advancing early autism detection through innovative technology and collaborative research
			</p>
		</header>

		<MissionSection />
		<PipelineSection />
		<TeamSection />
		<SponsorsSection />
	</div>
</div>

<style>
	.about-page {
		padding: 3rem 0 4rem;
	}

	.page-header {
		text-align: center;
		margin-bottom: 3.5rem;
	}

	.page-label {
		font-size: 0.8125rem;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.05em;
		color: var(--primary);
		margin-bottom: 0.5rem;
	}

	.page-title {
		font-size: 2.25rem;
		font-weight: 700;
		line-height: 1.1;
		letter-spacing: -0.025em;
	}

	@media (min-width: 768px) {
		.page-title {
			font-size: 3rem;
		}
	}

	.page-subtitle {
		margin-top: 0.75rem;
		max-width: 36rem;
		margin-left: auto;
		margin-right: auto;
		font-size: 1.0625rem;
		color: var(--muted-foreground);
		line-height: 1.6;
	}
</style>
