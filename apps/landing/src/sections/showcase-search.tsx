import {
	Calendar,
	ChevronDown,
	Hash,
	Heart,
	MessageSquare,
	Search,
	SlidersHorizontal,
	Users,
} from "lucide-react";
import { MockAvatar } from "../components/mock-avatar";
import { MockBrowser } from "../components/mock-browser";
import { SearchScan } from "../components/search-scan";
import { useInView } from "../hooks/use-in-view";

const RESULTS = [
	{
		author: "Maya Patel",
		initials: "MP",
		tone: 4,
		channel: "engineering",
		channelTone: 1,
		title: "Connection pool exhaustion during peak load",
		preview:
			"The root cause was the aggregation worker holding connections for too long. I've added a 30s timeout and a circuit breaker pattern...",
		highlight: "connection pool",
		replies: 18,
		reactions: 5,
		participants: 7,
		time: "Mar 12",
	},
	{
		author: "Carlos Mendez",
		initials: "CM",
		tone: 2,
		channel: "incidents",
		channelTone: 3,
		title: "Database connection limits on Neon",
		preview:
			"We're hitting the 100 connection limit during deployments. Options: pgbouncer, or switching to pooled connection strings...",
		highlight: "connection",
		replies: 24,
		reactions: 3,
		participants: 5,
		time: "Mar 8",
	},
];

export function ShowcaseSearch() {
	const { ref, visible } = useInView();

	return (
		<section ref={ref} className="px-6 py-20">
			<div className="mx-auto max-w-5xl">
				<div className="grid items-center justify-items-center gap-12 lg:justify-items-stretch lg:grid-cols-[1fr_1.4fr]">
					<div className={visible ? "anim-slide-left" : "opacity-0"}>
						<p
							className="text-sm font-medium tracking-wide uppercase"
							style={{ color: "var(--color-accent-soft)" }}
						>
							Search
						</p>
						<h2
							className="mt-3 text-3xl font-bold tracking-tight sm:text-4xl"
							style={{ color: "var(--color-text-primary)" }}
						>
							Find anything,
							<br />
							instantly
						</h2>
						<p
							className="mt-4 text-lg leading-relaxed"
							style={{ color: "var(--color-text-secondary)" }}
						>
							Full-text search across every archived thread. Filter by channel,
							narrow by date range, sort by relevance or time. The thread you
							need is always one search away.
						</p>
					</div>

					<div className={visible ? "anim-slide-right d-200" : "opacity-0"}>
						<MockBrowser url="app.arkivist.dev/search">
							<div
								className="p-4"
								style={{
									background:
										"radial-gradient(circle at top center, color-mix(in srgb, var(--color-accent) 6%, transparent), transparent 50%), var(--color-bg-base)",
								}}
							>
								{/* Search panel */}
								<div
									className="rounded-xl border p-4"
									style={{
										borderColor: "var(--color-border-subtle)",
										background:
											"color-mix(in srgb, var(--color-bg-panel) 94%, white 6%)",
										boxShadow: "var(--shadow-panel)",
									}}
								>
									<p
										className="text-[9px] font-medium tracking-[0.16em] uppercase"
										style={{ color: "var(--color-accent-soft)" }}
									>
										Search
									</p>
									<h3
										className="mt-1 text-base font-semibold tracking-tight"
										style={{ color: "var(--color-text-primary)" }}
									>
										Find threads
									</h3>

									{/* Search inputs */}
									<div className="mt-3 grid gap-2 sm:grid-cols-2">
										{/* Query input */}
										<div
											className="col-span-2 flex items-center gap-2 rounded-lg border px-3 py-2 sm:col-span-1"
											style={{
												borderColor:
													"color-mix(in srgb, var(--color-accent) 28%, transparent)",
												background: "var(--color-bg-elevated)",
												boxShadow:
													"0 0 0 2px color-mix(in srgb, var(--color-accent) 12%, transparent)",
											}}
										>
											<Search
												size={13}
												style={{ color: "var(--color-accent-soft)" }}
											/>
											<span
												className="text-[12px]"
												style={{ color: "var(--color-text-primary)" }}
											>
												connection pool
											</span>
										</div>

										{/* Channel select */}
										<div
											className="flex items-center gap-2 rounded-lg border px-3 py-2"
											style={{
												borderColor: "var(--color-border-subtle)",
												background: "var(--color-bg-elevated)",
											}}
										>
											<Hash
												size={13}
												style={{ color: "var(--color-text-quiet)" }}
											/>
											<span
												className="flex-1 text-[12px]"
												style={{ color: "var(--color-text-quiet)" }}
											>
												All channels
											</span>
											<ChevronDown
												size={12}
												style={{ color: "var(--color-text-quiet)" }}
											/>
										</div>

										{/* Date from */}
										<div
											className="flex items-center gap-2 rounded-lg border px-3 py-2"
											style={{
												borderColor: "var(--color-border-subtle)",
												background: "var(--color-bg-elevated)",
											}}
										>
											<Calendar
												size={13}
												style={{ color: "var(--color-text-quiet)" }}
											/>
											<span
												className="text-[12px]"
												style={{ color: "var(--color-text-quiet)" }}
											>
												From
											</span>
										</div>

										{/* Date to */}
										<div
											className="flex items-center gap-2 rounded-lg border px-3 py-2"
											style={{
												borderColor: "var(--color-border-subtle)",
												background: "var(--color-bg-elevated)",
											}}
										>
											<Calendar
												size={13}
												style={{ color: "var(--color-text-quiet)" }}
											/>
											<span
												className="text-[12px]"
												style={{ color: "var(--color-text-quiet)" }}
											>
												To
											</span>
										</div>
									</div>

									{/* Sort */}
									<div className="mt-3 flex items-center gap-2">
										<div
											className="flex items-center gap-1 rounded-full border px-2.5 py-1 text-[10px] font-medium"
											style={{
												borderColor: "var(--color-border-subtle)",
												background: "color-mix(in srgb, white 3%, transparent)",
												color: "var(--color-text-quiet)",
											}}
										>
											<SlidersHorizontal size={10} />
											Sort
										</div>
										<div
											className="rounded-md px-2.5 py-1 text-[10px] font-medium"
											style={{
												background: "var(--color-accent)",
												color: "oklch(0.15 0.013 152)",
											}}
										>
											relevance
										</div>
										<div
											className="rounded-md border px-2.5 py-1 text-[10px] font-medium"
											style={{
												borderColor: "var(--color-border-strong)",
												background: "color-mix(in srgb, white 3%, transparent)",
												color: "var(--color-text-secondary)",
											}}
										>
											newest
										</div>
									</div>
								</div>

								{/* Results section */}
								<div
									className="relative mt-3 rounded-xl border p-4"
									style={{
										borderColor: "var(--color-border-subtle)",
										background:
											"color-mix(in srgb, var(--color-bg-panel) 94%, white 6%)",
										boxShadow:
											"0 8px 24px rgba(0,0,0,0.16), inset 0 1px 0 rgba(255,255,255,0.03)",
									}}
								>
									<div className="flex items-center justify-between">
										<p
											className="text-[9px] font-medium tracking-[0.16em] uppercase"
											style={{ color: "var(--color-accent-soft)" }}
										>
											Results
										</p>
										<span
											className="text-[10px]"
											style={{ color: "var(--color-text-quiet)" }}
										>
											2 threads
										</span>
									</div>

									<div className="mt-3 space-y-2">
										{RESULTS.map((r) => (
											<SearchResultCard key={r.title} result={r} />
										))}
									</div>

									<SearchScan visible={visible} />
								</div>
							</div>
						</MockBrowser>
					</div>
				</div>
			</div>
		</section>
	);
}

