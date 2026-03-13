import { EmptyState } from "@/components/empty-state";
import { SectionCard } from "@/components/section-card";
import { ThreadCard } from "@/components/thread-card";
import { Button } from "@/components/ui/button";
import type { CatchUpChannel } from "@/lib/api";
import { formatCompactNumber } from "@/lib/format";
import { catchUpQueries } from "@/lib/queries";
import { cn } from "@/lib/utils";
import { useQuery } from "@tanstack/react-query";
import { Link, useSearch } from "@tanstack/react-router";
import {
	Calendar,
	Clock,
	Flame,
	Hash,
	RefreshCcw,
	TrendingUp,
} from "lucide-react";
import { type ReactNode, useState } from "react";

export function HomePage() {
	const { channel: channelFilter } = useSearch({ from: "/app/" });
	const dayQuery = useQuery(catchUpQueries.window("24h"));
	const weekQuery = useQuery(catchUpQueries.window("7d"));

	const availableChannels = [...(weekQuery.data?.channels ?? [])].sort(
		(a, b) => {
			const nameA = a.name || a.id;
			const nameB = b.name || b.id;
			return nameA.localeCompare(nameB);
		},
	);
	const activeFilter = channelFilter ?? "all";
	const filteredDayChannels = filterChannels(
		dayQuery.data?.channels ?? [],
		activeFilter,
	);
	const filteredWeekChannels = filterChannels(availableChannels, activeFilter);
	const trendingThreads = pickTrendingThreads(filteredWeekChannels);
	const highlights = pickChannelHighlights(filteredWeekChannels);

	const [activeTab, setActiveTab] = useState<
		"fresh" | "steady" | "trending" | "channels"
	>("fresh");

	return (
		<div className="mx-auto w-full max-w-3xl space-y-4 pb-8">
			<section className="rounded-[0.82rem] border border-white/8 bg-[#07090b] px-3 py-3 shadow-[0_8px_24px_rgba(0,0,0,0.16)] sm:px-3.5">
				<div className="flex flex-col gap-2 lg:flex-row lg:items-center lg:justify-between">
					<div className="max-w-3xl">
						<p className="text-[0.58rem] font-medium uppercase tracking-[0.24em] text-[#20cb74]">
							Public-channel catch-up
						</p>
						<h1 className="mt-1.5 text-[1.18rem] font-semibold tracking-tight text-white sm:text-[1.38rem]">
							Catch up without reopening the full Slack firehose.
						</h1>
						<p className="mt-1 max-w-2xl text-[0.72rem] leading-5 text-[#92949b]">
							Public threads, same data model, darker Archivist chrome.
						</p>
					</div>
					<div className="flex shrink-0 items-center gap-3">
						<Button
							variant="secondary"
							onClick={() => {
								void dayQuery.refetch();
								void weekQuery.refetch();
							}}
							className="rounded-lg border-white/10 bg-white/[0.04] px-3 py-2 text-[0.78rem] text-white hover:border-[#1fc86f]/30 hover:bg-white/[0.07]"
						>
							<RefreshCcw className="size-3.5" />
							Refresh
						</Button>
					</div>
				</div>

				<div className="mt-3 flex flex-wrap gap-1.5">
					<ChannelPill
						isActive={activeFilter === "all"}
						label="All channels"
						search={{ channel: undefined }}
					/>
					{availableChannels.map((channel) => (
						<ChannelPill
							key={channel.id}
							isActive={activeFilter === channel.id}
							label={channel.name || channel.id}
							search={{ channel: channel.id }}
						/>
					))}
				</div>
			</section>

			<div className="flex w-full items-center gap-1 rounded-[0.82rem] border border-white/8 bg-[#07090b] p-1.5 shadow-[0_8px_24px_rgba(0,0,0,0.16)] sm:gap-2 sm:p-2">
				<TabButton
					isActive={activeTab === "fresh"}
					onClick={() => setActiveTab("fresh")}
					icon={<Clock className="size-3.5 shrink-0 sm:size-4" />}
					label="Fresh (24h)"
				/>
				<TabButton
					isActive={activeTab === "steady"}
					onClick={() => setActiveTab("steady")}
					icon={<Calendar className="size-3.5 shrink-0 sm:size-4" />}
					label="This Week"
				/>
				<TabButton
					isActive={activeTab === "trending"}
					onClick={() => setActiveTab("trending")}
					icon={<Flame className="size-3.5 shrink-0 sm:size-4" />}
					label="Trending"
				/>
				<TabButton
					isActive={activeTab === "channels"}
					onClick={() => setActiveTab("channels")}
					icon={<TrendingUp className="size-3.5 shrink-0 sm:size-4" />}
					label="Channels"
				/>
			</div>

			<div className="mt-2">
				{activeTab === "fresh" && (
					<SectionCard eyebrow="Last 24 hours" title="Fresh threads">
						<CatchUpSection
							channels={filteredDayChannels}
							isPending={dayQuery.isPending}
							isError={dayQuery.isError}
							emptyTitle="No recent public activity"
							emptyDescription="Nothing crossed the 24-hour threshold for the selected channels."
						/>
					</SectionCard>
				)}

				{activeTab === "steady" && (
					<SectionCard eyebrow="This week" title="Steady conversations">
						<CatchUpSection
							channels={filteredWeekChannels}
							isPending={weekQuery.isPending}
							isError={weekQuery.isError}
							emptyTitle="No weekly catch-up yet"
							emptyDescription="Once the worker processes more history, longer windows will show up here."
						/>
					</SectionCard>
				)}

				{activeTab === "trending" && (
					<SectionCard
						eyebrow="Trending"
						title="Threads with momentum"
						actions={<Flame className="size-5 text-[#20cb74]" />}
					>
						{trendingThreads.length ? (
							<div className="space-y-2.5">
								{trendingThreads.map((thread) => (
									<ThreadCard
										key={thread.id}
										threadId={thread.id}
										channelName={thread.channelName}
										author={thread.author}
										title={thread.title}
										preview={thread.preview}
										lastActivityTs={thread.last_activity_ts}
										replyCount={thread.reply_count}
										participantCount={thread.participant_count}
										reactionCount={thread.reaction_count}
										fileCount={thread.file_count}
										className="bg-[#07090a]"
									/>
								))}
							</div>
						) : (
							<EmptyState
								title="Trending needs more history"
								description="This panel fills in automatically as the worker accumulates more public-channel thread summaries."
								icon={<Flame className="size-5" />}
							/>
						)}
					</SectionCard>
				)}

				{activeTab === "channels" && (
					<SectionCard
						eyebrow="Channel activity"
						title="Where people are gathering"
						actions={<TrendingUp className="size-5 text-[#20cb74]" />}
					>
						{highlights.length ? (
							<div className="space-y-2.5">
								{highlights.map((channel) => (
									<HighlightCard key={channel.id} channel={channel} />
								))}
							</div>
						) : (
							<EmptyState
								title="No channel highlights yet"
								description="More public-channel thread summaries will populate this panel automatically."
								icon={<Hash className="size-5" />}
							/>
						)}
					</SectionCard>
				)}
			</div>
		</div>
	);
}

