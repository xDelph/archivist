import { EmptyState } from "@/components/empty-state";
import { CardSkeletonList, QueryState } from "@/components/query-state";
import { SectionCard } from "@/components/section-card";
import { ThreadCard } from "@/components/thread-card";
import { SegmentedTabs } from "@/components/ui/segmented-tabs";
import type { CatchUpChannel, CatchUpSort, CatchUpWindow } from "@/lib/api";
import { fetchCatchUp } from "@/lib/api";
import {
	CATCH_UP_PAGE_SIZE,
	flattenCatchUpPages,
	getCatchUpChannelLabel,
	getCatchUpThreadChannelName,
	pickChannelHighlights,
} from "@/lib/catch-up";
import { formatCompactNumber } from "@/lib/format";
import {
	type HomeTab,
	normalizeHomeTab,
	toHomeTabSearch,
} from "@/lib/home-tabs";
import { catchUpQueries } from "@/lib/queries";
import { threadCardDataFromCatchUpThread } from "@/lib/thread-card-props";
import { cn } from "@/lib/utils";
import { useInfiniteQuery, useQuery } from "@tanstack/react-query";
import { Link, useNavigate, useSearch } from "@tanstack/react-router";
import { Calendar, Clock, Flame, Hash, TrendingUp } from "lucide-react";
import { useEffect, useRef } from "react";

export function HomePage() {
	const { channel: channelFilter, tab } = useSearch({ from: "/app/" });
	const navigate = useNavigate();
	const activeTab = normalizeHomeTab(tab);
	const activeFilter = channelFilter ?? "all";
	const activeChannelId = activeFilter === "all" ? undefined : activeFilter;
	const overviewQuery = useQuery(catchUpQueries.summary("7d"));
	const dayFeedQuery = useCatchUpFeed({
		window: "24h",
		channelId: activeChannelId,
		enabled: activeTab === "fresh",
	});
	const weekFeedQuery = useCatchUpFeed({
		window: "7d",
		channelId: activeChannelId,
		enabled: activeTab === "steady",
	});
	const trendingFeedQuery = useCatchUpFeed({
		window: "7d",
		channelId: activeChannelId,
		sort: "trending",
		enabled: activeTab === "trending",
	});

	const availableChannels = [...(overviewQuery.data?.channels ?? [])].sort(
		(left, right) =>
			getCatchUpChannelLabel(left).localeCompare(getCatchUpChannelLabel(right)),
	);
	const highlights = pickChannelHighlights(
		filterChannels(overviewQuery.data?.channels ?? [], activeFilter),
	);
	const tabItems = [
		{
			key: "fresh",
			label: "Fresh (24h)",
			shortLabel: "Fresh",
			icon: <Clock className="size-3.5 shrink-0 sm:size-4" />,
		},
		{
			key: "steady",
			label: "This Week",
			shortLabel: "Week",
			icon: <Calendar className="size-3.5 shrink-0 sm:size-4" />,
		},
		{
			key: "trending",
			label: "Trending",
			shortLabel: "Trend",
			icon: <Flame className="size-3.5 shrink-0 sm:size-4" />,
		},
		{
			key: "channels",
			label: "Channels",
			shortLabel: "Rooms",
			icon: <TrendingUp className="size-3.5 shrink-0 sm:size-4" />,
		},
	] as const;

	return (
		<div className="space-y-4">
			<section className="surface-panel surface-panel-soft relative overflow-hidden px-4 py-4 sm:px-5 sm:py-5">
				<div className="pointer-events-none absolute inset-0 bg-[radial-gradient(circle_at_top_left,color-mix(in_srgb,var(--color-accent)_14%,transparent),transparent_42%),radial-gradient(circle_at_88%_16%,color-mix(in_srgb,var(--color-signal)_10%,transparent),transparent_24%)]" />
				<div className="flex flex-col gap-3 lg:flex-row lg:items-end lg:justify-between">
					<div className="max-w-3xl">
						<p className="text-eyebrow text-[0.68rem] font-medium uppercase tracking-[0.16em] sm:text-[0.74rem]">
							Public-channel catch-up
						</p>
						<h1 className="mt-2 text-[clamp(1.95rem,8vw,2.3rem)] font-semibold leading-[1.08] tracking-tight text-white">
							Catch up on the go.
						</h1>
					</div>
				</div>

				<div className="relative mt-4 flex flex-wrap gap-2">
					<ChannelPill
						isActive={activeFilter === "all"}
						label="All channels"
						search={{
							channel: undefined,
							tab: toHomeTabSearch(activeTab),
						}}
					/>
					{availableChannels.map((channel) => (
						<ChannelPill
							key={channel.id}
							isActive={activeFilter === channel.id}
							label={getCatchUpChannelLabel(channel)}
							search={{
								channel: channel.id,
								tab: toHomeTabSearch(activeTab),
							}}
						/>
					))}
				</div>
			</section>

			<SegmentedTabs
				items={tabItems}
				value={activeTab}
				onChange={(nextTab) => {
					void navigate({
						to: "/",
						search: {
							channel: channelFilter,
							tab: toHomeTabSearch(nextTab),
						},
						replace: false,
					});
				}}
			/>

			<div className="mt-2">
				{activeTab === "fresh" && (
					<SectionCard eyebrow="Last 24 hours" title="Fresh threads">
						<CatchUpFeedSection
							query={dayFeedQuery}
							emptyTitle="No recent public activity"
							emptyDescription="Nothing crossed the 24-hour threshold for the selected channels."
						/>
					</SectionCard>
				)}

				{activeTab === "steady" && (
					<SectionCard eyebrow="This week" title="Steady conversations">
						<CatchUpFeedSection
							query={weekFeedQuery}
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
						<CatchUpFeedSection
							query={trendingFeedQuery}
							cardClassName="bg-(--color-bg-panel)"
							emptyTitle="Trending needs more history"
							emptyDescription="This panel fills in automatically as the worker accumulates more public-channel thread summaries."
							errorTitle="Trending is unavailable"
							errorDescription="The API route is reachable, but the trending query failed. Retry once the local API is healthy again."
						/>
					</SectionCard>
				)}

				{activeTab === "channels" && (
					<SectionCard
						eyebrow="Channel activity"
						title="Where people are gathering"
						actions={<TrendingUp className="text-eyebrow size-5" />}
					>
						<QueryState
							isPending={overviewQuery.isPending}
							isError={overviewQuery.isError}
							isEmpty={highlights.length === 0}
							loading={<CardSkeletonList count={3} />}
							error={
								<EmptyState
									title="Channel highlights are unavailable"
									description="The overview query failed. Retry once the local API is healthy again."
								/>
							}
							empty={
								<EmptyState
									title="No channel highlights yet"
									description="More public-channel thread summaries will populate this panel automatically."
									icon={<Hash className="size-5" />}
								/>
							}
						>
							<div className="space-y-2.5">
								{highlights.map((channel) => (
									<HighlightCard key={channel.id} channel={channel} />
								))}
							</div>
						</QueryState>
					</SectionCard>
				)}
			</div>
		</div>
	);
}