interface SearchResult {
	author: string;
	initials: string;
	tone: number;
	channel: string;
	channelTone: number;
	title: string;
	preview: string;
	highlight: string;
	replies: number;
	reactions: number;
	participants: number;
	time: string;
}

function SearchResultCard({ result }: { result: SearchResult }) {
	const channelToneClass = `channel-tone-${result.channelTone}`;

	// Highlight matching text
	const highlightPreview = (text: string, term: string) => {
		const idx = text.toLowerCase().indexOf(term.toLowerCase());
		if (idx === -1) return text;
		return (
			<>
				{text.slice(0, idx)}
				<mark
					style={{
						background:
							"color-mix(in srgb, var(--color-accent) 25%, transparent)",
						color: "var(--color-accent-strong)",
						borderRadius: "2px",
						padding: "0 2px",
					}}
				>
					{text.slice(idx, idx + term.length)}
				</mark>
				{text.slice(idx + term.length)}
			</>
		);
	};

	return (
		<div
			className="rounded-xl border p-3"
			style={{
				borderColor: "var(--color-border-subtle)",
				background: "color-mix(in srgb, var(--color-bg-panel) 94%, white 6%)",
				boxShadow:
					"0 8px 24px rgba(0,0,0,0.16), inset 0 1px 0 rgba(255,255,255,0.03)",
			}}
		>
			<div className="flex gap-2.5">
				<MockAvatar initials={result.initials} tone={result.tone} />
				<div className="min-w-0 flex-1">
					<div className="flex items-center gap-2">
						<span
							className="truncate text-[12px] font-medium"
							style={{ color: "var(--color-text-bright)" }}
						>
							{result.author}
						</span>
						<span
							className={`${channelToneClass} inline-flex items-center gap-0.5 rounded-lg border px-2 py-0.5 text-[9px] font-medium`}
							style={{
								boxShadow: "inset 0 1px 0 rgba(255,255,255,0.04)",
							}}
						>
							<Hash size={7} strokeWidth={2.5} />
							{result.channel}
						</span>
						<span
							className="ml-auto shrink-0 text-[10px]"
							style={{ color: "var(--color-text-quiet)" }}
						>
							{result.time}
						</span>
					</div>
					<p
						className="mt-0.5 text-[12px] font-medium"
						style={{ color: "var(--color-text-primary)" }}
					>
						{result.title}
					</p>
					<p
						className="mt-0.5 line-clamp-2 text-[11px] leading-relaxed"
						style={{ color: "var(--color-text-tertiary)" }}
					>
						{highlightPreview(result.preview, result.highlight)}
					</p>
					<div
						className="mt-1.5 flex items-center gap-2.5 text-[10px]"
						style={{ color: "var(--color-text-soft)" }}
					>
						<span className="inline-flex items-center gap-1">
							<MessageSquare size={10} />
							{result.replies}
						</span>
						{result.reactions > 0 && (
							<span className="inline-flex items-center gap-1">
								<Heart size={10} />
								{result.reactions}
							</span>
						)}
						{result.participants > 0 && (
							<span className="inline-flex items-center gap-1">
								<Users size={10} />
								{result.participants}
							</span>
						)}
					</div>
				</div>
			</div>
		</div>
	);
}