function TabButton({
	isActive,
	onClick,
	icon,
	label,
}: {
	isActive: boolean;
	onClick: () => void;
	icon: ReactNode;
	label: string;
}) {
	return (
		<button
			type="button"
			onClick={onClick}
			className={cn(
				"flex min-w-0 flex-1 items-center justify-center gap-1 rounded-[0.6rem] px-1 py-1.5 text-[0.62rem] font-medium transition-colors sm:gap-2 sm:px-3 sm:py-2 sm:text-[0.8rem] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[#20cb74]/40",
				isActive
					? "bg-[#1fc86f]/15 text-[#29d779]"
					: "text-[#9da0a8] hover:bg-white/[0.05] hover:text-white",
			)}
		>
			{icon}
			<span className="truncate">{label}</span>
		</button>
	);
}

function CatchUpSection({
	channels,
	isPending,
	isError,
	emptyTitle,
	emptyDescription,
}: {
	channels: CatchUpChannel[];
	isPending: boolean;
	isError: boolean;
	emptyTitle: string;
	emptyDescription: string;
}) {
	if (isPending) {
		return <LoadingGrid />;
	}

	if (isError) {
		return (
			<EmptyState
				title="The catch-up feed is unavailable"
				description="The API route is reachable, but the current query failed. Retry once the local API is healthy again."
			/>
		);
	}

	const threads = channels.flatMap((channel) =>
		channel.threads.map((thread) => ({
			...thread,
			channelName: channel.name || channel.id,
		})),
	);

	if (!threads.length) {
		return <EmptyState title={emptyTitle} description={emptyDescription} />;
	}

	return (
		<div className="space-y-2.5">
			{threads.map((thread) => (
				<ThreadCard
					key={thread.id}
					threadId={thread.id}
					channelName={thread.channelName}
					author={thread.author}
					title={thread.title}
					preview={thread.preview}
					lastActivityTs={thread.last_activity_ts}
					replyCount={thread.reply_count}
					participantCount={thread.participant_count}
					reactionCount={thread.reaction_count}
					fileCount={thread.file_count}
				/>
			))}
		</div>
	);
}

