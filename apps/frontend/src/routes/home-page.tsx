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
import { Flame, Hash, RefreshCcw, TrendingUp } from "lucide-react";

export function HomePage() {
	const { channel: channelFilter } = useSearch({ from: "/app/" });
	const dayQuery = useQuery(catchUpQueries.window("24h"));
	const weekQuery = useQuery(catchUpQueries.window("7d"));

	const availableChannels = weekQuery.data?.channels ?? [];
	const activeFilter = channelFilter ?? "all";
	const filteredDayChannels = filterChannels(
		dayQuery.data?.channels ?? [],
		activeFilter,
	);
	const filteredWeekChannels = filterChannels(availableChannels, activeFilter);
	const trendingThreads = pickTrendingThreads(filteredWeekChannels);
	const highlights = pickChannelHighlights(filteredWeekChannels);

	return (
		<div className="space-y-6">
			<section className="rounded-[1.9rem] border border-white/8 bg-[#090b0d] px-5 py-5 shadow-[0_24px_80px_rgba(0,0,0,0.42)] sm:px-6">
				<div className="flex flex-col gap-5 lg:flex-row lg:items-start lg:justify-between">
					<div className="max-w-4xl">
						<p className="text-[0.72rem] font-medium uppercase tracking-[0.32em] text-[#20cb74]">
							Public-channel catch-up
						</p>
						<h1 className="mt-4 text-3xl font-semibold tracking-tight text-white sm:text-[2.8rem]">
							Catch up without reopening the full Slack firehose.
						</h1>
						<p className="mt-4 max-w-3xl text-base leading-8 text-[#92949b]">
							The catch-up feed stays the same. This pass only tightens the
							presentation so threads read closer to the original Archivist:
							darker chrome, clearer identity, better channel pills, and denser
							conversation cards.
						</p>
					</div>
					<div className="flex shrink-0 items-center gap-3">
						<Button
							variant="secondary"
							onClick={() => {
								void dayQuery.refetch();
								void weekQuery.refetch();
							}}
							className="border-white/10 bg-white/4 text-white hover:border-[#1fc86f]/30 hover:bg-white/7"
						>
							<RefreshCcw className="size-4" />
							Refresh
						</Button>
					</div>
				</div>

				<div className="mt-6 flex flex-wrap gap-2.5">
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

			<div className="grid gap-6 xl:grid-cols-[minmax(0,1.45fr)_minmax(320px,0.85fr)]">
				<div className="space-y-6">
					<SectionCard
						eyebrow="Last 24 hours"
						title="Fresh threads"
						description="Recent public-channel movement, shown with the same catch-up logic but a cleaner surface."
					>
						<CatchUpSection
							channels={filteredDayChannels}
							isPending={dayQuery.isPending}
							isError={dayQuery.isError}
							emptyTitle="No recent public activity"
							emptyDescription="Nothing crossed the 24-hour threshold for the selected channels."
						/>
					</SectionCard>

					<SectionCard
						eyebrow="This week"
						title="Steady conversations"
						description="The longer weekly window stays visible so you can scan ongoing threads without changing the feature set."
					>
						<CatchUpSection
							channels={filteredWeekChannels}
							isPending={weekQuery.isPending}
							isError={weekQuery.isError}
							emptyTitle="No weekly catch-up yet"
							emptyDescription="Once the worker processes more history, longer windows will show up here."
						/>
					</SectionCard>
				</div>

				<div className="space-y-6">
					<SectionCard
						eyebrow="Trending"
						title="Threads with momentum"
						description="A secondary view of the same weekly data. Replies, participants, reactions, and files stay visible without turning the main feed into a ranked list."
						actions={<Flame className="size-5 text-[#20cb74]" />}
					>
						{trendingThreads.length ? (
							<div className="space-y-3">
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

					<SectionCard
						eyebrow="Channel activity"
						title="Where people are gathering"
						description="The channel summary stays the same, but the visual treatment now matches the darker original Archivist surface."
						actions={<TrendingUp className="size-5 text-[#20cb74]" />}
					>
						{highlights.length ? (
							<div className="space-y-3">
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
				</div>
			</div>
		</div>
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
		<div className="space-y-3">
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
		<div className="space-y-3">
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
				"rounded-full border px-4 py-2 text-sm font-medium transition-colors",
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
		<div className="rounded-[1.35rem] border border-white/8 bg-[#07090a] px-4 py-4">
			<div className="flex items-start justify-between gap-3">
				<div className="min-w-0">
					<p className="text-[0.72rem] font-medium uppercase tracking-[0.28em] text-[#71747d]">
						Channel
					</p>
					<p className="mt-2 truncate text-lg font-semibold text-white">
						#{channel.name || channel.id}
					</p>
				</div>
				<div className="rounded-full border border-[#1fc86f]/25 bg-[#132118] px-3 py-1 text-sm font-medium text-[#29d779]">
					{formatCompactNumber(channel.thread_count)}
				</div>
			</div>
			<div className="mt-4 h-2 overflow-hidden rounded-full bg-white/[0.05]">
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
