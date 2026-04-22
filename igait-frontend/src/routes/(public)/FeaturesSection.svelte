<script lang="ts">
	import * as Card from '$lib/components/ui/card';
	import {
		Clock,
		Target,
		Home,
		Lock,
		Users,
		FlaskConical,
		Video,
		Brain,
		BarChart3,
		Stethoscope,
		AlertCircle
	} from '@lucide/svelte';

	const statistics = [
		{
			number: '1 in 31',
			label: 'Children are autistic',
			description: "That's 3.2% of 8-year-olds in the United States, according to CDC's 2025 data."
		},
		{
			number: '27+',
			label: 'Months of waiting',
			description: 'Average time between initial screening and formal autism diagnosis.'
		},
		{
			number: '4-5',
			label: 'Years old at diagnosis',
			description:
				'By the time most children receive a diagnosis, critical early intervention windows have narrowed.'
		}
	];

	const features = [
		{
			icon: Clock,
			title: 'Fast Results',
			description:
				'Get preliminary screening results in minutes, not months. Upload videos from home and receive AI-powered analysis quickly.'
		},
		{
			icon: Target,
			title: 'Accurate Detection',
			description:
				'Advanced AI trained on thousands of gait patterns identifies subtle markers that might indicate autism spectrum characteristics.'
		},
		{
			icon: Home,
			title: 'From Home',
			description:
				'No need to travel to specialists. Record simple walking videos at home using your smartphone or tablet.'
		},
		{
			icon: Lock,
			title: 'Private & Secure',
			description:
				"Your data is encrypted and HIPAA-compliant. We take your family's privacy seriously with enterprise-grade security."
		},
		{
			icon: Users,
			title: 'For All Families',
			description:
				'Bridging the gap for rural areas, underserved communities, and families without easy access to diagnostic specialists.'
		},
		{
			icon: FlaskConical,
			title: 'Research-Backed',
			description:
				'Developed by Northern Illinois University and Southern Illinois University Edwardsville with years of peer-reviewed research.'
		}
	];

	const steps = [
		{
			icon: Video,
			title: 'Record Videos',
			description:
				'Use your smartphone to record two short videos: one of your child walking toward the camera, and one from the side.'
		},
		{
			icon: Brain,
			title: 'AI Analysis',
			description:
				'Our advanced AI analyzes gait patterns, identifying subtle biomechanical markers associated with autism spectrum characteristics.'
		},
		{
			icon: BarChart3,
			title: 'Get Results',
			description:
				'Receive a detailed screening report within minutes, helping you decide whether to pursue formal diagnostic evaluation.'
		},
		{
			icon: Stethoscope,
			title: 'Consult Professionals',
			description:
				'Share your results with healthcare providers to expedite the diagnostic process and access early intervention services.'
		}
	];
</script>

