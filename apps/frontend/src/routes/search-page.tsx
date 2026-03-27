import { EmptyState } from "@/components/empty-state";
import { CardSkeletonList, QueryState } from "@/components/query-state";
import { SavableThreadCard } from "@/components/savable-thread-card";
import { SectionCard } from "@/components/section-card";
import { ThreadListSortBar } from "@/components/thread-list-sort-bar";
import { Button } from "@/components/ui/button";
import { InputField, SelectField } from "@/components/ui/form-field";
import { highlightMatches } from "@/lib/highlight";
import {
	channelQueries,
	savedQueries,
	searchQueries,
	starredQueries,
} from "@/lib/queries";
import { flattenSearchPages } from "@/lib/search";
import { threadCardDataFromSearchResult } from "@/lib/thread-card-props";
import {
	normalizeSearchThreadSort,
	searchThreadSortOptions,
	toSearchThreadSortSearch,
} from "@/lib/thread-list-sort";
import { indexSavedItemsByThreadId } from "@/lib/thread-save";
import { indexStarredItemsByThreadId } from "@/lib/thread-star";
import { useInfiniteQuery, useQuery } from "@tanstack/react-query";
import { useNavigate, useSearch } from "@tanstack/react-router";
import { CalendarRange, Search, SlidersHorizontal } from "lucide-react";
import { useDeferredValue, useEffect, useId, useRef, useState } from "react";

