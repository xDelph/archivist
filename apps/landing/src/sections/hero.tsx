import { ArrowRight, Github } from "lucide-react";

const REPO_URL = "https://github.com/xDelph/archivist";

const PROOF_POINTS = [
	"Archives public-channel history without manual exports.",
	"Searches threads, files, and summaries from one place.",
	"Keeps the archive inside your infrastructure.",
];

const PREVIEW_THREADS = [
	{
		channel: "#product",
		title: "Pricing rollout notes",
		summary: "Seven replies, one decision, and the final launch checklist.",
	},
	{
		channel: "#engineering",
		title: "Incident recap",
		summary: "Recovered root cause, linked files, and next follow-ups.",
	},
	{
		channel: "#ops",
		title: "Vendor renewal thread",
		summary: "Contract terms, approvals, and the final owner assignment.",
	},
];

export function Hero() {
	return (
		<section id="top" className="px-6 pb-12 pt-14 sm:pb-16 sm:pt-18">
			<div className="mx-auto grid max-w-6xl gap-12 lg:grid-cols-[minmax(0,1.2fr)_minmax(320px,0.8fr)] lg:items-start">
				<div className="max-w-3xl">
					<p className="text-[0.78rem] font-medium uppercase tracking-[0.24em] text-(--color-accent)">
						Slack archives for teams that need receipts
					</p>
					<h1
						className="mt-5 max-w-4xl text-[clamp(3rem,9vw,5.8rem)] font-semibold leading-[0.96] tracking-[-0.04em] text-(--color-text-primary)"
						style={{ textShadow: "var(--landing-title-shadow)" }}
					>
						Keep every important thread readable after Slack forgets it.
					</h1>
					<p className="mt-6 max-w-2xl text-[1.05rem] leading-7 text-(--color-text-secondary) sm:text-[1.18rem]">
						Arkivist captures public-channel history, turns long threads into
						quick summaries, and gives your team a calm place to search for
						decisions instead of reconstructing them from memory.
					</p>
					<div className="mt-8 flex flex-col gap-3 sm:flex-row">
						<a
							href={REPO_URL}
							target="_blank"
							rel="noreferrer"
							className="inline-flex min-h-12 items-center justify-center gap-2 rounded-full bg-(--color-accent) px-6 text-sm font-semibold text-(--color-on-accent) transition-colors hover:bg-(--color-accent-strong)"
						>
							Read the code
							<Github className="size-4" />
						</a>
						<a
							href="#workflow"
							className="inline-flex min-h-12 items-center justify-center gap-2 rounded-full border border-(--color-border-strong) px-6 text-sm font-semibold text-(--color-text-primary) transition-colors hover:border-(--color-border-accent) hover:bg-(--color-bg-panel)"
						>
							See the workflow
							<ArrowRight className="size-4" />
						</a>
					</div>
					<ul className="mt-10 space-y-3 border-t border-(--color-border-subtle) pt-6">
						{PROOF_POINTS.map((point) => (
							<li
								key={point}
								className="flex items-start gap-3 text-sm leading-6 text-(--color-text-secondary)"
							>
								<span className="mt-2 h-1.5 w-1.5 rounded-full bg-(--color-accent)" />
								<span>{point}</span>
							</li>
						))}
					</ul>
				</div>

				<ArchivePreview />
			</div>
		</section>
	);
}

function ArchivePreview() {
	return (
		<section
			className="border bg-(--color-bg-panel) p-5 sm:p-6"
			style={{
				borderColor: "var(--landing-panel-border)",
				boxShadow: "var(--landing-panel-shadow)",
			}}
		>
			<div
				className="mb-5 h-px w-16"
				style={{ background: "var(--landing-panel-top-line)" }}
			/>
			<div className="flex items-end justify-between gap-4 border-b border-(--color-border-subtle) pb-4">
				<div>
					<p className="text-[0.72rem] uppercase tracking-[0.18em] text-(--color-text-quiet)">
						Archive view
					</p>
					<h2 className="mt-1 text-xl font-semibold tracking-tight text-(--color-text-primary)">
						One place to reopen the thread that actually settled it
					</h2>
				</div>
				<p className="text-right text-[0.72rem] uppercase tracking-[0.18em] text-(--color-accent)">
					Search
					<br />
					Summaries
					<br />
					Offline
				</p>
			</div>

			<div className="mt-5 space-y-4">
				{PREVIEW_THREADS.map((thread, index) => (
					<article
						key={thread.title}
						className="border-b border-(--color-border-subtle) pb-4 last:border-b-0 last:pb-0"
					>
						<div className="flex items-start gap-4">
							<span className="pt-1 text-[0.78rem] font-medium text-(--color-text-quiet)">
								0{index + 1}
							</span>
							<div className="min-w-0">
								<p className="text-[0.72rem] uppercase tracking-[0.18em] text-(--color-accent)">
									{thread.channel}
								</p>
								<h3 className="mt-1 text-base font-semibold text-(--color-text-primary)">
									{thread.title}
								</h3>
								<p className="mt-1 text-sm leading-6 text-(--color-text-secondary)">
									{thread.summary}
								</p>
							</div>
						</div>
					</article>
				))}
			</div>

			<div className="mt-5 border-t border-(--color-border-subtle) pt-4 text-sm leading-6 text-(--color-text-secondary)">
				Arkivist is for teams that treat Slack as working memory but still need
				retrievable evidence when a launch, incident, or policy decision comes
				back up later.
			</div>
		</section>
	);
}
