import { EmptyState } from "@/components/empty-state";
import { CardSkeletonList, QueryState } from "@/components/query-state";
import { SavableThreadCard } from "@/components/savable-thread-card";
import { SectionCard } from "@/components/section-card";
import { Button } from "@/components/ui/button";
import { InputField, SelectField } from "@/components/ui/form-field";
import { highlightMatches } from "@/lib/highlight";
import { channelQueries, savedQueries, searchQueries } from "@/lib/queries";
import { threadCardDataFromSearchResult } from "@/lib/thread-card-props";
import { indexSavedItemsByThreadId } from "@/lib/thread-save";
import { useQuery } from "@tanstack/react-query";
import { useNavigate, useSearch } from "@tanstack/react-router";
import { CalendarRange, Search, SlidersHorizontal } from "lucide-react";
import { useDeferredValue } from "react";

export function SearchPage() {
	const search = useSearch({ from: "/app/search" });
	const navigate = useNavigate();
	const defaultDateRange = getDefaultSearchDateRange();

	const query = search.q ?? "";
	const channelId = search.channel_id ?? "";
	const dateFrom = search.date_from ?? defaultDateRange.from;
	const dateTo = search.date_to ?? defaultDateRange.to;
	const sort = search.sort ?? "relevance";

	const deferredQuery = useDeferredValue(query.trim());
	const channelsQuery = useQuery(channelQueries.list());
	const savedQuery = useQuery(savedQueries.list());
	const savedItemsByThreadId = indexSavedItemsByThreadId(
		savedQuery.data?.items,
	);

	const searchQuery = useQuery(
		searchQueries.results({
			query: deferredQuery,
			channelId: channelId || undefined,
			dateFrom: dateFrom || undefined,
			dateTo: dateTo || undefined,
			sort,
		}),
	);

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
				<h2 className="mt-2 text-[clamp(1.4rem,3vw,2rem)] font-semibold tracking-tight text-white">
					Search public-thread history, then open the full conversation when a
					snippet looks promising.
				</h2>
				<div className="mt-4 grid gap-2 xl:grid-cols-[minmax(0,1fr)_200px_200px_200px]">
					<InputField
						label="Query"
						value={query}
						onValueChange={(value) => updateSearch({ q: value || undefined })}
						placeholder="Search for a question, project, decision, or phrase"
						inputMode="search"
						enterKeyHint="search"
						icon={<Search className="text-copy-quiet size-3.5 shrink-0" />}
						fieldClassName="sm:col-span-2"
						shellClassName="gap-2.5 focus-within:border-(--color-border-accent)"
						inputClassName="text-[0.92rem]"
					/>
					<SelectField
						label="Channel"
						value={channelId}
						onValueChange={(value) =>
							updateSearch({ channel_id: value || undefined })
						}
						disabled={channelsQuery.isPending || channelsQuery.isError}
						options={[
							{ key: "__all_channels__", value: "", label: "All channels" },
							...channelOptions.map((channel) => ({
								value: channel.id,
								label: `#${channel.name ?? channel.id}`,
							})),
							...(channelId && !hasSelectedChannelOption
								? [{ value: channelId, label: channelId }]
								: []),
						]}
					/>
					<div className="grid grid-cols-2 gap-2">
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
				<div className="mt-4 flex flex-wrap items-center gap-2">
					<span className="surface-frost inline-flex min-h-10 items-center gap-2 rounded-[0.8rem] px-3 py-2 text-[0.74rem] uppercase tracking-[0.14em] text-(--color-text-muted)">
						<SlidersHorizontal className="size-3.5" />
						Sort
					</span>
					{(["relevance", "newest"] as const).map((option) => (
						<Button
							key={option}
							type="button"
							variant={sort === option ? "default" : "secondary"}
							size="sm"
							className={
								sort === option
									? "bg-(--color-accent) text-black hover:bg-(--color-accent-strong)"
									: "button-ghost"
							}
							onClick={() =>
								updateSearch({
									sort: option === "relevance" ? undefined : option,
								})
							}
						>
							{option}
						</Button>
					))}
				</div>
			</section>

			<SectionCard
				eyebrow="Matches"
				title={
					deferredQuery
						? `Results for "${deferredQuery}"`
						: "Search the archive"
				}
			>
				<QueryState
					isPending={searchQuery.isPending && deferredQuery.length > 0}
					isError={searchQuery.isError}
					isEmpty={
						deferredQuery.length === 0 ||
						(searchQuery.data?.items.length ?? 0) === 0
					}
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
						{searchQuery.data?.items.map((item) => (
							<SavableThreadCard
								key={item.id}
								{...threadCardDataFromSearchResult(item, {
									title: highlightMatches(item.title, deferredQuery),
									preview: highlightMatches(item.snippet, deferredQuery),
								})}
								savedItem={savedItemsByThreadId.get(item.thread_id)}
								isOnline={true}
							/>
						))}
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

function formatDateInputValue(value: Date) {
	const year = value.getFullYear();
	const month = String(value.getMonth() + 1).padStart(2, "0");
	const day = String(value.getDate()).padStart(2, "0");
	return `${year}-${month}-${day}`;
}