export function SearchPage() {
	const search = useSearch({ from: "/app/search" });
	const navigate = useNavigate();
	const defaultDateRange = getDefaultSearchDateRange();

	const query = search.q ?? "";
	const channelId = search.channel_id ?? "";
	const dateFrom = search.date_from ?? defaultDateRange.from;
	const dateTo = search.date_to ?? defaultDateRange.to;
	const sort = normalizeSearchThreadSort(search.sort);
	const filterSummary = getSearchFilterSummary({
		channelId,
		dateFrom,
		dateTo,
		defaultDateRange,
	});
	const [areFiltersOpen, setAreFiltersOpen] = useState(
		filterSummary.hasActiveFilters,
	);
	const filtersPanelId = useId();

	const deferredQuery = useDeferredValue(query.trim());
	const channelsQuery = useQuery(channelQueries.list());
	const savedQuery = useQuery(savedQueries.list());
	const starredQuery = useQuery(starredQueries.list());
	const savedItemsByThreadId = indexSavedItemsByThreadId(
		savedQuery.data?.items,
	);
	const starredItemsByThreadId = indexStarredItemsByThreadId(
		starredQuery.data?.items,
	);

	const searchQuery = useInfiniteQuery(
		searchQueries.results({
			query: deferredQuery,
			channelId: channelId || undefined,
			dateFrom: dateFrom || undefined,
			dateTo: dateTo || undefined,
			sort,
		}),
	);
	const searchResults = flattenSearchPages(searchQuery.data?.pages ?? []);

	function updateSearch(updates: Record<string, string | undefined>) {
		void navigate({
			to: "/search",
			search: { ...search, ...updates },
		});
	}

	const channelOptions = channelsQuery.data ?? [];
	const hasSelectedChannelOption = channelId
		? channelOptions.some((channel) => channel.id === channelId)
		: true;

	return (
		<div className="space-y-4">
			<section className="surface-panel p-4 sm:p-5">
				<p className="text-eyebrow text-[0.72rem] font-medium uppercase tracking-[0.18em]">
					Search
				</p>
				<h2 className="mt-2 text-[clamp(1.4rem,3vw,2rem)] font-semibold tracking-tight text-(--color-text-primary)">
					Search public-thread history, then open the full conversation when a
					thread preview looks promising.
				</h2>
				<div className="mt-4 flex flex-col gap-3 sm:flex-row sm:items-end">
					<InputField
						label="Query"
						value={query}
						onValueChange={(value) => updateSearch({ q: value || undefined })}
						placeholder="Search for a question, project, decision, or phrase"
						inputMode="search"
						enterKeyHint="search"
						icon={<Search className="text-copy-quiet size-3.5 shrink-0" />}
						fieldClassName="min-w-0 flex-1"
						shellClassName="gap-2.5 focus-within:border-(--color-border-accent)"
						inputClassName="text-[0.92rem]"
					/>
					<Button
						type="button"
						variant="secondary"
						className="h-11 min-w-34 justify-between gap-3 rounded-[0.9rem] px-4 text-left"
						aria-expanded={areFiltersOpen}
						aria-controls={filtersPanelId}
						onClick={() => setAreFiltersOpen((current) => !current)}
					>
						<span className="inline-flex items-center gap-2">
							<SlidersHorizontal className="size-4" />
							{areFiltersOpen ? "Hide filters" : "Filters"}
						</span>
						{filterSummary.activeCount > 0 ? (
							<span className="accent-pill rounded-full px-2 py-0.5 text-[0.68rem]">
								{filterSummary.activeCount}
							</span>
						) : null}
					</Button>
				</div>
				<div
					className={`grid transition-[grid-template-rows,opacity,margin] duration-200 ease-[cubic-bezier(0.16,1,0.3,1)] ${
						areFiltersOpen
							? "mt-4 grid-rows-[1fr] opacity-100"
							: "mt-0 grid-rows-[0fr] opacity-0"
					}`}
				>
					<div className="overflow-hidden">
						<div
							id={filtersPanelId}
							className={`surface-frost rounded-(--radius-subpanel) border border-(--color-border-subtle) p-3 transition-opacity duration-200 ${
								areFiltersOpen ? "pointer-events-auto" : "pointer-events-none"
							}`}
							aria-hidden={!areFiltersOpen}
						>
							<div className="grid gap-2 xl:grid-cols-[minmax(0,1fr)_200px_200px]">
								<SelectField
									label="Channel"
									value={channelId}
									onValueChange={(value) =>
										updateSearch({ channel_id: value || undefined })
									}
									disabled={channelsQuery.isPending || channelsQuery.isError}
									options={[
										{
											key: "__all_channels__",
											value: "",
											label: "All channels",
										},
										...channelOptions.map((channel) => ({
											value: channel.id,
											label: `#${channel.name ?? channel.id}`,
										})),
										...(channelId && !hasSelectedChannelOption
											? [{ value: channelId, label: channelId }]
											: []),
									]}
									fieldClassName="xl:col-span-1"
								/>
								<div className="grid grid-cols-2 gap-2 xl:col-span-2">
									<InputField
										label="From"
										type="date"
										value={dateFrom}
										icon={<CalendarRange className="text-copy-quiet size-4" />}
										onValueChange={(value) =>
											updateSearch({ date_from: value || undefined })
										}
									/>
									<InputField
										label="To"
										type="date"
										value={dateTo}
										icon={<CalendarRange className="text-copy-quiet size-4" />}
										onValueChange={(value) =>
											updateSearch({ date_to: value || undefined })
										}
									/>
								</div>
							</div>
							<div className="mt-3 flex justify-end">
								<Button
									type="button"
									variant="ghost"
									size="sm"
									className="text-copy-soft"
									onClick={() =>
										updateSearch({
											channel_id: undefined,
											date_from: undefined,
											date_to: undefined,
										})
									}
								>
									Clear filters
								</Button>
							</div>
						</div>
					</div>
				</div>
			</section>

			<SectionCard
				eyebrow="Matches"
				title={
					deferredQuery
						? `Results for "${deferredQuery}"`
						: "Search the archive"
				}
				toolbar={
					<ThreadListSortBar
						label="Order"
						options={searchThreadSortOptions}
						value={sort}
						onChange={(nextSort) => {
							updateSearch({
								sort: toSearchThreadSortSearch(nextSort),
							});
						}}
					/>
				}
			>
				<QueryState
					isPending={searchQuery.isPending && deferredQuery.length > 0}
					isError={searchQuery.isError}
					isEmpty={deferredQuery.length === 0 || searchResults.length === 0}
					loading={
						<CardSkeletonList
							count={4}
							cardClassName="h-28 rounded-(--radius-card) border border-(--color-border-subtle) bg-(--color-bg-surface)/40"
							className="space-y-3"
						/>
					}
					error={
						<EmptyState
							title="Search is unavailable"
							description="The API search route failed for this query. Check the local API session and try again."
						/>
					}
					empty={
						deferredQuery.length === 0 ? (
							<EmptyState
								title="Search needs a query"
								description="Type a phrase above to search across public threads with channel and date filters applied."
								icon={<Search className="size-5" />}
							/>
						) : (
							<EmptyState
								title="No matches yet"
								description="Try a broader term, remove the channel filter, or widen the date range."
							/>
						)
					}
				>
					<div className="space-y-3">
						{searchResults.map((item) => {
							return (
								<SavableThreadCard
									key={item.id}
									{...threadCardDataFromSearchResult(item, {
										title: highlightMatches(item.title, deferredQuery),
										preview: highlightMatches(item.preview, deferredQuery),
									})}
									savedItem={savedItemsByThreadId.get(item.thread_id)}
									starredItem={
										starredItemsByThreadId.get(item.thread_id) ?? null
									}
									isOnline={true}
								/>
							);
						})}
						<SearchResultsSentinel
							hasNextPage={searchQuery.hasNextPage}
							isFetchingNextPage={searchQuery.isFetchingNextPage}
							onLoadMore={() => {
								if (!searchQuery.isFetchingNextPage) {
									void searchQuery.fetchNextPage();
								}
							}}
						/>
					</div>
				</QueryState>
			</SectionCard>
		</div>
	);
}

function getDefaultSearchDateRange() {
	const today = new Date();
	const yesterday = new Date(today);
	yesterday.setDate(today.getDate() - 1);

	return {
		from: formatDateInputValue(yesterday),
		to: formatDateInputValue(today),
	};
}

export function getSearchFilterSummary({
	channelId,
	dateFrom,
	dateTo,
	defaultDateRange,
}: {
	channelId: string;
	dateFrom: string;
	dateTo: string;
	defaultDateRange: { from: string; to: string };
}) {
	const activeCount = [
		channelId.length > 0,
		dateFrom !== defaultDateRange.from,
		dateTo !== defaultDateRange.to,
	].filter(Boolean).length;

	return {
		activeCount,
		hasActiveFilters: activeCount > 0,
	};
}

function formatDateInputValue(value: Date) {
	const year = value.getFullYear();
	const month = String(value.getMonth() + 1).padStart(2, "0");
	const day = String(value.getDate()).padStart(2, "0");
	return `${year}-${month}-${day}`;
}

function SearchResultsSentinel({
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
					cardClassName="h-28 rounded-(--radius-card) border border-(--color-border-subtle) bg-(--color-bg-surface)/40"
					className="space-y-3"
				/>
			) : (
				<div className="h-8" aria-hidden="true" />
			)}
		</div>
	);
}
