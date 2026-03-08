import { RoadmapCard } from "@/components/roadmap-card";
import { buttonVariants } from "@/components/ui/button";
import { fetchApiHealth } from "@/lib/api";
import { cn } from "@/lib/utils";
import { useQuery } from "@tanstack/react-query";
import { Boxes, DatabaseZap, Radar, Workflow } from "lucide-react";

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
	"db: local JSONL event store for offline development",
	"slack: signature verification and envelope parsing",
	"queue: direct worker publishing helper",
	"search: placeholder boundary for search_documents work",
];

const nextSlices = [
	"Port SQLx models and repositories into crates/db",
	"Replace the direct HTTP queue helper with QStash publishing",
	"Expand worker jobs beyond process_event and heartbeat",
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
								Milestone 0 Workspace
							</p>
							<h1 className="mt-5 max-w-2xl text-4xl font-bold tracking-tight text-white sm:text-6xl">
								Clean v2 workspace for the new Archivist architecture.
							</h1>
							<p className="mt-5 max-w-2xl text-base leading-7 text-slate-300 sm:text-lg">
								The legacy app now lives in backup, and the new monorepo starts
								from a clean Rust services + Vite frontend baseline aligned with
								TASKS.md.
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
								The frontend probes the new API health endpoint directly. Once
								the services are up, this card becomes the simplest end-to-end
								smoke signal in the workspace.
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
						title="A thin, useful foundation"
						description="The crates are small on purpose, but they already carry real shape: payload normalization, deduplication, signature checks, and queue boundaries."
						items={sharedCrates}
					/>
					<RoadmapCard
						eyebrow="Next development slice"
						title="Move from mock vertical to real infra"
						description="This shell now supports the next phase without dragging the legacy runtime around. The remaining work is concrete and incremental."
						items={nextSlices}
					/>
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
