<script lang="ts">
	import * as Card from '$lib/components/ui/card';
	import * as Accordion from '$lib/components/ui/accordion';
	import { Badge } from '$lib/components/ui/badge';
	import { Separator } from '$lib/components/ui/separator';
	import { Lock, Eye, Share2, UserCheck, Clock, Mail } from '@lucide/svelte';
	import type { Component } from 'svelte';

	interface Props {
		pageTitle: string;
		metaDescription: string;
		noticeTitle: string;
		cardTitle: string;
		cardDescription: string;
		icon: Component;
	}

	let {
		pageTitle,
		metaDescription,
		noticeTitle,
		cardTitle,
		cardDescription,
		icon: Icon
	}: Props = $props();
</script>

<svelte:head>
	<title>{pageTitle} - iGait</title>
	<meta name="description" content={metaDescription} />
</svelte:head>

<div class="legal-page">
	<div class="legal-container">
		<!-- Header -->
		<div class="legal-header">
			<Badge variant="secondary" class="header-badge">Legal</Badge>
			<h1 class="legal-title">{noticeTitle}</h1>
			<p class="legal-subtitle">Protecting Your Personal and Health Information</p>
		</div>

		<Card.Root>
			<Card.Header>
				<div class="legal-card-header">
					<div class="legal-icon">
						<Icon class="h-6 w-6 text-primary" />
					</div>
					<div>
						<Card.Title class="text-2xl">{cardTitle}</Card.Title>
						<Card.Description>{cardDescription}</Card.Description>
					</div>
				</div>
			</Card.Header>
			<Card.Content class="space-y-8">
				<!-- Introduction -->
				<div class="legal-intro">
					<p class="legal-text">
						Northern Illinois University is committed to protecting the privacy of patient personal
						and health information. Applicable Federal and State laws require us to maintain the
						privacy of your personal and health information. This Notice explains our privacy
						practices, our legal duties, and your rights concerning your personal and health
						information.
					</p>
					<p class="legal-text">
						In this Notice, your personal or protected health information (PHI) is referred to as
						"health information" and includes information regarding your health care and treatment
						with identifiable factors such as your name, age, address, income, or other financial
						information.
					</p>
				</div>

				<Separator />

				<!-- How We Protect Your Information -->
				<section>
					<div class="mb-4 flex items-center gap-2">
						<Lock class="h-5 w-5 text-primary" />
						<h2 class="text-xl font-semibold">How We Protect Your Health Information</h2>
					</div>
					<ul class="space-y-3">
						{#each ['Treating all of your health information that we collect as confidential', 'Stating confidentiality policies and practices in our policies, as well as disciplinary measures for privacy violations', 'Restricting access to your health information only to those who need to know your health information', 'Only disclosing your health information that is necessary and ensuring all those receiving information have agreed to protect and maintain confidentiality', 'Maintaining physical, electronic, and procedural safeguards to comply with federal and state regulations'] as item (item)}
							<li class="flex items-start gap-3">
								<div class="mt-1.5 h-2 w-2 flex-shrink-0 rounded-full bg-primary"></div>
								<span class="text-muted-foreground">{item}</span>
							</li>
						{/each}
					</ul>
				</section>

				<Separator />

				<!-- Uses and Disclosures -->
				<section>
					<div class="mb-4 flex items-center gap-2">
						<Eye class="h-5 w-5 text-primary" />
						<h2 class="text-xl font-semibold">Uses and Disclosures</h2>
					</div>

					<Accordion.Root type="multiple" class="w-full">
						<Accordion.Item value="definitions">
							<Accordion.Trigger>Key Definitions</Accordion.Trigger>
							<Accordion.Content>
								<div class="space-y-4 pt-2">
									<div class="rounded-lg border p-4">
										<h4 class="mb-2 font-semibold">Use</h4>
										<p class="text-sm text-muted-foreground">
											Applies only to activities within our organization such as sharing, employing,
											applying, utilizing, examining, and analyzing information that identifies you.
										</p>
									</div>
									<div class="rounded-lg border p-4">
										<h4 class="mb-2 font-semibold">Disclosure</h4>
										<p class="text-sm text-muted-foreground">
											Applies to activities outside of our organization, such as releasing,
											transferring, or providing access to information about you to other parties.
										</p>
									</div>
								</div>
							</Accordion.Content>
						</Accordion.Item>

						<Accordion.Item value="authorization">
							<Accordion.Trigger>Uses Requiring Authorization</Accordion.Trigger>
							<Accordion.Content>
								<div class="space-y-4 pt-2 text-muted-foreground">
									<p>
										We may use or disclose PHI for purposes outside treatment, payment, or
										healthcare operations when your appropriate authorization is obtained. An
										"authorization" is written permission beyond the general consent that permits
										only specific disclosures.
									</p>
									<p>
										You may revoke all such authorizations at any time, provided each revocation is
										in writing. You may not revoke an authorization to the extent that (1) we relied
										on that authorization; or (2) if the authorization was obtained as a condition
										of obtaining insurance coverage, law provides the insurer the right to contest
										the claim under the policy.
									</p>
								</div>
							</Accordion.Content>
						</Accordion.Item>
					</Accordion.Root>
				</section>

				<Separator />

				<!-- Your Rights -->
				<section>
					<div class="mb-4 flex items-center gap-2">
						<UserCheck class="h-5 w-5 text-primary" />
						<h2 class="text-xl font-semibold">Your Rights</h2>
					</div>

					<Accordion.Root type="multiple" class="w-full">
						<Accordion.Item value="restrictions">
							<Accordion.Trigger>Right to Request Restrictions</Accordion.Trigger>
							<Accordion.Content>
								<p class="pt-2 text-muted-foreground">
									You have the right to request additional restrictions on certain uses and
									disclosures of protected health information. We may not be able to accept your
									request, but if we do, we will uphold the restriction unless it is an emergency.
								</p>
							</Accordion.Content>
						</Accordion.Item>

						<Accordion.Item value="communications">
							<Accordion.Trigger>Right to Confidential Communications</Accordion.Trigger>
							<Accordion.Content>
								<p class="pt-2 text-muted-foreground">
									You have the right to request and receive confidential communications of PHI by
									alternative means and at alternative locations.
								</p>
							</Accordion.Content>
						</Accordion.Item>

						<Accordion.Item value="inspect">
							<Accordion.Trigger>Right to Inspect and Copy</Accordion.Trigger>
							<Accordion.Content>
								<p class="pt-2 text-muted-foreground">
									You have the right to inspect or obtain a copy (or both) of your health records. A
									reasonable fee may be charged for copying or, if necessary, redacting the record.
									Access to your records may be limited or denied under certain circumstances, but
									in most cases, you have a right to request a review of that decision.
								</p>
							</Accordion.Content>
						</Accordion.Item>

						<Accordion.Item value="amend">
							<Accordion.Trigger>Right to Amend</Accordion.Trigger>
							<Accordion.Content>
								<p class="pt-2 text-muted-foreground">
									You have the right to request in writing an amendment of your health information
									for as long as PHI records are maintained. The request must identify which
									information is incorrect and include an explanation of why you think it should be
									amended. If the request is denied, a written explanation stating why will be
									provided to you. Amending a record does not mean that any portion of your health
									information will be deleted.
								</p>
							</Accordion.Content>
						</Accordion.Item>

						<Accordion.Item value="accounting">
							<Accordion.Trigger>Right to an Accounting</Accordion.Trigger>
							<Accordion.Content>
								<p class="pt-2 text-muted-foreground">
									You generally have the right to receive an accounting of disclosures of PHI. If
									your health information is disclosed for any reason other than treatment, program
									evaluation, payment, or operation, you have the right to an accounting of each
									disclosure for the previous six (6) years. If more than one (1) accounting is
									requested in a twelve (12) month period, a reasonable fee may be charged.
								</p>
							</Accordion.Content>
						</Accordion.Item>

						<Accordion.Item value="paper">
							<Accordion.Trigger>Right to Paper Copy</Accordion.Trigger>
							<Accordion.Content>
								<p class="pt-2 text-muted-foreground">
									If you receive this notice electronically (e.g., accessing a website), you have
									the right to obtain a paper copy of the notice upon request.
								</p>
							</Accordion.Content>
						</Accordion.Item>
					</Accordion.Root>
				</section>

				<Separator />

				<!-- Our Duties -->
				<section>
					<div class="mb-4 flex items-center gap-2">
						<Share2 class="h-5 w-5 text-primary" />
						<h2 class="text-xl font-semibold">Our Duties</h2>
					</div>
					<p class="text-muted-foreground">
						We are required by law to maintain the privacy of PHI and to provide you with a notice
						of legal duties and privacy practices. Northern Illinois University reserves the right
						to change the privacy policies and practices described in this notice. Unless we notify
						you of such changes, however, we are required to abide by the terms currently in effect.
					</p>
				</section>

				<Separator />

				<!-- Changes to Notice -->
				<section>
					<div class="mb-4 flex items-center gap-2">
						<Clock class="h-5 w-5 text-primary" />
						<h2 class="text-xl font-semibold">Changes to this Notice</h2>
					</div>
					<p class="text-muted-foreground">
						The University reserves the right to change our privacy practices and terms of this
						Notice at any time, as permitted by applicable law. We reserve the right to make the
						changes in our privacy practices and the new terms of our Notice effective for all
						health information that we maintain, including health information we created or received
						before we made the changes. You may request a copy of the Notice at any time.
					</p>
				</section>

				<Separator />

				<!-- Contact Information -->
				<section class="rounded-lg bg-muted/50 p-6">
					<div class="mb-4 flex items-center gap-2">
						<Mail class="h-5 w-5 text-primary" />
						<h2 class="text-xl font-semibold">Questions and Complaints</h2>
					</div>
					<p class="mb-4 text-muted-foreground">
						For questions regarding this Notice or our privacy practices, please contact the
						University Privacy Officer. If you are concerned that your privacy rights may have been
						violated, you may contact the University Privacy Officer to make a complaint. You may
						also make a written complaint to the U.S. Department of Health and Human Services.
					</p>
					<p class="mb-6 text-muted-foreground">
						<strong>If you choose to make a complaint, we will not retaliate in any way.</strong>
					</p>
					<Card.Root>
						<Card.Content class="pt-6">
							<h3 class="mb-2 font-semibold">University Privacy Officer</h3>
							<address class="space-y-1 text-muted-foreground not-italic">
								<p>Jack Yetter</p>
								<p>Health Services Building 217</p>
								<p>Northern Illinois University</p>
								<p>DeKalb, IL 60115</p>
								<a href="mailto:jyetter@niu.edu" class="text-primary hover:underline"
									>jyetter@niu.edu</a
								>
							</address>
						</Card.Content>
					</Card.Root>
				</section>
			</Card.Content>
		</Card.Root>
	</div>
</div>

<style>
	.legal-page {
		padding: var(--spacing-xl) var(--spacing-md);
		min-height: 100vh;
	}

	.legal-container {
		max-width: var(--container-narrow);
		margin: 0 auto;
	}

	.legal-header {
		text-align: center;
		margin-bottom: var(--spacing-lg);
	}

	.legal-header :global(.header-badge) {
		margin-bottom: var(--spacing-sm);
	}

	.legal-title {
		font-size: 2.25rem;
		font-weight: bold;
		letter-spacing: -0.025em;
		margin-bottom: var(--spacing-sm);
	}

	.legal-subtitle {
		font-size: 1.125rem;
		color: hsl(var(--muted-foreground));
	}

	.legal-card-header {
		display: flex;
		align-items: center;
		gap: var(--spacing-xs);
	}

	.legal-icon {
		display: inline-flex;
		height: 3rem;
		width: 3rem;
		align-items: center;
		justify-content: center;
		border-radius: var(--radius-md);
		background-color: hsl(var(--primary) / 0.1);
	}

	.legal-intro {
		padding: var(--spacing-md);
		border-radius: var(--radius-lg);
		background-color: hsl(var(--muted) / 0.5);
	}

	.legal-text {
		color: hsl(var(--muted-foreground));
		line-height: 1.6;
	}

	.legal-intro .legal-text + .legal-text {
		margin-top: var(--spacing-sm);
	}

	@media (max-width: 768px) {
		.legal-title {
			font-size: 1.75rem;
		}

		.legal-subtitle {
			font-size: 1rem;
		}
	}
</style>
