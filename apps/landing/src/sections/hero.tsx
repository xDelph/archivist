import {
	Bookmark,
	Calendar,
	Clock,
	Flame,
	Hash,
	Heart,
	House,
	MessageSquare,
	Paperclip,
	Search,
	Star,
	Users,
} from "lucide-react";
import { useState } from "react";
import { FloatingPages } from "../components/floating-pages";
import { MockAvatar } from "../components/mock-avatar";
import { MockBrowser } from "../components/mock-browser";
import { MockPhone } from "../components/mock-phone";

type TabId = "starred" | "fresh" | "week" | "trending";

const TABS: {
	id: TabId;
	icon: typeof Star;
	label: string;
	shortLabel: string;
}[] = [
	{ id: "starred", icon: Star, label: "Starred", shortLabel: "Starred" },
	{ id: "fresh", icon: Clock, label: "Fresh (24h)", shortLabel: "Fresh" },
	{ id: "week", icon: Calendar, label: "This Week", shortLabel: "Week" },
	{ id: "trending", icon: Flame, label: "Trending", shortLabel: "Trending" },
];

const CHANNEL_PILLS = [
	{ name: "All channels", channel: null, tone: 0 },
	{ name: "engineering", channel: "engineering", tone: 1 },
	{ name: "product", channel: "product", tone: 2 },
	{ name: "incidents", channel: "incidents", tone: 3 },
	{ name: "design", channel: "design", tone: 4 },
];

interface ThreadData {
	channel: string;
	channelTone: number;
	author: string;
	authorTone: number;
	initials: string;
	title: string;
	preview: string;
	replies: number;
	reactions: number;
	participants: number;
	files: number;
	time: string;
	starred: boolean;
	tab: TabId;
}

