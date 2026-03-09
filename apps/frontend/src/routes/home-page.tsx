import { RoadmapCard } from "@/components/roadmap-card";
import { buttonVariants } from "@/components/ui/button";
import { fetchApiHealth } from "@/lib/api";
import { cn } from "@/lib/utils";
import { useQuery } from "@tanstack/react-query";
import { Boxes, DatabaseZap, Radar, ScrollText, Workflow } from "lucide-react";

const services = [
	{
		name: "api",
		port: "4000",
		description:
			"Public JSON API shell ready for auth, search, and thread reads.",
		icon: Radar,
	},
	{
		name: "ingest",
		port: "4001",
		description:
			"Slack Events API receiver with signature validation and worker publish.",
		icon: Workflow,
	},
	{
		name: "worker",
		port: "4002",
		description:
			"Background consumer with local mock persistence for the first vertical.",
		icon: DatabaseZap,
	},
];

const sharedCrates = [
	"domain: channel scope and process-event job types",
	"db: JSONL and in-memory stores for offline tests and worker replays",
	"slack: signature verification and public-channel event parsing",
	"queue: direct delivery plus signed QStash publishing helpers",
	"search: normalized query/filter placeholder for ranked search work",
];

const workingVertical = [
	"Slack event ingestion acknowledges public-channel callbacks and ignores unsupported envelopes.",
	"Signed QStash-style delivery is covered end-to-end with a local mock and worker verification.",
	"Worker persistence now stores messages, reactions, file shares, and channel rename/archive state.",
	"Slack slash command endpoints exist as safe stubs for /ask-archivist, /recap, and /save-thread.",
];

const nextSlices = [
	"Slack auth and session handling in the API layer",
	"Worker backfill jobs for channel history and file archival",
	"Real thread and search surfaces in the frontend instead of a workspace shell",
];

