import { AlertTriangle, Clock, Search } from "lucide-react";
import { ScatteredPages } from "../components/scattered-pages";
import { useInView } from "../hooks/use-in-view";

const PAIN_POINTS = [
	{
		icon: Clock,
		title: "Conversations vanish",
		description:
			"Slack's free plan erases messages after 90 days. Even paid plans bury threads under endless scroll. Critical decisions disappear.",
	},
	{
		icon: Search,
		title: "Search is broken",
		description:
			"Slack search returns noise, not answers. Finding that one thread from three months ago? Good luck scrolling through hundreds of results.",
	},
	{
		icon: AlertTriangle,
		title: "Knowledge walks out the door",
		description:
			"When team members leave, their context goes with them. Onboarding takes weeks because institutional knowledge lives in lost threads.",
	},
];

export function Problem() {
	const { ref, visible } = useInView();

	return (
		<section ref={ref} className="relative px-6 py-20">
			<ScatteredPages />
			<div className="relative mx-auto max-w-5xl">
				<div
					className={`text-center ${visible ? "anim-fade-up" : "opacity-0"}`}
				>
					<p
						className="text-sm font-medium tracking-wide uppercase"
						style={{ color: "var(--color-accent-soft)" }}
					>
						The problem
					</p>
					<h2
						className="mt-3 text-3xl font-bold tracking-tight sm:text-4xl"
						style={{ color: "var(--color-text-primary)" }}
					>
						Your best ideas are trapped in Slack
					</h2>
					<p
						className="mx-auto mt-4 max-w-2xl text-lg"
						style={{ color: "var(--color-text-secondary)" }}
					>
						Every day, your team makes decisions, solves problems, and shares
						knowledge in Slack. And every day, those conversations sink into the
						void.
					</p>
				</div>

				<div className="mt-16 grid gap-6 md:grid-cols-3">
					{PAIN_POINTS.map((point, i) => (
						<div
							key={point.title}
							className={`rounded-2xl border p-7 ${visible ? `anim-fade-up d-${(i + 1) * 200}` : "opacity-0"}`}
							style={{
								borderColor: "var(--color-border-subtle)",
								background:
									"color-mix(in srgb, var(--color-bg-panel) 94%, white 6%)",
								boxShadow: "var(--shadow-panel)",
							}}
						>
							<div className="flex items-center gap-3">
								<div
									className="flex h-11 w-11 shrink-0 items-center justify-center rounded-xl"
									style={{
										background:
											"color-mix(in srgb, var(--color-accent) 12%, transparent)",
										color: "var(--color-accent-soft)",
									}}
								>
									<point.icon size={22} />
								</div>
								<h3
									className="text-lg font-semibold"
									style={{ color: "var(--color-text-bright)" }}
								>
									{point.title}
								</h3>
							</div>
							<p
								className="mt-2 text-sm leading-relaxed"
								style={{ color: "var(--color-text-tertiary)" }}
							>
								{point.description}
							</p>
						</div>
					))}
				</div>
			</div>
		</section>
	);
}
