import { EmptyState } from "@/components/empty-state";
import { CardSkeletonList, QueryState } from "@/components/query-state";
import { SavableThreadCard } from "@/components/savable-thread-card";
import { SectionCard } from "@/components/section-card";
import { SegmentedTabs } from "@/components/ui/segmented-tabs";
import type {
	CatchUpSort,
	CatchUpWindow,
	SavedItem,
	StarredItem,
} from "@/lib/api";
import {
	CATCH_UP_PAGE_SIZE,
	flattenCatchUpPages,
	getCatchUpChannelLabel,
	getCatchUpThreadChannelName,
} from "@/lib/catch-up";
import {
	type HomeTab,
	normalizeHomeTab,
	toHomeTabSearch,
} from "@/lib/home-tabs";
import { catchUpQueries, savedQueries, starredQueries } from "@/lib/queries";
import {
	threadCardDataFromCatchUpThread,
	threadCardDataFromStarredItem,
} from "@/lib/thread-card-props";
import { indexSavedItemsByThreadId } from "@/lib/thread-save";
import { indexStarredItemsByThreadId } from "@/lib/thread-star";
import { cn } from "@/lib/utils";
import { useInfiniteQuery, useQuery } from "@tanstack/react-query";
import { Link, useNavigate, useSearch } from "@tanstack/react-router";
import {
	Calendar,
	ChevronDown,
	Clock,
	Flame,
	Hash,
	Star,
} from "lucide-react";
import { useEffect, useRef } from "react";

export function HomePage() {
	const { channel: channelFilter, tab } = useSearch({ from: "/app/" });
	const navigate = useNavigate();
	const activeTab = normalizeHomeTab(tab);
	const activeFilter = channelFilter ?? "all";
	const activeChannelId = activeFilter === "all" ? undefined : activeFilter;
	const overviewQuery = useQuery(catchUpQueries.summary("7d"));
	const savedQuery = useQuery(savedQueries.list());
	const starredQuery = useQuery({
		...starredQueries.list({ channelId: activeChannelId }),
	});
	const savedItemsByThreadId = indexSavedItemsByThreadId(
		savedQuery.data?.items,
	);
	const starredItemsByThreadId = indexStarredItemsByThreadId(
		starredQuery.data?.items,
	);
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
	const tabItems = [
		{
			key: "starred",
			label: "Starred",
			shortLabel: "Starred",
			icon: <Star className="size-3.5 shrink-0 sm:size-4" />,
		},
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
						<h1 className="mt-2 text-[clamp(1.95rem,8vw,2.3rem)] font-semibold leading-[1.08] tracking-tight text-(--color-text-primary)">
							Catch up on the go.
						</h1>
					</div>
				</div>

				<div className="relative mt-4 sm:hidden">
					<MobileChannelFilter
						activeFilter={activeFilter}
						activeTab={activeTab}
						availableChannels={availableChannels}
					/>
				</div>

				<div className="relative mt-4 hidden flex-wrap gap-2 sm:flex">
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
				{activeTab === "starred" && (
					<SectionCard
						eyebrow="Starred"
						title="Admin picks worth opening"
						actions={<Star className="text-eyebrow size-5" />}
					>
						<StarredSection
							items={starredQuery.data?.items ?? []}
							isPending={starredQuery.isPending}
							isError={starredQuery.isError}
							savedItemsByThreadId={savedItemsByThreadId}
							starredItemsByThreadId={starredItemsByThreadId}
						/>
					</SectionCard>
				)}

				{activeTab === "fresh" && (
					<SectionCard eyebrow="Last 24 hours" title="Fresh threads">
						<CatchUpFeedSection
							query={dayFeedQuery}
							savedItemsByThreadId={savedItemsByThreadId}
							starredItemsByThreadId={starredItemsByThreadId}
							emptyTitle="No recent public activity"
							emptyDescription="Nothing crossed the 24-hour threshold for the selected channels."
						/>
					</SectionCard>
				)}

				{activeTab === "steady" && (
					<SectionCard eyebrow="This week" title="Steady conversations">
						<CatchUpFeedSection
							query={weekFeedQuery}
							savedItemsByThreadId={savedItemsByThreadId}
							starredItemsByThreadId={starredItemsByThreadId}
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
							savedItemsByThreadId={savedItemsByThreadId}
							starredItemsByThreadId={starredItemsByThreadId}
							cardClassName="bg-(--color-bg-panel)"
							emptyTitle="Trending needs more history"
							emptyDescription="This panel fills in automatically as the worker accumulates more public-channel thread summaries."
							errorTitle="Trending is unavailable"
							errorDescription="The API route is reachable, but the trending query failed. Retry once the local API is healthy again."
						/>
					</SectionCard>
				)}
			</div>
		</div>
	);
}

