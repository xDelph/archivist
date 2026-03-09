import { EmptyState } from "@/components/empty-state";
import { SectionCard } from "@/components/section-card";
import { ThreadCard } from "@/components/thread-card";
import { Button } from "@/components/ui/button";
import type { CatchUpChannel } from "@/lib/api";
import { formatCompactNumber } from "@/lib/format";
import { catchUpQueries } from "@/lib/queries";
import { cn } from "@/lib/utils";
import { useQuery } from "@tanstack/react-query";
import { useSearch } from "@tanstack/react-router";
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
		<div className="space-y-5">
			<section className="overflow-hidden rounded-(--radius-section) border border-(--color-border-subtle) bg-[linear-gradient(135deg,var(--color-bg-surface),var(--color-bg-base))] p-5 shadow-[0_16px_48px_oklch(0.05_0.02_220/0.4)] sm:p-6">
				<div className="flex flex-col gap-4 sm:flex-row sm:items-end sm:justify-between">
					<div>
						<p className="text-[0.62rem] font-medium uppercase tracking-[0.32em] text-(--color-accent-soft)">
							Home / Catch up
						</p>
						<h2 className="mt-3 max-w-2xl text-2xl font-semibold text-(--color-text-primary) sm:text-3xl">
							Start with what moved this week, not with the entire Slack
							firehose.
						</h2>
						<p className="mt-3 max-w-2xl text-sm leading-relaxed text-(--color-text-secondary)">
							Archivist groups recent public-channel threads into a catch-up
							feed so you can see the conversations that actually shifted.
						</p>
					</div>
					<div className="flex shrink-0 gap-3">
						<Button
							variant="secondary"
							onClick={() => {
								void dayQuery.refetch();
								void weekQuery.refetch();
							}}
						>
							<RefreshCcw className="size-4" />
							Refresh
						</Button>
					</div>
				</div>
				<div className="mt-5 flex flex-wrap gap-2">
					<ChannelPill
						isActive={activeFilter === "all"}
						label="All channels"
						href="/?channel="
					/>
					{availableChannels.map((channel) => (
						<ChannelPill
							key={channel.id}
							isActive={activeFilter === channel.id}
							label={channel.name || channel.id}
							href={`/?channel=${channel.id}`}
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
						actions={<Flame className="size-5 text-(--color-accent-soft)" />}
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
								icon={<Flame className="size-5" />}
							/>
						)}
					</SectionCard>

					<SectionCard
						eyebrow="Community highlights"
						title="Where people are gathering"
						description="Channels ranked by recent thread volume in the selected window."
						actions={
							<TrendingUp className="size-5 text-(--color-accent-soft)" />
						}
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
								description="As soon as the catch-up feed sees more thread summaries, this panel will call out the liveliest channels."
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
					className="h-32 animate-pulse rounded-(--radius-card) border border-(--color-border-subtle) bg-(--color-bg-surface)/40"
				/>
			))}
		</div>
	);
}

function ChannelPill({
	label,
	isActive,
	href,
}: {
	label: string;
	isActive: boolean;
	href: string;
}) {
	return (
		<a
			href={href}
			className={cn(
				"rounded-(--radius-pill) border px-4 py-2 text-xs font-medium transition-colors",
				isActive
					? "border-(--color-border-accent) bg-(--color-accent-soft)/12 text-(--color-accent-soft)"
					: "border-(--color-border-subtle) bg-(--color-bg-surface)/50 text-(--color-text-secondary) hover:border-(--color-border-default) hover:text-(--color-text-primary)",
			)}
		>
			{label}
		</a>
	);
}

interface ChannelHighlight {
	id: string;
	name: string;
	kind: string;
	threadCount: number;
	highlight: string;
}

function HighlightCard({ channel }: { channel: ChannelHighlight }) {
	return (
		<article className="rounded-(--radius-card) border border-(--color-border-subtle) bg-(--color-bg-base)/60 p-4">
			<div className="flex items-start justify-between gap-4">
				<div>
					<p className="text-[0.6rem] font-medium uppercase tracking-[0.22em] text-(--color-text-muted)">
						{channel.kind}
					</p>
					<h3 className="mt-1.5 text-base font-semibold text-(--color-text-primary)">
						# {channel.name}
					</h3>
				</div>
				<span className="rounded-(--radius-pill) border border-(--color-border-subtle) bg-(--color-bg-surface) px-3 py-1 text-xs tabular-nums text-(--color-text-secondary)">
					{formatCompactNumber(channel.threadCount)} threads
				</span>
			</div>
			<p className="mt-2.5 text-sm leading-relaxed text-(--color-text-secondary)">
				{channel.highlight}
			</p>
		</article>
	);
}

function filterChannels(channels: CatchUpChannel[], filter: string) {
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
