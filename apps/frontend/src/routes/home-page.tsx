import { EmptyState } from "@/components/empty-state";
import { CardSkeletonList, QueryState } from "@/components/query-state";
import { SectionCard } from "@/components/section-card";
import { ThreadCard } from "@/components/thread-card";
import { Button } from "@/components/ui/button";
import { SegmentedTabs } from "@/components/ui/segmented-tabs";
import type { CatchUpChannel } from "@/lib/api";
import { flattenCatchUpThreads } from "@/lib/catch-up";
import { formatCompactNumber } from "@/lib/format";
import { catchUpQueries } from "@/lib/queries";
import { threadCardDataFromCatchUpThread } from "@/lib/thread-card-props";
import { cn } from "@/lib/utils";
import { useQuery } from "@tanstack/react-query";
import { Link, useSearch } from "@tanstack/react-router";
import { Calendar, Clock, Flame, Hash, TrendingUp } from "lucide-react";
import { useState } from "react";

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
	const tabItems = [
		{
			key: "fresh",
			label: "Fresh (24h)",
			icon: <Clock className="size-3.5 shrink-0 sm:size-4" />,
		},
		{
			key: "steady",
			label: "This Week",
			icon: <Calendar className="size-3.5 shrink-0 sm:size-4" />,
		},
		{
			key: "trending",
			label: "Trending",
			icon: <Flame className="size-3.5 shrink-0 sm:size-4" />,
		},
		{
			key: "channels",
			label: "Channels",
			icon: <TrendingUp className="size-3.5 shrink-0 sm:size-4" />,
		},
	] as const;

	return (
		<div className="space-y-4 ">
			<section className="surface-panel surface-panel-soft px-3 py-3 sm:px-3.5">
				<div className="flex flex-col gap-2 lg:flex-row lg:items-center lg:justify-between">
					<div className="max-w-3xl">
						<p className="text-eyebrow text-[0.58rem] font-medium uppercase tracking-[0.24em]">
							Public-channel catch-up
						</p>
						<h1 className="mt-1.5 text-[1.18rem] font-semibold tracking-tight text-white sm:text-[1.38rem]">
							Catch up without reopening the full Slack firehose.
						</h1>
						<p className="text-copy-muted mt-1 max-w-2xl text-[0.72rem] leading-5">
							Public threads, same data model, darker Archivist chrome.
						</p>
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

			<SegmentedTabs
				items={tabItems}
				value={activeTab}
				onChange={setActiveTab}
			/>

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
						actions={<Flame className="text-eyebrow size-5" />}
					>
						{trendingThreads.length ? (
							<div className="space-y-2.5">
								{trendingThreads.map((thread) => (
									<ThreadCard
										key={thread.id}
										{...threadCardDataFromCatchUpThread(
											thread,
											thread.channelName,
										)}
										className="bg-(--color-bg-panel)"
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
						actions={<TrendingUp className="text-eyebrow size-5" />}
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
	const sortedThreads = flattenCatchUpThreads(channels);

	return (
		<QueryState
			isPending={isPending}
			isError={isError}
			isEmpty={sortedThreads.length === 0}
			loading={<CardSkeletonList />}
			error={
				<EmptyState
					title="The catch-up feed is unavailable"
					description="The API route is reachable, but the current query failed. Retry once the local API is healthy again."
				/>
			}
			empty={<EmptyState title={emptyTitle} description={emptyDescription} />}
		>
			<div className="space-y-2.5">
				{sortedThreads.map((thread) => (
					<ThreadCard
						key={thread.id}
						{...threadCardDataFromCatchUpThread(thread, thread.channelName)}
					/>
				))}
			</div>
		</QueryState>
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
					? "border-(--color-border-accent) bg-(--color-accent)/12 text-(--color-accent-soft)"
					: "surface-ghost surface-ghost-hover text-(--color-text-secondary)",
			)}
		>
			{label}
		</Link>
	);
}

function HighlightCard({ channel }: { channel: CatchUpChannel }) {
	return (
		<div className="surface-subpanel px-3 py-3">
			<div className="flex items-start justify-between gap-3">
				<div className="min-w-0">
					<p className="text-copy-quiet text-[0.62rem] font-medium uppercase tracking-[0.22em]">
						Channel
					</p>
					<p className="mt-1.5 truncate text-[0.95rem] font-semibold text-white">
						#{channel.name || channel.id}
					</p>
				</div>
				<div className="accent-pill rounded-full px-2.5 py-0.5 text-[0.72rem] font-medium">
					{formatCompactNumber(channel.thread_count)}
				</div>
			</div>
			<div className="progress-track mt-3 h-1.5 overflow-hidden rounded-full">
				<div
					className="h-full rounded-full bg-linear-to-r from-(--color-accent) to-(--color-accent-strong)"
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
