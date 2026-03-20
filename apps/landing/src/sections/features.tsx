import {
	Bookmark,
	Flame,
	Search,
	Shield,
	Sparkles,
	WifiOff,
	Zap,
} from "lucide-react";
import { useInView } from "../hooks/use-in-view";

const FEATURES = [
	{
		icon: Zap,
		title: "Automatic capture",
		description:
			"Every message, reaction, file, and thread reply is captured through periodic backfills. Incremental and resumable — your archive stays fresh without any manual exports.",
	},
	{
		icon: Search,
		title: "Full-text search",
		description:
			"Search across every archived thread. Filter by channel, date range, sort by relevance or time. Find any decision in seconds.",
	},
	{
		icon: Sparkles,
		title: "AI summaries",
		description:
			"Long threads get automatic summaries — topic tags, key takeaways, status classification, and staleness detection. Skim 50 messages in 3 sentences.",
	},
	{
		icon: Flame,
		title: "Smart feeds",
		description:
			"Time-windowed feeds surface what matters: Fresh (24h), Steady (7d), and Trending. Plus admin-curated starred highlights.",
	},
	{
		icon: Bookmark,
		title: "Save & bookmark",
		description:
			"Bookmark threads for quick access. Build a personal library. Saved threads are cached locally for offline reading.",
	},
	{
		icon: WifiOff,
		title: "Offline reading",
		description:
			"PWA with Service Worker caching. Saved threads stored in IndexedDB — read them on a plane, on the subway, during an outage.",
	},
	{
		icon: Shield,
		title: "Workspace security",
		description:
			"Slack OIDC authentication. Role-based permissions. Workspace-scoped access. Your data never leaves your infrastructure.",
	},
];

export function Features() {
	const { ref, visible } = useInView();

	return (
		<section ref={ref} id="features" className="px-6 py-20">
			<div className="mx-auto max-w-5xl">
				<div
					className={`text-center ${visible ? "anim-fade-up" : "opacity-0"}`}
				>
					<p
						className="text-sm font-medium tracking-wide uppercase"
						style={{ color: "var(--color-accent-soft)" }}
					>
						Features
					</p>
					<h2
						className="mt-3 text-3xl font-bold tracking-tight sm:text-4xl"
						style={{ color: "var(--color-text-primary)" }}
					>
						Everything you need to recall
					</h2>
					<p
						className="mx-auto mt-4 max-w-2xl text-lg"
						style={{ color: "var(--color-text-secondary)" }}
					>
						A complete toolkit for capturing, searching, and surfacing your
						team&apos;s Slack knowledge.
					</p>
				</div>

				<div className="mt-16 grid gap-5 sm:grid-cols-2 lg:grid-cols-3">
					{FEATURES.map((feature, i) => (
						<div
							key={feature.title}
							className={`group rounded-2xl border p-6 transition-all ${visible ? `anim-fade-up anim-pulse-border d-${Math.min(i * 100, 600)}` : "opacity-0"}`}
							style={{
								borderColor: "var(--color-border-subtle)",
								background: "color-mix(in srgb, white 3%, transparent)",
							}}
							onMouseEnter={(e) => {
								e.currentTarget.style.borderColor =
									"color-mix(in srgb, var(--color-accent) 24%, transparent)";
								e.currentTarget.style.background =
									"color-mix(in srgb, var(--color-accent) 5%, white 3%)";
								e.currentTarget.style.transform = "translateY(-2px)";
							}}
							onMouseLeave={(e) => {
								e.currentTarget.style.borderColor =
									"var(--color-border-subtle)";
								e.currentTarget.style.background =
									"color-mix(in srgb, white 3%, transparent)";
								e.currentTarget.style.transform = "translateY(0)";
							}}
						>
							<div className="flex items-center gap-3">
								<div
									className="flex h-10 w-10 shrink-0 items-center justify-center rounded-lg"
									style={{
										background:
											"color-mix(in srgb, var(--color-accent) 12%, transparent)",
										color: "var(--color-accent-soft)",
									}}
								>
									<feature.icon size={20} />
								</div>
								<h3
									className="text-base font-semibold"
									style={{ color: "var(--color-text-bright)" }}
								>
									{feature.title}
								</h3>
							</div>
							<p
								className="mt-2 text-sm leading-relaxed"
								style={{ color: "var(--color-text-tertiary)" }}
							>
								{feature.description}
							</p>
						</div>
					))}
				</div>
			</div>
		</section>
	);
}
