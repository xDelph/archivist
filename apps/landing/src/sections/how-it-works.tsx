import { Cable, Database, Layout, RefreshCw } from "lucide-react";
import { useInView } from "../hooks/use-in-view";

const STEPS = [
	{
		icon: Cable,
		step: "01",
		title: "Install & authenticate",
		description:
			"Install the Arkivist Slack app in your workspace. Provide a user token for backfill access — Arkivist needs it to pull channel history and sync users. OIDC handles login.",
	},
	{
		icon: RefreshCw,
		step: "02",
		title: "Backfill & stay current",
		description:
			"Arkivist backfills your workspace — channels, messages, threads, reactions, files. Incremental and resumable. Periodic syncs keep your archive up to date automatically.",
	},
	{
		icon: Database,
		step: "03",
		title: "AI summaries & indexing",
		description:
			"Background workers generate AI summaries for every thread, archive files to durable storage, and build full-text search indexes. All async, all automatic.",
	},
	{
		icon: Layout,
		step: "04",
		title: "Search, catch up, read offline",
		description:
			"Open the web app or install the PWA. Browse time-windowed feeds, search across everything, save threads for offline reading. Use Slack commands to pin highlights.",
	},
];

export function HowItWorks() {
	const { ref, visible } = useInView();

	return (
		<section ref={ref} id="how-it-works" className="px-6 py-20">
			<div className="mx-auto max-w-5xl">
				<div
					className={`text-center ${visible ? "anim-fade-up" : "opacity-0"}`}
				>
					<p
						className="text-sm font-medium tracking-wide uppercase"
						style={{ color: "var(--color-accent-soft)" }}
					>
						How it works
					</p>
					<h2
						className="mt-3 text-3xl font-bold tracking-tight sm:text-4xl"
						style={{ color: "var(--color-text-primary)" }}
					>
						Up and running in minutes
					</h2>
					<p
						className="mx-auto mt-4 max-w-2xl text-lg"
						style={{ color: "var(--color-text-secondary)" }}
					>
						From zero to a fully searchable Slack archive with AI summaries.
					</p>
				</div>

				<div className="mt-16 grid gap-6 sm:grid-cols-2 lg:grid-cols-4">
					{STEPS.map((step, i) => (
						<div
							key={step.step}
							className={
								visible ? `anim-fade-up d-${(i + 1) * 200}` : "opacity-0"
							}
						>
							<div className="relative">
								{/* Step number */}
								<span
									className="text-[11px] font-bold tracking-widest uppercase"
									style={{ color: "var(--color-accent-muted)" }}
								>
									Step {step.step}
								</span>

								{/* Timeline connector */}
								<div
									className="mt-2 mb-3 h-px"
									style={{
										background:
											"linear-gradient(to right, var(--color-accent-soft), transparent)",
										transformOrigin: "left",
										animation: visible
											? `timeline-draw 0.8s var(--ease-out-expo) ${(i + 1) * 300}ms both`
											: "none",
									}}
								/>

								{/* Icon + title */}
								<div className="flex items-center gap-3">
									<div
										className="flex h-10 w-10 shrink-0 items-center justify-center rounded-xl"
										style={{
											background: "var(--color-accent)",
											color: "var(--color-on-accent)",
											boxShadow: "var(--shadow-accent)",
										}}
									>
										<step.icon size={20} />
									</div>
									<h3
										className="text-base font-semibold"
										style={{ color: "var(--color-text-bright)" }}
									>
										{step.title}
									</h3>
								</div>
								<p
									className="mt-2 text-sm leading-relaxed"
									style={{ color: "var(--color-text-tertiary)" }}
								>
									{step.description}
								</p>
							</div>
						</div>
					))}
				</div>
			</div>
		</section>
	);
}
