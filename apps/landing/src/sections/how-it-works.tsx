const STEPS = [
	{
		step: "01",
		title: "Connect Slack and schedule the archive.",
		description:
			"Arkivist backfills public-channel history so the archive becomes a living record instead of a one-time import.",
	},
	{
		step: "02",
		title: "Store the thread, file, and summary together.",
		description:
			"Messages stay readable, summaries stay attached, and the original context remains available when someone needs the full thread.",
	},
	{
		step: "03",
		title: "Open the archive from catch-up, search, or saved reading.",
		description:
			"People can recover the right conversation from the workflow that matches the moment instead of hunting through Slack.",
	},
];

export function HowItWorks() {
	return (
		<section id="workflow" className="px-6 py-12 sm:py-16">
			<div className="mx-auto max-w-6xl border-t border-(--color-border-subtle) pt-8">
				<div className="grid gap-10 lg:grid-cols-[minmax(0,0.9fr)_minmax(0,1.1fr)]">
					<div>
						<p className="text-[0.78rem] font-medium uppercase tracking-[0.22em] text-(--color-accent)">
							How it works
						</p>
						<h2 className="mt-3 max-w-md text-[clamp(2rem,5vw,3.3rem)] font-semibold leading-[1.02] tracking-[-0.035em] text-(--color-text-primary)">
							Built to reduce re-explaining, not to create another dashboard.
						</h2>
					</div>
					<div className="space-y-8">
						{STEPS.map((item) => (
							<article
								key={item.step}
								className="grid gap-4 sm:grid-cols-[72px_minmax(0,1fr)]"
							>
								<p className="text-xl font-semibold tracking-tight text-(--color-text-quiet)">
									{item.step}
								</p>
								<div className="border-l border-(--color-border-subtle) pl-5">
									<h3 className="text-lg font-semibold tracking-tight text-(--color-text-primary)">
										{item.title}
									</h3>
									<p className="mt-2 max-w-2xl text-sm leading-6 text-(--color-text-secondary)">
										{item.description}
									</p>
								</div>
							</article>
						))}
					</div>
				</div>
			</div>
		</section>
	);
}