const THREADS: ThreadData[] = [
	{
		channel: "engineering",
		channelTone: 1,
		author: "Sarah Chen",
		authorTone: 1,
		initials: "SC",
		title: "RFC: Migration to event-driven architecture",
		preview:
			"After reviewing our current sync patterns, I think we should move to an event-driven model for the ingest pipeline. Here's my proposal...",
		replies: 23,
		reactions: 14,
		participants: 8,
		files: 2,
		time: "2h ago",
		starred: true,
		tab: "fresh",
	},
	{
		channel: "product",
		channelTone: 2,
		author: "Alex Rivera",
		authorTone: 3,
		initials: "AR",
		title: "Q2 roadmap priorities and timeline",
		preview:
			"Sharing the updated roadmap for Q2. Key themes: search improvements, offline support, and onboarding.",
		replies: 15,
		reactions: 9,
		participants: 6,
		files: 1,
		time: "4h ago",
		starred: false,
		tab: "fresh",
	},
	{
		channel: "incidents",
		channelTone: 3,
		author: "Jamie Ortiz",
		authorTone: 5,
		initials: "JO",
		title: "Post-mortem: API latency spike March 12",
		preview:
			"Root cause identified: the connection pool was exhausted due to a missing timeout on the aggregation worker.",
		replies: 31,
		reactions: 7,
		participants: 11,
		files: 0,
		time: "6h ago",
		starred: false,
		tab: "fresh",
	},
	{
		channel: "engineering",
		channelTone: 1,
		author: "Sarah Chen",
		authorTone: 1,
		initials: "SC",
		title: "RFC: Migration to event-driven architecture",
		preview:
			"After reviewing our current sync patterns, I think we should move to an event-driven model for the ingest pipeline.",
		replies: 23,
		reactions: 14,
		participants: 8,
		files: 2,
		time: "2h ago",
		starred: true,
		tab: "starred",
	},
	{
		channel: "design",
		channelTone: 4,
		author: "Mia Foster",
		authorTone: 4,
		initials: "MF",
		title: "Design system v3 component audit",
		preview:
			"I've audited all 47 components against the new tokens. Here's the migration plan with priority tiers.",
		replies: 12,
		reactions: 8,
		participants: 4,
		files: 3,
		time: "1d ago",
		starred: true,
		tab: "starred",
	},
	{
		channel: "product",
		channelTone: 2,
		author: "Alex Rivera",
		authorTone: 3,
		initials: "AR",
		title: "Q2 roadmap priorities and timeline",
		preview:
			"Key themes: search improvements, offline support, and onboarding. Let's align before next sprint.",
		replies: 15,
		reactions: 9,
		participants: 6,
		files: 1,
		time: "3d ago",
		starred: false,
		tab: "week",
	},
	{
		channel: "engineering",
		channelTone: 1,
		author: "Liam Nakamura",
		authorTone: 2,
		initials: "LN",
		title: "Benchmark results: new search index",
		preview:
			"The tantivy-based index is 3.2x faster on p99 queries. Full results and methodology attached.",
		replies: 19,
		reactions: 22,
		participants: 7,
		files: 1,
		time: "5d ago",
		starred: false,
		tab: "week",
	},
	{
		channel: "incidents",
		channelTone: 3,
		author: "Jamie Ortiz",
		authorTone: 5,
		initials: "JO",
		title: "Post-mortem: API latency spike March 12",
		preview:
			"Root cause: connection pool exhaustion from missing timeout on aggregation worker.",
		replies: 31,
		reactions: 7,
		participants: 11,
		files: 0,
		time: "1w ago",
		starred: false,
		tab: "week",
	},
	{
		channel: "engineering",
		channelTone: 1,
		author: "Sarah Chen",
		authorTone: 1,
		initials: "SC",
		title: "RFC: Migration to event-driven architecture",
		preview:
			"After reviewing our current sync patterns, I think we should move to an event-driven model.",
		replies: 23,
		reactions: 14,
		participants: 8,
		files: 2,
		time: "2h ago",
		starred: true,
		tab: "trending",
	},
	{
		channel: "engineering",
		channelTone: 1,
		author: "Liam Nakamura",
		authorTone: 2,
		initials: "LN",
		title: "Benchmark results: new search index",
		preview:
			"The tantivy-based index is 3.2x faster on p99 queries. Full methodology attached.",
		replies: 19,
		reactions: 22,
		participants: 7,
		files: 1,
		time: "5d ago",
		starred: false,
		tab: "trending",
	},
	{
		channel: "product",
		channelTone: 2,
		author: "Alex Rivera",
		authorTone: 3,
		initials: "AR",
		title: "Q2 roadmap priorities and timeline",
		preview:
			"Sharing the updated roadmap for Q2. Key themes: search improvements, offline support.",
		replies: 15,
		reactions: 9,
		participants: 6,
		files: 1,
		time: "4h ago",
		starred: false,
		tab: "trending",
	},
];

function getVisibleThreads(tab: TabId, channel: string | null): ThreadData[] {
	return THREADS.filter(
		(t) => t.tab === tab && (channel === null || t.channel === channel),
	).slice(0, 3);
}

