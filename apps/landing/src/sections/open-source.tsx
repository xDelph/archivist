import { ArrowUpRight, Github } from "lucide-react";

const REPO_URL = "https://github.com/xDelph/archivist";

const TRUST_POINTS = [
	"AGPL-3.0 licensed.",
	"Compose-ready self-hosted stack.",
	"Frontend, API, ingest, and worker stay under your control.",
];

export function OpenSource() {
	return (
		<section id="open-source" className="px-6 py-12 sm:py-16">
			<div
				className="mx-auto max-w-6xl border bg-(--color-bg-panel) p-6 sm:p-8"
				style={{
					borderColor: "var(--landing-panel-border)",
					boxShadow: "var(--landing-panel-shadow)",
				}}
			>
				<div
					className="mb-6 h-px w-16"
					style={{ background: "var(--landing-panel-top-line)" }}
				/>
				<div className="grid gap-10 lg:grid-cols-[minmax(0,1.1fr)_minmax(0,0.9fr)] lg:items-end">
					<div>
						<p className="text-[0.78rem] font-medium uppercase tracking-[0.22em] text-(--color-accent)">
							Open source
						</p>
						<h2 className="mt-3 max-w-2xl text-[clamp(2rem,5vw,3.3rem)] font-semibold leading-[1.02] tracking-[-0.035em] text-(--color-text-primary)">
							If Slack is part of how your team thinks, the archive should be
							runnable and inspectable too.
						</h2>
						<p className="mt-4 max-w-2xl text-[1rem] leading-7 text-(--color-text-secondary)">
							Arkivist is open source, self-hostable, and designed so the same
							stack that captures history is the one your team reads from later.
						</p>
					</div>
					<div className="space-y-6">
						<ul className="space-y-3">
							{TRUST_POINTS.map((point) => (
								<li
									key={point}
									className="flex items-start gap-3 text-sm leading-6 text-(--color-text-secondary)"
								>
									<span className="mt-2 h-1.5 w-1.5 rounded-full bg-(--color-accent)" />
									<span>{point}</span>
								</li>
							))}
						</ul>
						<a
							href={REPO_URL}
							target="_blank"
							rel="noreferrer"
							className="inline-flex min-h-12 items-center gap-2 rounded-full border border-(--color-border-strong) px-5 text-sm font-semibold text-(--color-text-primary) transition-colors hover:border-(--color-border-accent) hover:bg-(--color-bg-elevated)"
						>
							<Github className="size-4" />
							View repository
							<ArrowUpRight className="size-4" />
						</a>
					</div>
				</div>
			</div>
		</section>
	);
}
