<script lang="ts">
	import { Separator } from '$lib/components/ui/separator';
</script>

<!--
	Shared footer component with legal links
	Used across all layouts to ensure legal pages are always accessible
-->

<footer class="border-t bg-background">
	<div class="mx-auto max-w-7xl px-4 py-8">
		<div class="flex flex-col gap-6">
			<!-- Medical Disclaimer -->
			<div class="disclaimer-box">
				<h3 class="disclaimer-title">Important Disclaimer</h3>
				<p class="disclaimer-text">
					iGAIT is an AI-based screening tool designed to assist in early autism detection. Results
					must not be interpreted as a diagnosis and should not substitute professional medical
					advice. Always consult your physician or qualified healthcare professional for
					comprehensive evaluation and guidance.
				</p>
			</div>

			<Separator />

			<!-- Main footer content -->
			<div class="flex flex-col items-center justify-between gap-4 sm:flex-row">
				<p class="text-xs text-muted-foreground">
					© {new Date().getFullYear()} Northern Illinois University & Southern Illinois University Edwardsville.
					All rights reserved.
				</p>

				<!-- Navigation links -->
				<nav class="flex flex-wrap items-center justify-center gap-x-6 gap-y-2">
					<a
						href="/about"
						class="text-sm text-muted-foreground transition-colors hover:text-foreground"
					>
						About
					</a>
					<a
						href="/terms"
						class="text-sm text-muted-foreground transition-colors hover:text-foreground"
					>
						Terms of Service
					</a>
					<a
						href="/policy"
						class="text-sm text-muted-foreground transition-colors hover:text-foreground"
					>
						Privacy Policy
					</a>
					<a
						href="/hipaa"
						class="text-sm text-muted-foreground transition-colors hover:text-foreground"
					>
						HIPAA Compliance
					</a>
					<a
						href="mailto:GaitStudy@niu.edu"
						class="text-sm text-muted-foreground transition-colors hover:text-foreground"
					>
						Contact Us
					</a>
				</nav>
			</div>
		</div>
	</div>
</footer>

<style>
	.disclaimer-box {
		background-color: hsl(var(--muted) / 0.5);
		border: 1px solid hsl(var(--border));
		border-radius: var(--radius-lg);
		padding: var(--spacing-lg);
		text-align: center;
	}

	.disclaimer-title {
		font-size: 1.125rem;
		font-weight: 600;
		margin-bottom: var(--spacing-sm);
		color: hsl(var(--foreground));
	}

	.disclaimer-text {
		font-size: 0.875rem;
		line-height: 1.6;
		color: hsl(var(--muted-foreground));
		max-width: 56rem;
		margin: 0 auto;
	}
</style>