export function Hero() {
	const [activeTab, setActiveTab] = useState<TabId>("fresh");
	const [activeChannel, setActiveChannel] = useState<string | null>(null);
	const visibleThreads = getVisibleThreads(activeTab, activeChannel);

	return (
		<section className="relative flex flex-col items-center justify-center px-6 py-12">
			{/* Floating page decorations */}
			<FloatingPages />

			{/* Grid background */}
			<div
				className="pointer-events-none absolute inset-0"
				style={{
					backgroundImage: `
						linear-gradient(var(--hero-grid-line) 1px, transparent 1px),
						linear-gradient(90deg, var(--hero-grid-line) 1px, transparent 1px)
					`,
					backgroundSize: "64px 64px",
					maskImage:
						"radial-gradient(circle at 50% 30%, black 20%, transparent 65%)",
				}}
			/>

			{/* Glow orbs */}
			<div
				className="anim-pulse-glow pointer-events-none absolute top-[15%] left-1/2 h-[600px] w-[600px] -translate-x-1/2 -translate-y-1/2 rounded-full"
				style={{
					background:
						"radial-gradient(circle, color-mix(in srgb, var(--color-accent) 18%, transparent), transparent 70%)",
				}}
			/>
			<div
				className="anim-pulse-glow pointer-events-none absolute top-[25%] right-[10%] h-[300px] w-[300px] rounded-full"
				style={{
					background:
						"radial-gradient(circle, color-mix(in srgb, oklch(0.72 0.1 var(--info-hue)) 10%, transparent), transparent 70%)",
					animationDelay: "1.5s",
				}}
			/>

			<div className="relative mx-auto max-w-4xl text-center">
				<div className="anim-fade-up">
					<span
						className="mb-6 inline-flex items-center gap-2 rounded-full border px-4 py-1.5 text-xs font-medium tracking-wide uppercase"
						style={{
							borderColor:
								"color-mix(in srgb, var(--color-accent) 20%, transparent)",
							background:
								"color-mix(in srgb, var(--color-accent) 8%, transparent)",
							color: "var(--color-accent-soft)",
						}}
					>
						<span
							className="h-1.5 w-1.5 rounded-full"
							style={{
								background: "var(--color-accent)",
								boxShadow: "0 0 8px var(--color-accent)",
							}}
						/>
						Open source Slack archiver
					</span>
				</div>

				<h1
					className="anim-fade-up d-100 mt-8 text-5xl leading-[1.08] font-bold tracking-tight sm:text-6xl lg:text-7xl"
					style={{ color: "var(--color-text-primary)" }}
				>
					Your Slack threads,
					<br />
					<span
						className="anim-gradient"
						style={{
							backgroundImage:
								"linear-gradient(90deg, var(--color-accent), var(--color-accent-strong), oklch(0.75 0.13 var(--signal-hue)), var(--color-accent))",
							WebkitBackgroundClip: "text",
							WebkitTextFillColor: "transparent",
						}}
					>
						remembered forever
					</span>
				</h1>

				<p
					className="anim-fade-up d-200 mx-auto mt-6 max-w-2xl text-lg leading-relaxed sm:text-xl"
					style={{ color: "var(--color-text-secondary)" }}
				>
					Arkivist captures every Slack conversation automatically, generates AI
					summaries, and lets you search, catch up, and read offline. Your
					team&apos;s knowledge, permanently accessible.
				</p>
			</div>

			{/* Desktop + Mobile screenshots side by side */}
			<div className="anim-scale-in d-400 relative mx-auto mt-14 flex w-full max-w-6xl items-end justify-center gap-6">
				{/* Desktop screenshot */}
				<div className="w-full max-w-4xl">
					<MockBrowser>
						{/* App header */}
						<div
							className="flex items-center justify-between border-b px-5 py-2.5"
							style={{
								borderColor: "var(--color-border-subtle)",
								background:
									"color-mix(in srgb, var(--color-bg-deep) 94%, transparent)",
							}}
						>
							<div className="flex items-center gap-2">
								<div
									className="flex h-7 w-7 items-center justify-center rounded-md text-xs font-bold"
									style={{
										background: "var(--color-accent)",
										color: "var(--color-on-accent)",
										boxShadow: "var(--shadow-accent)",
									}}
								>
									R
								</div>
								<span
									className="text-sm font-semibold"
									style={{ color: "var(--color-text-bright)" }}
								>
									Arkivist
								</span>
							</div>
							<div className="hidden items-center gap-1 sm:flex">
								{[
									{ label: "Catch up", active: true },
									{ label: "Search", active: false },
									{ label: "Saved", active: false },
								].map((item) => (
									<div
										key={item.label}
										className="rounded-lg px-3 py-1.5 text-[11px] font-medium"
										style={{
											background: item.active
												? "color-mix(in srgb, var(--color-accent) 10%, var(--color-bg-panel))"
												: "transparent",
											color: item.active
												? "var(--color-text-primary)"
												: "var(--color-text-quiet)",
											border: item.active
												? "1px solid color-mix(in srgb, var(--color-accent) 20%, transparent)"
												: "1px solid transparent",
										}}
									>
										{item.label}
									</div>
								))}
							</div>
							<MockAvatar initials="TD" tone={1} size="sm" />
						</div>

						<div
							className="p-4"
							style={{
								background:
									"radial-gradient(circle at top center, color-mix(in srgb, var(--color-accent) 8%, transparent), transparent 40%), var(--color-bg-base)",
							}}
						>
							{/* Hero section card */}
							<div
								className="rounded-xl border p-5"
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
									Catch up
								</p>
								<h2
									className="mt-1 text-lg font-semibold tracking-tight sm:text-xl"
									style={{ color: "var(--color-text-primary)" }}
								>
									What&apos;s happening
								</h2>
								<div className="mt-3 flex flex-wrap gap-1.5">
									{CHANNEL_PILLS.map((ch) => {
										const isActive = activeChannel === ch.channel;
										return (
											<button
												key={ch.name}
												type="button"
												className="inline-flex cursor-pointer items-center gap-1 rounded-full border px-2.5 py-1 text-[10px] font-medium transition-all duration-200"
												style={{
													borderColor: isActive
														? "color-mix(in srgb, var(--color-accent) 30%, transparent)"
														: "var(--color-border-strong)",
													background: isActive
														? "color-mix(in srgb, var(--color-accent) 12%, transparent)"
														: "color-mix(in srgb, white 3%, transparent)",
													color: isActive
														? "var(--color-accent-soft)"
														: "var(--color-text-quiet)",
												}}
												onClick={() => setActiveChannel(ch.channel)}
											>
												{ch.channel !== null && (
													<Hash size={9} strokeWidth={2.5} />
												)}
												{ch.name}
											</button>
										);
									})}
								</div>
							</div>

							{/* Tabs */}
							<div
								className="mt-3 flex gap-0.5 rounded-xl border p-1"
								style={{
									borderColor: "var(--color-border-subtle)",
									background: "color-mix(in srgb, white 2%, transparent)",
								}}
							>
								{TABS.map((tab) => {
									const isActive = activeTab === tab.id;
									return (
										<button
											key={tab.id}
											type="button"
											className="flex flex-1 cursor-pointer items-center justify-center gap-1 rounded-lg py-2 text-[10px] font-medium transition-all duration-200"
											style={{
												background: isActive
													? "color-mix(in srgb, var(--color-accent) 14%, transparent)"
													: "transparent",
												color: isActive
													? "var(--color-accent-soft)"
													: "var(--color-text-quiet)",
												border: isActive
													? "1px solid color-mix(in srgb, var(--color-accent) 20%, transparent)"
													: "1px solid transparent",
												boxShadow: isActive ? "var(--mock-card-inset)" : "none",
											}}
											onClick={() => setActiveTab(tab.id)}
										>
											<tab.icon size={11} />
											<span className="hidden sm:inline">{tab.label}</span>
											<span className="sm:hidden">{tab.shortLabel}</span>
										</button>
									);
								})}
							</div>

							{/* Thread list — fixed height to prevent layout shift on tab/filter changes */}
							<div className="mt-3 min-h-[30rem] space-y-2 sm:min-h-[24rem]">
								{visibleThreads.length > 0 ? (
									visibleThreads.map((thread) => (
										<MockThreadCard
											key={`${thread.tab}-${thread.title}`}
											thread={thread}
										/>
									))
								) : (
									<div
										className="rounded-xl border p-8 text-center text-[12px]"
										style={{
											borderColor: "var(--color-border-subtle)",
											background:
												"color-mix(in srgb, var(--color-bg-panel) 94%, white 6%)",
											color: "var(--color-text-quiet)",
										}}
									>
										No threads in this view
									</div>
								)}
							</div>
						</div>

						{/* Mobile bottom nav */}
						<div
							className="flex border-t sm:hidden"
							style={{
								borderColor: "var(--color-border-subtle)",
								background:
									"color-mix(in srgb, var(--color-bg-deep) 88%, black 12%)",
							}}
						>
							{[
								{ icon: House, label: "Catch up", active: true },
								{ icon: Search, label: "Search", active: false },
								{ icon: Bookmark, label: "Saved", active: false },
							].map((item) => (
								<div
									key={item.label}
									className="flex flex-1 flex-col items-center gap-0.5 py-2.5"
									style={{
										color: item.active
											? "var(--color-accent-strong)"
											: "var(--color-text-quiet)",
									}}
								>
									<item.icon size={16} />
									<span className="text-[9px] font-medium">{item.label}</span>
								</div>
							))}
						</div>
					</MockBrowser>
				</div>

				{/* Mobile phone — overlapping from right */}
				<div className="anim-slide-right d-700 absolute -right-4 -bottom-8 z-10 hidden lg:block">
					<MockPhone>
						<div
							className="p-3"
							style={{
								background:
									"radial-gradient(circle at top center, color-mix(in srgb, var(--color-accent) 6%, transparent), transparent 50%), var(--color-bg-base)",
							}}
						>
							<div
								className="rounded-lg border p-3"
								style={{
									borderColor: "var(--color-border-subtle)",
									background:
										"color-mix(in srgb, var(--color-bg-panel) 94%, white 6%)",
									boxShadow: "var(--shadow-panel)",
								}}
							>
								<p
									className="text-[7px] font-medium tracking-[0.16em] uppercase"
									style={{ color: "var(--color-accent-soft)" }}
								>
									AI Summary
								</p>
								<p
									className="mt-1 text-[10px] font-semibold leading-snug"
									style={{ color: "var(--color-text-primary)" }}
								>
									RFC: Migration to event-driven architecture
								</p>
								<div className="mt-1.5 flex items-center gap-1.5">
									<MockAvatar initials="SC" tone={1} size="sm" />
									<span
										className="text-[8px] font-medium"
										style={{ color: "var(--color-text-bright)" }}
									>
										Sarah Chen
									</span>
									<span
										className="channel-tone-1 rounded border px-1.5 py-0.5 text-[7px] font-medium"
										style={{
											boxShadow: "var(--mock-card-inset)",
										}}
									>
										#engineering
									</span>
								</div>
								<div className="mt-2 flex gap-1">
									{["Answered", "event-driven", "architecture"].map(
										(tag, i) => (
											<span
												key={tag}
												className="rounded border px-1.5 py-0.5 text-[7px] font-medium"
												style={{
													borderColor:
														i === 0
															? "color-mix(in srgb, oklch(0.77 0.17 150) 38%, transparent)"
															: "var(--color-border-strong)",
													background:
														i === 0
															? "color-mix(in srgb, oklch(0.77 0.17 150) 14%, var(--color-bg-base))"
															: "color-mix(in srgb, white 5%, transparent)",
													color:
														i === 0
															? "color-mix(in srgb, oklch(0.77 0.17 150) 82%, white 18%)"
															: "var(--color-text-tertiary)",
												}}
											>
												{tag}
											</span>
										),
									)}
								</div>
							</div>

							{[
								{
									initials: "SC",
									tone: 1,
									name: "Sarah Chen",
									text: "After reviewing our sync patterns, I think we should move to event-driven for ingest.",
									time: "10:32",
								},
								{
									initials: "AR",
									tone: 3,
									name: "Alex Rivera",
									text: "Strong +1. This aligns with the Q2 scalability goals.",
									time: "10:45",
								},
							].map((msg) => (
								<div
									key={msg.time}
									className="mt-2 rounded-lg border p-2.5"
									style={{
										borderColor: "var(--color-border-subtle)",
										background:
											"color-mix(in srgb, var(--color-bg-panel) 94%, white 6%)",
									}}
								>
									<div className="flex gap-2">
										<MockAvatar
											initials={msg.initials}
											tone={msg.tone}
											size="sm"
										/>
										<div>
											<div className="flex items-center gap-1.5">
												<span
													className="text-[8px] font-medium"
													style={{
														color: "var(--color-text-bright)",
													}}
												>
													{msg.name}
												</span>
												<span
													className="text-[7px]"
													style={{
														color: "var(--color-text-quiet)",
													}}
												>
													{msg.time}
												</span>
											</div>
											<p
												className="mt-0.5 text-[8px] leading-relaxed"
												style={{
													color: "var(--color-text-tertiary)",
												}}
											>
												{msg.text}
											</p>
										</div>
									</div>
								</div>
							))}
						</div>

						{/* Mobile bottom nav */}
						<div
							className="mt-auto flex border-t"
							style={{
								borderColor: "var(--color-border-subtle)",
								background:
									"color-mix(in srgb, var(--color-bg-deep) 88%, black 12%)",
							}}
						>
							{[
								{ icon: House, label: "Catch up", active: false },
								{ icon: Search, label: "Search", active: false },
								{ icon: Bookmark, label: "Saved", active: true },
							].map((item) => (
								<div
									key={item.label}
									className="flex flex-1 flex-col items-center gap-0.5 py-2"
									style={{
										color: item.active
											? "var(--color-accent-strong)"
											: "var(--color-text-quiet)",
									}}
								>
									<item.icon size={14} />
									<span className="text-[7px] font-medium">{item.label}</span>
								</div>
							))}
						</div>
					</MockPhone>
				</div>
			</div>
		</section>
	);
}

