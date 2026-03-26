const FEATURES = [
	{
		name: "Capture",
		title: "Archive public-channel threads and files automatically.",
		description:
			"Backfills keep history current without asking someone to export Slack by hand whenever context is missing.",
	},
	{
		name: "Search",
		title: "Find a thread by phrase, channel, or time window.",
		description:
			"Arkivist indexes the archive for retrieval instead of making people remember which month, channel, or teammate touched it.",
	},
	{
		name: "Summaries",
		title: "Turn long threads into something a busy teammate can scan.",
		description:
			"AI summaries condense the important parts without replacing the original conversation.",
	},
	{
		name: "Reading",
		title: "Save useful threads and reopen them offline.",
		description:
			"The archive is not only searchable. It is also readable when the network drops or the team is traveling.",
	},
	{
		name: "Control",
		title: "Keep the system in your infrastructure.",
		description:
			"Auth, storage, and access stay under your control instead of becoming another external black box.",
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
							Five jobs. No filler.
						</h2>
						<p className="mt-4 max-w-sm text-[1rem] leading-7 text-(--color-text-secondary)">
							The product should answer one question quickly: can this team
							reliably recover the thread they need later? These are the parts
							that make the answer yes.
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