export function HomePage() {
	const apiHealth = useQuery({
		queryKey: ["api-health"],
		queryFn: fetchApiHealth,
	});

	const statusText = apiHealth.isPending
		? "Waiting for the API shell"
		: apiHealth.isError
			? "API shell offline"
			: `API shell healthy: ${apiHealth.data.service}@${apiHealth.data.version}`;
	const statusTone = apiHealth.isError
		? "bg-rose-400"
		: apiHealth.isSuccess
			? "bg-[var(--accent-strong)]"
			: "bg-amber-300";

	return (
		<main className="relative isolate min-h-screen overflow-hidden px-6 py-10 text-white sm:px-10">
			<div className="mx-auto flex w-full max-w-6xl flex-col gap-8">
				<section className="overflow-hidden rounded-[2.5rem] border border-white/10 bg-[linear-gradient(135deg,rgba(18,28,44,0.94),rgba(7,17,26,0.88))] px-8 py-10 shadow-[0_30px_120px_rgba(4,9,16,0.6)]">
					<div className="grid gap-10 lg:grid-cols-[1.35fr_0.95fr] lg:items-end">
						<div>
							<p className="text-xs uppercase tracking-[0.34em] text-[var(--accent-soft)]">
								Milestone 0 Vertical
							</p>
							<h1 className="mt-5 max-w-2xl text-4xl font-bold tracking-tight text-white sm:text-6xl">
								The clean v2 workspace is up. The first Slack vertical is live.
							</h1>
							<p className="mt-5 max-w-2xl text-base leading-7 text-slate-300 sm:text-lg">
								The legacy app is parked in backup. The new monorepo now has a
								real ingest to worker flow, offline persistence helpers, and a
								frontend shell that tracks what is actually shipping next.
							</p>
							<div className="mt-8 flex flex-wrap gap-3">
								<a
									className={buttonVariants({ variant: "default" })}
									href="http://127.0.0.1:4001/health"
									rel="noreferrer"
									target="_blank"
								>
									Check ingest
								</a>
								<a
									className={buttonVariants({ variant: "secondary" })}
									href="http://127.0.0.1:4002/health"
									rel="noreferrer"
									target="_blank"
								>
									Check worker
								</a>
							</div>
						</div>
						<div className="rounded-[2rem] border border-white/10 bg-white/6 p-6 backdrop-blur">
							<div className="flex items-center gap-3">
								<span className={cn("size-3 rounded-full", statusTone)} />
								<p className="text-xs uppercase tracking-[0.28em] text-slate-300">
									Local health signal
								</p>
							</div>
							<p className="mt-4 text-2xl font-semibold text-white">
								{statusText}
							</p>
							<p className="mt-3 text-sm leading-6 text-slate-300">
								The frontend probes the API health endpoint directly. It is
								still a shell, but it now mirrors the actual backend milestone
								instead of the bootstrap state.
							</p>
						</div>
					</div>
				</section>

				<section className="grid gap-4 lg:grid-cols-3">
					{services.map((service) => {
						const Icon = service.icon;

						return (
							<article
								key={service.name}
								className="rounded-[2rem] border border-white/10 bg-white/6 p-6 backdrop-blur"
							>
								<div className="flex items-center justify-between">
									<div>
										<p className="text-xs uppercase tracking-[0.28em] text-slate-300">
											service
										</p>
										<h2 className="mt-3 text-2xl font-semibold text-white">
											{service.name}
										</h2>
									</div>
									<div className="rounded-2xl border border-white/10 bg-white/8 p-3">
										<Icon className="size-6 text-[var(--accent-soft)]" />
									</div>
								</div>
								<p className="mt-4 text-sm leading-6 text-slate-300">
									{service.description}
								</p>
								<p className="mt-6 text-xs uppercase tracking-[0.24em] text-slate-400">
									localhost:{service.port}
								</p>
							</article>
						);
					})}
				</section>

				<section className="grid gap-5 lg:grid-cols-2">
					<RoadmapCard
						eyebrow="Shared crates"
						title="Small crates, real behavior"
						description="The shared layer is still lean, but it already carries the core contracts for event normalization, deduplication, queue delivery, and placeholder search boundaries."
						items={sharedCrates}
					/>
					<RoadmapCard
						eyebrow="Vertical status"
						title="What works right now"
						description="The first delivery path is not theoretical anymore. These slices are covered in the current workspace and test suite."
						items={workingVertical}
					/>
				</section>

				<section className="grid gap-5 lg:grid-cols-2">
					<RoadmapCard
						eyebrow="Next development slice"
						title="What moves the product forward"
						description="The remaining work has shifted from workspace bootstrapping to actual product capability: identity, backfills, and the first reader surfaces."
						items={nextSlices}
					/>
					<article className="rounded-[2rem] border border-white/10 bg-white/6 p-6 shadow-[0_20px_50px_rgba(3,7,18,0.35)] backdrop-blur">
						<div className="flex items-center gap-3">
							<ScrollText className="size-5 text-[var(--accent-soft)]" />
							<h2 className="text-xl font-semibold text-white">
								Current operating model
							</h2>
						</div>
						<p className="mt-4 text-sm leading-6 text-slate-300">
							Development is running as short vertical slices. Legacy code can
							be referenced from backup, but the active workspace stays clean,
							small, and validated after each task.
						</p>
						<ul className="mt-6 space-y-3 text-sm text-slate-200">
							<li className="flex items-start gap-3">
								<span className="mt-1 size-2 rounded-full bg-[var(--accent-soft)]" />
								<span>
									Rust apps finish every slice with format, lint, test, build,
									and check.
								</span>
							</li>
							<li className="flex items-start gap-3">
								<span className="mt-1 size-2 rounded-full bg-[var(--accent-soft)]" />
								<span>
									Backend logic is exercised offline first through JSONL and
									in-memory stores.
								</span>
							</li>
							<li className="flex items-start gap-3">
								<span className="mt-1 size-2 rounded-full bg-[var(--accent-soft)]" />
								<span>
									Frontend work follows the new services instead of preserving
									old app structure.
								</span>
							</li>
						</ul>
					</article>
				</section>

				<section className="rounded-[2rem] border border-white/10 bg-white/6 p-6 backdrop-blur">
					<div className="flex items-center gap-3">
						<Boxes className="size-5 text-[var(--accent-soft)]" />
						<h2 className="text-xl font-semibold text-white">
							Why this shell exists
						</h2>
					</div>
					<p className="mt-4 max-w-3xl text-sm leading-6 text-slate-300">
						The goal here is not polish for its own sake. This frontend gives
						the new workspace a live entrypoint, TanStack Router and Query are
						wired, shadcn conventions are present, and the health probe makes it
						obvious when the backend slice is alive.
					</p>
				</section>
			</div>
		</main>
	);
}