function MockThreadCard({ thread }: { thread: ThreadData }) {
	const channelToneClass = `channel-tone-${thread.channelTone}`;

	return (
		<div
			className="rounded-xl border p-4 transition-[border-color,box-shadow] duration-200 hover:border-[color-mix(in_srgb,var(--color-accent)_16%,var(--color-border-subtle))]"
			style={{
				borderColor: "var(--color-border-subtle)",
				background: "color-mix(in srgb, var(--color-bg-panel) 94%, white 6%)",
				boxShadow: "0 8px 24px rgba(0,0,0,0.16), var(--mock-card-inset)",
			}}
		>
			<div className="flex gap-3">
				<MockAvatar initials={thread.initials} tone={thread.authorTone} />
				<div className="min-w-0 flex-1">
					<div className="flex items-center gap-2">
						<span
							className="truncate text-[13px] font-medium"
							style={{ color: "var(--color-text-bright)" }}
						>
							{thread.author}
						</span>
						<span
							className={`${channelToneClass} inline-flex items-center gap-0.5 rounded-lg border px-2 py-0.5 text-[10px] font-medium`}
							style={{
								boxShadow: "var(--mock-card-inset)",
							}}
						>
							<Hash size={8} strokeWidth={2.5} />
							{thread.channel}
						</span>
						{thread.starred && (
							<span
								className="inline-flex items-center gap-0.5 rounded-full border px-2 py-0.5 text-[9px] font-medium tracking-wide"
								style={{
									borderColor:
										"color-mix(in srgb, var(--color-accent) 30%, transparent)",
									background:
										"color-mix(in srgb, var(--color-accent) 10%, transparent)",
									color: "var(--color-accent-soft)",
								}}
							>
								<Star size={8} fill="currentColor" />
								Starred
							</span>
						)}
						<span
							className="ml-auto hidden shrink-0 text-[10px] sm:inline"
							style={{ color: "var(--color-text-quiet)" }}
						>
							{thread.time}
						</span>
					</div>
					<p
						className="mt-1 text-[13px] font-medium leading-snug"
						style={{ color: "var(--color-text-primary)" }}
					>
						{thread.title}
					</p>
					<p
						className="mt-0.5 line-clamp-2 text-[12px] leading-relaxed"
						style={{ color: "var(--color-text-tertiary)" }}
					>
						{thread.preview}
					</p>
					<div
						className="mt-2 flex items-center gap-3 text-[10px]"
						style={{ color: "var(--color-text-soft)" }}
					>
						<span className="inline-flex items-center gap-1">
							<MessageSquare size={11} />
							{thread.replies}
						</span>
						{thread.reactions > 0 && (
							<span className="inline-flex items-center gap-1">
								<Heart size={11} />
								{thread.reactions}
							</span>
						)}
						{thread.participants > 0 && (
							<span className="inline-flex items-center gap-1">
								<Users size={11} />
								{thread.participants}
							</span>
						)}
						{thread.files > 0 && (
							<span className="inline-flex items-center gap-1">
								<Paperclip size={11} />
								{thread.files}
							</span>
						)}
						<span className="ml-auto">
							<Bookmark
								size={12}
								style={{ color: "var(--color-text-quiet)" }}
							/>
						</span>
					</div>
				</div>
			</div>
		</div>
	);
}