<!-- The Problem Section -->
<section class="problem-section">
	<div class="page-container">
		<h2 class="section-title">Understanding the Challenge</h2>
		<div class="stats-grid">
			{#each statistics as stat, i (i)}
				<Card.Root>
					<Card.Header>
						<div class="stat-number">{stat.number}</div>
						<Card.Title class="stat-label">{stat.label}</Card.Title>
						<Card.Description class="stat-description">
							{stat.description}
						</Card.Description>
					</Card.Header>
				</Card.Root>
			{/each}
		</div>

		<div class="timeline-section">
			<h3 class="subsection-title">The Wait Timeline</h3>
			<div class="timeline">
				<div class="timeline-item">
					<div class="timeline-marker"></div>
					<div class="timeline-content">
						<h4>The Wait Begins</h4>
						<p>
							Parents notice early signs, but accessing diagnostic services proves challenging due
							to long waitlists and limited specialists.
						</p>
					</div>
				</div>
				<div class="timeline-item">
					<div class="timeline-marker"></div>
					<div class="timeline-content">
						<h4>Rural & Underserved Families</h4>
						<p>
							Children from rural areas and under-resourced families wait even longer—often closer
							to 8 years for a diagnosis.
						</p>
					</div>
				</div>
				<div class="timeline-item">
					<div class="timeline-marker"></div>
					<div class="timeline-content">
						<h4>Missed Opportunities</h4>
						<p>
							Every month of delay means missed opportunities for early intervention during the most
							critical developmental periods.
						</p>
					</div>
				</div>
			</div>
		</div>
	</div>
</section>

<!-- The Solution Section -->
<section class="solution-section">
	<div class="page-container">
		<h2 class="section-title">The iGAIT Solution</h2>
		<div class="features-grid">
			{#each features as feature, i (i)}
				<Card.Root>
					<Card.Header>
						<div class="feature-icon">
							<svelte:component this={feature.icon} class="h-6 w-6 text-primary" />
						</div>
						<Card.Title>{feature.title}</Card.Title>
						<Card.Description>
							{feature.description}
						</Card.Description>
					</Card.Header>
				</Card.Root>
			{/each}
		</div>
	</div>
</section>

<!-- How It Works Section -->
<section class="how-it-works-section">
	<div class="page-container">
		<h2 class="section-title">How It Works</h2>
		<div class="steps-grid">
			{#each steps as step, index (index)}
				<div class="step-card">
					<div class="step-number">{index + 1}</div>
					<div class="step-icon">
						<svelte:component this={step.icon} class="h-8 w-8 text-primary" />
					</div>
					<h3 class="step-title">{step.title}</h3>
					<p class="step-description">{step.description}</p>
				</div>
			{/each}
		</div>
	</div>
</section>

<!-- Research Video Section -->
<section class="research-video-section">
	<div class="page-container">
		<h2 class="section-title">See How It Works</h2>
		<p class="section-description">Learn how to record your child's walking gait for analysis</p>
		<div class="video-container">
			<iframe
				src="https://mediaspace.niu.edu/embed/secure/iframe/entryId/1_z7bejbcu/uiConfId/45549541/st/0"
				title="How to record your child's walking gait"
				allowfullscreen
				class="research-video"
			></iframe>
		</div>
	</div>
</section>

<!-- Behind the Scenes Section -->
<section class="behind-scenes-section">
	<div class="page-container">
		<h2 class="section-title">Behind the Scenes</h2>
		<p class="section-description">Watch how our AI transforms raw video into insights</p>
		<div class="pipeline-grid">
			<div class="pipeline-stage">
				<div class="stage-number">01</div>
				<div class="stage-visual">
					<img src="/Videos/110.gif" alt="Raw video of child walking" class="stage-image" />
				</div>
				<h3 class="stage-title">Raw Video</h3>
				<p class="stage-description">
					Your smartphone captures natural walking patterns in everyday settings
				</p>
			</div>
			<div class="pipeline-arrow">→</div>
			<div class="pipeline-stage">
				<div class="stage-number">02</div>
				<div class="stage-visual">
					<img
						src="/Videos/9S.gif"
						alt="AI processing video with tracking points"
						class="stage-image"
					/>
				</div>
				<h3 class="stage-title">AI Processing</h3>
				<p class="stage-description">
					Advanced computer vision tracks movement and identifies key biomechanical points
				</p>
			</div>
			<div class="pipeline-arrow">→</div>
			<div class="pipeline-stage">
				<div class="stage-number">03</div>
				<div class="stage-visual">
					<img
						src="/Videos/114.gif"
						alt="Skeleton view showing gait analysis"
						class="stage-image"
					/>
				</div>
				<h3 class="stage-title">Gait Analysis</h3>
				<p class="stage-description">
					Machine learning analyzes skeletal patterns to detect subtle autism indicators
				</p>
			</div>
		</div>
		<div class="tech-highlights">
			<div class="tech-stat">
				<strong>Real-time Processing</strong>
			</div>
			<div class="tech-stat">
				<strong>30+ Data Points</strong>
			</div>
			<div class="tech-stat">
				<strong>Research-Validated</strong>
			</div>
		</div>
	</div>
</section>

<!-- Why This Matters Section -->
<section class="significance-section">
	<div class="page-container">
		<h2 class="section-title">Why This Matters</h2>
		<div class="significance-content">
			<p class="significance-text">
				The US is currently experiencing a <strong>public health crisis</strong> regarding access to timely
				autism diagnosis.
			</p>

			<p class="significance-text">
				Delayed diagnosis prevents children from receiving <strong>early intervention</strong> at the
				crucial time of their optimal developmental impact, leading to reduced quality of life and increased
				costs at individual, family, and societal levels.
			</p>

			<Card.Root class="barriers-card">
				<Card.Header>
					<div class="barriers-icon">
						<AlertCircle class="h-6 w-6" />
					</div>
					<Card.Title>Major Barriers</Card.Title>
					<Card.Description>
						<ul class="barriers-list">
							<li>Shortage of qualified professionals, especially in rural areas</li>
							<li>Unaffordable diagnostic services for many families</li>
							<li>Current practices rely on subjective observations</li>
							<li>Existing tools have demographic and language limitations</li>
						</ul>
					</Card.Description>
				</Card.Header>
			</Card.Root>

			<p class="significance-text call-to-action-text">
				There is an <strong>urgent need</strong> for technologically innovative, widely accessible, low-cost,
				objective approaches to autism detection.
			</p>
		</div>
	</div>
</section>

<style>
	/* Section base styles */
	section {
		padding: 4rem 0;
	}

	.page-container {
		max-width: 1200px;
		margin: 0 auto;
		padding: 0 1.5rem;
	}

	.section-title {
		font-size: 2.5rem;
		font-weight: 700;
		text-align: center;
		margin-bottom: 1rem;
	}

	.section-description {
		text-align: center;
		font-size: 1.125rem;
		color: hsl(var(--muted-foreground));
		margin-bottom: 3rem;
		max-width: 42rem;
		margin-left: auto;
		margin-right: auto;
	}

	.subsection-title {
		font-size: 1.75rem;
		font-weight: 600;
		text-align: center;
		margin-top: 4rem;
		margin-bottom: 2rem;
	}

	/* Problem Section */
	.problem-section {
		background-color: hsl(var(--background));
	}

	.stats-grid {
		display: grid;
		gap: 1.5rem;
		grid-template-columns: repeat(auto-fit, minmax(280px, 1fr));
		margin-bottom: 4rem;
	}

	.stat-number {
		font-size: 3rem;
		font-weight: 700;
		color: hsl(var(--primary));
		margin-bottom: 0.5rem;
		line-height: 1;
	}

	:global(.stats-grid .stat-label) {
		font-size: 1.125rem;
		font-weight: 600;
		margin-bottom: 0.5rem;
	}

	:global(.stats-grid .stat-description) {
		line-height: 1.6;
		color: hsl(var(--muted-foreground));
	}

	/* Timeline */
	.timeline-section {
		max-width: 48rem;
		margin: 0 auto;
	}

	.timeline {
		position: relative;
		padding-left: 2rem;
		border-left: 2px solid hsl(var(--primary) / 0.3);
	}

	.timeline-item {
		position: relative;
		padding-bottom: 2.5rem;
	}

	.timeline-item:last-child {
		padding-bottom: 0;
	}

	.timeline-marker {
		position: absolute;
		left: -2.5rem;
		top: 0.25rem;
		width: 1rem;
		height: 1rem;
		border-radius: 50%;
		background-color: hsl(var(--primary));
		border: 3px solid hsl(var(--background));
	}

	.timeline-content h4 {
		font-size: 1.125rem;
		font-weight: 600;
		margin-bottom: 0.5rem;
	}

	.timeline-content p {
		color: hsl(var(--muted-foreground));
		line-height: 1.6;
	}

	/* Solution Section */
	.solution-section {
		background-color: hsl(var(--muted) / 0.3);
	}

	.features-grid {
		display: grid;
		gap: 1.5rem;
		grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
	}

	.feature-icon {
		display: inline-flex;
		height: 3rem;
		width: 3rem;
		align-items: center;
		justify-content: center;
		border-radius: 0.5rem;
		background-color: hsl(var(--primary) / 0.1);
		margin-bottom: 1rem;
	}

	/* How It Works Section */
	.how-it-works-section {
		background-color: hsl(var(--background));
	}

	.steps-grid {
		display: grid;
		gap: 2rem;
		grid-template-columns: repeat(auto-fit, minmax(250px, 1fr));
	}

	.step-card {
		text-align: center;
		padding: 2rem 1.5rem;
		border-radius: 0.75rem;
		background-color: hsl(var(--card));
		border: 1px solid hsl(var(--border));
		transition: all 0.2s ease;
	}

	.step-card:hover {
		transform: translateY(-4px);
		box-shadow: 0 12px 24px -8px hsl(var(--foreground) / 0.12);
		border-color: hsl(var(--primary) / 0.3);
	}

	.step-number {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: 2.5rem;
		height: 2.5rem;
		border-radius: 50%;
		background-color: hsl(var(--primary));
		color: hsl(var(--primary-foreground));
		font-weight: 700;
		font-size: 1.25rem;
		margin-bottom: 1rem;
	}

	.step-icon {
		margin: 1rem auto;
		display: flex;
		justify-content: center;
		color: hsl(var(--primary));
	}

	.step-title {
		font-size: 1.25rem;
		font-weight: 600;
		margin-bottom: 0.75rem;
	}

	.step-description {
		color: hsl(var(--muted-foreground));
		line-height: 1.6;
	}

	/* Research Video Section */
	.research-video-section {
		background-color: hsl(var(--muted) / 0.3);
	}

	.video-container {
		position: relative;
		width: 100%;
		max-width: 56rem;
		margin: 0 auto;
		padding-bottom: 56.25%; /* 16:9 aspect ratio */
		border-radius: 0.75rem;
		overflow: hidden;
		box-shadow: 0 10px 40px -10px hsl(var(--foreground) / 0.15);
	}

	.research-video {
		position: absolute;
		top: 0;
		left: 0;
		width: 100%;
		height: 100%;
		border: none;
	}

	/* Behind the Scenes Section */
	.behind-scenes-section {
		background-color: hsl(var(--background));
	}

	.pipeline-grid {
		display: grid;
		gap: 0;
		grid-template-columns: 1fr auto 1fr auto 1fr;
		align-items: center;
		max-width: 64rem;
		margin: 0 auto 3rem;
	}

	.pipeline-stage {
		text-align: center;
		padding: 1rem;
	}

	.stage-number {
		font-size: 0.875rem;
		font-weight: 600;
		color: hsl(var(--primary));
		margin-bottom: 0.75rem;
		text-transform: uppercase;
		letter-spacing: 0.05em;
	}

	.stage-visual {
		margin-bottom: 1rem;
		border-radius: 0.75rem;
		overflow: hidden;
		background-color: hsl(var(--muted));
		box-shadow: 0 4px 12px -4px hsl(var(--foreground) / 0.1);
	}

	.stage-image {
		width: 100%;
		height: auto;
		display: block;
	}

	.stage-title {
		font-size: 1.125rem;
		font-weight: 600;
		margin-bottom: 0.5rem;
	}

	.stage-description {
		color: hsl(var(--muted-foreground));
		font-size: 0.875rem;
		line-height: 1.6;
	}

	.pipeline-arrow {
		font-size: 1.5rem;
		color: hsl(var(--primary) / 0.4);
		text-align: center;
		font-weight: 300;
		padding: 0 0.5rem;
	}

	.tech-highlights {
		display: flex;
		justify-content: center;
		gap: 3rem;
		flex-wrap: wrap;
	}

	.tech-stat {
		text-align: center;
		font-size: 1rem;
		color: hsl(var(--muted-foreground));
	}

	.tech-stat strong {
		color: hsl(var(--foreground));
	}

	/* Significance Section */
	.significance-section {
		background-color: hsl(var(--muted) / 0.3);
	}

	.significance-content {
		max-width: 48rem;
		margin: 0 auto;
		display: flex;
		flex-direction: column;
		gap: 1.5rem;
	}

	.significance-text {
		font-size: 1.125rem;
		line-height: 1.8;
		text-align: center;
	}

	:global(.barriers-card) {
		margin: 2rem 0;
		border: 2px solid hsl(var(--destructive) / 0.2);
		box-shadow: 0 8px 24px -8px hsl(var(--destructive) / 0.15);
		background: linear-gradient(to bottom, hsl(var(--card)), hsl(var(--muted) / 0.2));
	}

	.barriers-icon {
		display: inline-flex;
		height: 3.5rem;
		width: 3.5rem;
		align-items: center;
		justify-content: center;
		border-radius: 0.75rem;
		background: linear-gradient(
			135deg,
			hsl(var(--destructive) / 0.15),
			hsl(var(--destructive) / 0.05)
		);
		color: hsl(var(--destructive));
		margin-bottom: 1rem;
		border: 1px solid hsl(var(--destructive) / 0.2);
	}

	:global(.barriers-card .card-header) {
		padding: 2rem;
	}

	:global(.barriers-card .card-title) {
		font-size: 1.5rem;
		font-weight: 600;
		color: hsl(var(--foreground));
		margin-bottom: 0.5rem;
	}

	:global(.barriers-card .card-description) {
		color: hsl(var(--foreground));
		font-size: 1rem;
	}

	.barriers-list {
		margin-top: 1rem;
		padding-left: 1.5rem;
		text-align: left;
		line-height: 1.9;
	}

	.barriers-list li {
		margin-bottom: 0.75rem;
		color: hsl(var(--foreground) / 0.9);
	}

	.call-to-action-text {
		font-weight: 500;
		color: hsl(var(--foreground));
	}

	/* Responsive adjustments */
	@media (max-width: 768px) {
		section {
			padding: 3rem 0;
		}

		.page-container {
			padding: 0 1rem;
		}

		.section-title {
			font-size: 2rem;
		}

		.section-description,
		.significance-text {
			font-size: 1rem;
		}

		.subsection-title {
			font-size: 1.5rem;
		}

		.stat-number {
			font-size: 2.5rem;
		}

		.pipeline-arrow {
			display: none;
		}

		.pipeline-grid {
			grid-template-columns: 1fr;
			gap: 2rem;
		}

		.tech-highlights {
			gap: 2rem;
		}

		.features-grid,
		.stats-grid,
		.steps-grid {
			grid-template-columns: 1fr;
		}
	}
</style>
