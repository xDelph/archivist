import { EmptyState } from "@/components/empty-state";
import { SectionCard } from "@/components/section-card";
import { ThreadCard } from "@/components/thread-card";
import { Button } from "@/components/ui/button";
import {
	type CatchUpChannel,
	type CatchUpThread,
	fetchCatchUp,
} from "@/lib/api";
import { formatCompactNumber } from "@/lib/format";
import { useQuery } from "@tanstack/react-query";
import { Flame, RefreshCcw, TrendingUp } from "lucide-react";
import { useState } from "react";

type ChannelFilter = "all" | string;

export function HomePage() {
	const [channelFilter, setChannelFilter] = useState<ChannelFilter>("all");
	const dayQuery = useQuery({
		queryKey: ["catch-up", "24h"],
		queryFn: () => fetchCatchUp("24h"),
	});
	const weekQuery = useQuery({
		queryKey: ["catch-up", "7d"],
		queryFn: () => fetchCatchUp("7d"),
	});

	const availableChannels = weekQuery.data?.channels ?? [];
	const filteredDayChannels = filterChannels(
		dayQuery.data?.channels ?? [],
		channelFilter,
	);
	const filteredWeekChannels = filterChannels(availableChannels, channelFilter);
	const trendingThreads = pickTrendingThreads(filteredWeekChannels);
	const highlights = pickChannelHighlights(filteredWeekChannels);

	return (
		<div className="space-y-5">
			<section className="overflow-hidden rounded-[2rem] border border-white/10 bg-[linear-gradient(135deg,rgba(16,33,52,0.95),rgba(11,18,32,0.88))] p-5 shadow-[0_24px_80px_rgba(5,12,24,0.35)] sm:p-6">
				<div className="flex flex-col gap-4 sm:flex-row sm:items-end sm:justify-between">
					<div>
						<p className="text-[0.68rem] uppercase tracking-[0.32em] text-[var(--accent-soft)]">
							Home / Catch up
						</p>
						<h2 className="mt-3 max-w-2xl text-3xl font-semibold text-white sm:text-4xl">
							Start with what moved this week, not with the entire Slack
							firehose.
						</h2>
						<p className="mt-3 max-w-2xl text-sm leading-6 text-slate-300 sm:text-base">
							Archivist groups recent public-channel threads into a catch-up
							feed so you can see the conversations that actually shifted.
						</p>
					</div>
					<div className="flex gap-3">
						<Button
							variant="secondary"
							onClick={() => {
								void dayQuery.refetch();
								void weekQuery.refetch();
							}}
						>
							<RefreshCcw className="mr-2 size-4" />
							Refresh
						</Button>
					</div>
				</div>
				<div className="mt-5 flex flex-wrap gap-2">
					<ChannelPill
						isActive={channelFilter === "all"}
						label="All channels"
						onClick={() => setChannelFilter("all")}
					/>
					{availableChannels.map((channel) => (
						<ChannelPill
							key={channel.id}
							isActive={channelFilter === channel.id}
							label={channel.name || channel.id}
							onClick={() => setChannelFilter(channel.id)}
						/>
					))}
				</div>
			</section>

			<div className="grid gap-5 xl:grid-cols-[1.35fr_0.95fr]">
				<div className="space-y-5">
					<SectionCard
						eyebrow="Since yesterday"
						title="Fresh threads"
						description="The last 24 hours of public-channel activity, ranked by actual movement."
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
						eyebrow="Since last week"
						title="Steady conversations"
						description="Longer-running threads that still matter when you zoom out past today."
					>
						<CatchUpSection
							channels={filteredWeekChannels}
							isPending={weekQuery.isPending}
							isError={weekQuery.isError}
							emptyTitle="No weekly catch-up yet"
							emptyDescription="Once the worker processes more public-channel history, longer windows will appear here."
						/>
					</SectionCard>
				</div>

				<div className="space-y-5">
					<SectionCard
						eyebrow="Trending"
						title="Threads with momentum"
						description="A simple blend of replies, participants, reactions, and files from the weekly window."
						actions={<Flame className="size-5 text-[var(--accent-soft)]" />}
					>
						{trendingThreads.length ? (
							<div className="space-y-3">
								{trendingThreads.map((thread) => (
									<ThreadCard
										key={thread.id}
										threadId={thread.id}
										channelName={thread.channelName}
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
						) : (
							<EmptyState
								title="Trending needs more history"
								description="This card fills in automatically as the worker accumulates public-channel thread summaries."
							/>
						)}
					</SectionCard>

					<SectionCard
						eyebrow="Community highlights"
						title="Where people are gathering"
						description="Channels ranked by recent thread volume in the selected window."
						actions={
							<TrendingUp className="size-5 text-[var(--accent-soft)]" />
						}
					>
						{highlights.length ? (
							<div className="space-y-3">
								{highlights.map((channel) => (
									<article
										key={channel.id}
										className="rounded-[1.5rem] border border-white/10 bg-slate-950/35 p-4"
									>
										<div className="flex items-start justify-between gap-4">
											<div>
												<p className="text-[0.65rem] uppercase tracking-[0.22em] text-slate-400">
													{channel.kind}
												</p>
												<h3 className="mt-2 text-lg font-semibold text-white">
													{channel.name}
												</h3>
											</div>
											<span className="rounded-full border border-white/10 bg-white/[0.05] px-3 py-1 text-xs text-slate-300">
												{formatCompactNumber(channel.threadCount)} threads
											</span>
										</div>
										<p className="mt-3 text-sm leading-6 text-slate-300">
											{channel.highlight}
										</p>
									</article>
								))}
							</div>
						) : (
							<EmptyState
								title="No channel highlights yet"
								description="As soon as the catch-up feed sees more thread summaries, this panel will call out the liveliest channels."
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
			{["day-skeleton-1", "day-skeleton-2", "day-skeleton-3"].map((key) => (
				<div
					key={key}
					className="h-36 animate-pulse rounded-[1.5rem] border border-white/8 bg-white/[0.04]"
				/>
			))}
		</div>
	);
}

function ChannelPill({
	label,
	isActive,
	onClick,
}: {
	label: string;
	isActive: boolean;
	onClick: () => void;
}) {
	return (
		<button
			type="button"
			onClick={onClick}
			className={
				isActive
					? "rounded-full border border-[var(--accent-soft)]/40 bg-[var(--accent-soft)]/14 px-4 py-2 text-xs uppercase tracking-[0.22em] text-[var(--accent-soft)]"
					: "rounded-full border border-white/10 bg-white/[0.04] px-4 py-2 text-xs uppercase tracking-[0.22em] text-slate-300 transition hover:border-white/20 hover:text-white"
			}
		>
			{label}
		</button>
	);
}

function filterChannels(channels: CatchUpChannel[], filter: ChannelFilter) {
	if (filter === "all") {
		return channels;
	}

	return channels.filter((channel) => channel.id === filter);
}

function pickTrendingThreads(channels: CatchUpChannel[]) {
	return channels
		.flatMap((channel) =>
			channel.threads.map((thread) => ({
				...thread,
				channelName: channel.name || channel.id,
				trendingScore:
					thread.reply_count * 3 +
					thread.participant_count * 2 +
					thread.reaction_count * 2 +
					thread.file_count,
			})),
		)
		.sort((left, right) => right.trendingScore - left.trendingScore)
		.slice(0, 4);
}

function pickChannelHighlights(channels: CatchUpChannel[]) {
	return channels
		.map((channel) => ({
			id: channel.id,
			name: channel.name || channel.id,
			kind: channel.kind.replace("_", " "),
			threadCount: channel.thread_count,
			highlight:
				channel.threads[0]?.title ||
				"Recent public-channel activity is available, but this channel has not produced a summary title yet.",
		}))
		.sort((left, right) => right.threadCount - left.threadCount)
		.slice(0, 4);
}
