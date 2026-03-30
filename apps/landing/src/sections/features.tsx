const FEATURES = [
	{
		name: "Capture",
		title: "Continuously backfill public-channel threads, files, and activity.",
		description:
			"Workers keep the archive warm in the background so new catch-up views and searches do not depend on a manual export.",
	},
	{
		name: "Catch up",
		title: "Open Fresh, This Week, or Starred instead of raw scrollback.",
		description:
			"The app surfaces the threads worth revisiting first, with shared sorting across the main lists.",
	},
	{
		name: "Search",
		title: "Filter by phrase, channel, date range, or ranking.",
		description:
			"Search is tuned for finding the right conversation and opening the full thread card immediately.",
	},
	{
		name: "Summaries",
		title: "Read full AI thread briefs with status, tags, and context.",
		description:
			"Short previews help scan lists; full summaries and why-it-mattered notes stay attached to thread detail.",
	},
	{
		name: "Saved",
		title: "Bookmark useful threads and keep them readable offline.",
		description:
			"Saved conversations stay available for travel, spotty networks, and installable home-screen access.",
	},
	{
		name: "Control",
		title: "Run the whole stack in your own infrastructure.",
		description:
			"Frontend, API, ingest, worker, and migrations can ship together without turning the archive into somebody else's SaaS.",
	},
];

export function Features() {
	return (
		<section id="features" className="px-6 py-12 sm:py-16">
			<div className="mx-auto max-w-6xl border-t border-(--color-border-subtle) pt-8">
				<div className="grid gap-10 lg:grid-cols-[minmax(0,0.78fr)_minmax(0,1.22fr)]">
					<div>
						<p className="text-[0.78rem] font-medium uppercase tracking-[0.22em] text-(--color-accent)">
							What it does
						</p>
						<h2 className="mt-3 text-[clamp(2rem,5vw,3.3rem)] font-semibold leading-[1.02] tracking-[-0.035em] text-(--color-text-primary)">
							Six jobs. Still focused.
						</h2>
						<p className="mt-4 max-w-sm text-[1rem] leading-7 text-(--color-text-secondary)">
							The product answers a sharper question now: can a teammate recover
							the right thread, on time, from wherever they are? These are the
							capabilities that make that true.
						</p>
					</div>
					<div className="space-y-6">
						{FEATURES.map((feature) => (
							<article
								key={feature.name}
								className="grid gap-3 border-b border-(--color-border-subtle) pb-6 last:border-b-0 last:pb-0 sm:grid-cols-[120px_minmax(0,1fr)]"
							>
								<p className="text-[0.72rem] font-medium uppercase tracking-[0.18em] text-(--color-accent)">
									{feature.name}
								</p>
								<div>
									<h3 className="text-lg font-semibold tracking-tight text-(--color-text-primary)">
										{feature.title}
									</h3>
									<p className="mt-2 max-w-2xl text-sm leading-6 text-(--color-text-secondary)">
										{feature.description}
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
