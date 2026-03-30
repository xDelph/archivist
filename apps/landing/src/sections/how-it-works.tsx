const STEPS = [
	{
		step: "01",
		title: "Connect Slack and bootstrap the stack.",
		description:
			"Arkivist can run as a self-hosted frontend, API, ingest, worker, and migration stack, then start pulling public-channel history.",
	},
	{
		step: "02",
		title: "Let the worker keep history fresh and enrich it.",
		description:
			"Background backfills attach files, generate summaries, and keep thread previews aligned with the latest activity instead of freezing after the first import.",
	},
	{
		step: "03",
		title: "Reopen the right thread from Catch up, Search, or Saved.",
		description:
			"People can start with Fresh, This Week, or Starred, jump to filtered search when needed, and keep critical threads available offline.",
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
							Built to shorten the path from a question back to the thread that
							answered it.
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