function LoadingGrid() {
	return (
		<div className="space-y-2.5">
			{["skel-a", "skel-b", "skel-c"].map((id) => (
				<div
					key={id}
					className="h-36 animate-pulse rounded-[1.45rem] border border-white/8 bg-white/[0.03]"
				/>
			))}
		</div>
	);
}

function ChannelPill({
	label,
	isActive,
	search,
}: {
	label: string;
	isActive: boolean;
	search: { channel?: string };
}) {
	return (
		<Link
			to="/"
			search={search}
			className={cn(
				"rounded-[0.65rem] border px-2.5 py-1 text-[0.7rem] font-medium transition-colors",
				isActive
					? "border-[#1fc86f]/35 bg-[#122318] text-[#29d779]"
					: "border-white/10 bg-white/[0.03] text-[#9da0a8] hover:border-white/16 hover:bg-white/[0.05] hover:text-white",
			)}
		>
			{label}
		</Link>
	);
}

function HighlightCard({ channel }: { channel: CatchUpChannel }) {
	return (
		<div className="rounded-[0.9rem] border border-white/8 bg-[#07090a] px-3 py-3">
			<div className="flex items-start justify-between gap-3">
				<div className="min-w-0">
					<p className="text-[0.62rem] font-medium uppercase tracking-[0.22em] text-[#71747d]">
						Channel
					</p>
					<p className="mt-1.5 truncate text-[0.95rem] font-semibold text-white">
						#{channel.name || channel.id}
					</p>
				</div>
				<div className="rounded-full border border-[#1fc86f]/25 bg-[#132118] px-2.5 py-0.5 text-[0.72rem] font-medium text-[#29d779]">
					{formatCompactNumber(channel.thread_count)}
				</div>
			</div>
			<div className="mt-3 h-1.5 overflow-hidden rounded-full bg-white/[0.05]">
				<div
					className="h-full rounded-full bg-linear-to-r from-[#14a64e] to-[#29d779]"
					style={{
						width: `${Math.max(12, Math.min(100, channel.thread_count * 12))}%`,
					}}
				/>
			</div>
		</div>
	);
}

function filterChannels(channels: CatchUpChannel[], activeFilter: string) {
	if (activeFilter === "all") {
		return channels;
	}

	return channels.filter((channel) => channel.id === activeFilter);
}

function pickTrendingThreads(channels: CatchUpChannel[]) {
	return channels
		.flatMap((channel) =>
			channel.threads.map((thread) => ({
				...thread,
				channelName: channel.name || channel.id,
				movement:
					thread.reply_count * 3 +
					thread.participant_count * 2 +
					thread.reaction_count * 2 +
					thread.file_count * 4,
			})),
		)
		.sort((left, right) => right.movement - left.movement)
		.slice(0, 4);
}

function pickChannelHighlights(channels: CatchUpChannel[]) {
	return [...channels]
		.sort((left, right) => right.thread_count - left.thread_count)
		.slice(0, 5);
}