function StarredSection({
	items,
	isPending,
	isError,
	savedItemsByThreadId,
	starredItemsByThreadId,
}: {
	items: StarredItem[];
	isPending: boolean;
	isError: boolean;
	savedItemsByThreadId: Map<string, SavedItem>;
	starredItemsByThreadId: Map<string, StarredItem>;
}) {
	return (
		<QueryState
			isPending={isPending}
			isError={isError}
			isEmpty={items.length === 0}
			loading={<CardSkeletonList />}
			error={
				<EmptyState
					title="Starred are unavailable"
					description="The API route is reachable, but the curated thread query failed. Retry once the local API is healthy again."
				/>
			}
			empty={
				<EmptyState
					title="No starred yet"
					description="Admin-picked threads will surface here as soon as they are curated."
				/>
			}
		>
			<div className="space-y-2.5">
				{items.map((item) => (
					<SavableThreadCard
						key={item.id}
						{...threadCardDataFromStarredItem(item)}
						savedItem={savedItemsByThreadId.get(item.thread_id)}
						starredItem={
							starredItemsByThreadId.get(item.thread_id) ?? null
						}
						isOnline={true}
					/>
				))}
			</div>
		</QueryState>
	);
}

function MobileChannelFilter({
	activeFilter,
	activeTab,
	availableChannels,
}: {
	activeFilter: string;
	activeTab: HomeTab;
	availableChannels: {
		id: string;
		name: string | null;
	}[];
}) {
	const navigate = useNavigate();
	const selectedChannel =
		activeFilter === "all"
			? null
			: (availableChannels.find((channel) => channel.id === activeFilter) ??
				null);
	const selectedLabel =
		activeFilter === "all"
			? "All channels"
			: getCatchUpChannelLabel(
					selectedChannel ?? { id: activeFilter, name: null },
				);

	return (
		<label className="surface-input relative flex min-h-12 items-center gap-3 rounded-[1rem] px-3 py-2.5">
			<div className="flex min-w-0 flex-1 items-center gap-2.5">
				<div className="flex size-9 shrink-0 items-center justify-center rounded-[0.85rem] bg-(--color-accent)/12 text-(--color-accent-soft)">
					<Hash className="size-4" />
				</div>
				<div className="min-w-0">
					<p className="text-copy-quiet text-[0.62rem] font-medium uppercase tracking-[0.18em]">
						Channel
					</p>
					<p className="truncate text-[0.94rem] font-medium text-(--color-text-primary)">
						{selectedLabel}
					</p>
				</div>
			</div>

			<select
				aria-label="Filter by channel"
				value={activeFilter}
				onChange={(event) => {
					const nextValue = event.target.value;
					void navigate({
						to: "/",
						search: {
							channel: nextValue === "all" ? undefined : nextValue,
							tab: toHomeTabSearch(activeTab),
						},
						replace: false,
					});
				}}
				className="absolute inset-0 opacity-0"
			>
				<option value="all">All channels</option>
				{availableChannels.map((channel) => (
					<option key={channel.id} value={channel.id}>
						{getCatchUpChannelLabel(channel)}
					</option>
				))}
			</select>

			<ChevronDown className="text-copy-quiet size-4 shrink-0" />
		</label>
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
		...catchUpQueries.feed({
			window,
			channelId,
			sort,
			limit: CATCH_UP_PAGE_SIZE,
		}),
		enabled,
	});
}

function CatchUpFeedSection({
	query,
	savedItemsByThreadId,
	starredItemsByThreadId,
	emptyTitle,
	emptyDescription,
	errorTitle = "The catch-up feed is unavailable",
	errorDescription = "The API route is reachable, but the current query failed. Retry once the local API is healthy again.",
	cardClassName,
}: {
	query: ReturnType<typeof useCatchUpFeed>;
	savedItemsByThreadId: Map<string, SavedItem>;
	starredItemsByThreadId: Map<string, StarredItem>;
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
					<SavableThreadCard
						key={thread.id}
						{...threadCardDataFromCatchUpThread(
							thread,
							getCatchUpThreadChannelName(thread),
						)}
						savedItem={savedItemsByThreadId.get(thread.id)}
						starredItem={starredItemsByThreadId.get(thread.id) ?? null}
						isOnline={true}
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
			{ rootMargin: "760px 0px" },
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
