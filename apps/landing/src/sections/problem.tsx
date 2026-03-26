const PROBLEMS = [
	{
		number: "01",
		title: "Decisions disappear into scrollback.",
		description:
			"Slack is where teams actually decide things, but its default experience treats that history as disposable chat.",
	},
	{
		number: "02",
		title: "Search returns fragments instead of context.",
		description:
			"Even when you find the right phrase, you still have to rebuild the thread, the file, and the final outcome yourself.",
	},
	{
		number: "03",
		title: "The person who remembers becomes the system.",
		description:
			"That works until they leave, go offline, or simply stop being available when the question comes back.",
	},
];

export function Problem() {
	return (
		<section id="why" className="px-6 py-12 sm:py-16">
			<div className="mx-auto max-w-6xl border-t border-(--color-border-subtle) pt-8">
				<div className="grid gap-8 lg:grid-cols-[minmax(0,0.8fr)_minmax(0,1.2fr)]">
					<div>
						<p className="text-[0.78rem] font-medium uppercase tracking-[0.22em] text-(--color-accent)">
							Why it matters
						</p>
						<h2 className="mt-3 max-w-md text-[clamp(2rem,5vw,3.3rem)] font-semibold leading-[1.02] tracking-[-0.035em] text-(--color-text-primary)">
							Most teams do not need more chat. They need recall.
						</h2>
					</div>
					<div className="space-y-6">
						{PROBLEMS.map((problem) => (
							<article
								key={problem.number}
								className="grid gap-3 border-b border-(--color-border-subtle) pb-6 last:border-b-0 last:pb-0 sm:grid-cols-[56px_minmax(0,1fr)]"
							>
								<p className="text-sm font-medium text-(--color-text-quiet)">
									{problem.number}
								</p>
								<div>
									<h3 className="text-lg font-semibold tracking-tight text-(--color-text-primary)">
										{problem.title}
									</h3>
									<p className="mt-2 max-w-2xl text-sm leading-6 text-(--color-text-secondary)">
										{problem.description}
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