function useCatchUpFeed({
	window,
	channelId,
	sort = "activity",
	enabled,
}: {
	window: CatchUpWindow;
	channelId?: string;
	sort?: CatchUpSort;
	enabled: boolean;
}) {
	return useInfiniteQuery({
		queryKey: catchUpQueries.feedKey({ window, channelId, sort }),
		queryFn: ({ pageParam }) =>
			fetchCatchUp({
				window,
				channelId,
				sort,
				cursor: pageParam || undefined,
				limit: CATCH_UP_PAGE_SIZE,
			}),
		initialPageParam: "",
		getNextPageParam: (lastPage) => lastPage.next_cursor ?? undefined,
		staleTime: 30_000,
		enabled,
	});
}

function CatchUpFeedSection({
	query,
	emptyTitle,
	emptyDescription,
	errorTitle = "The catch-up feed is unavailable",
	errorDescription = "The API route is reachable, but the current query failed. Retry once the local API is healthy again.",
	cardClassName,
}: {
	query: ReturnType<typeof useCatchUpFeed>;
	emptyTitle: string;
	emptyDescription: string;
	errorTitle?: string;
	errorDescription?: string;
	cardClassName?: string;
}) {
	const threads = flattenCatchUpPages(query.data?.pages ?? []);

	return (
		<QueryState
			isPending={query.isPending}
			isError={query.isError}
			isEmpty={threads.length === 0}
			loading={<CardSkeletonList />}
			error={<EmptyState title={errorTitle} description={errorDescription} />}
			empty={<EmptyState title={emptyTitle} description={emptyDescription} />}
		>
			<div className="space-y-2.5">
				{threads.map((thread) => (
					<ThreadCard
						key={thread.id}
						{...threadCardDataFromCatchUpThread(
							thread,
							getCatchUpThreadChannelName(thread),
						)}
						className={cardClassName}
					/>
				))}
				<InfiniteScrollSentinel
					hasNextPage={query.hasNextPage}
					isFetchingNextPage={query.isFetchingNextPage}
					onLoadMore={() => {
						if (!query.isFetchingNextPage) {
							void query.fetchNextPage();
						}
					}}
				/>
			</div>
		</QueryState>
	);
}

function InfiniteScrollSentinel({
	hasNextPage,
	isFetchingNextPage,
	onLoadMore,
}: {
	hasNextPage: boolean;
	isFetchingNextPage: boolean;
	onLoadMore: () => void;
}) {
	const ref = useRef<HTMLDivElement | null>(null);

	useEffect(() => {
		if (!hasNextPage || isFetchingNextPage || !ref.current) {
			return;
		}

		const observer = new IntersectionObserver(
			(entries) => {
				if (entries.some((entry) => entry.isIntersecting)) {
					onLoadMore();
				}
			},
			{ rootMargin: "280px 0px" },
		);
		observer.observe(ref.current);

		return () => observer.disconnect();
	}, [hasNextPage, isFetchingNextPage, onLoadMore]);

	if (!hasNextPage && !isFetchingNextPage) {
		return null;
	}

	return (
		<div ref={ref} className="pt-1">
			{isFetchingNextPage ? (
				<CardSkeletonList
					count={2}
					cardClassName="surface-frost h-28 rounded-(--radius-card)"
				/>
			) : (
				<div className="h-8" aria-hidden="true" />
			)}
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
	search: { channel?: string; tab?: HomeTab };
}) {
	return (
		<Link
			to="/"
			search={search}
			className={cn(
				"rounded-[0.8rem] border px-3 py-2 text-[0.82rem] font-medium transition-[background-color,border-color,color,box-shadow] duration-200 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-(--color-accent-soft)/40",
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
						#{getCatchUpChannelLabel(channel)}
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
