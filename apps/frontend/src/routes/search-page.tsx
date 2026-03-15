import { EmptyState } from "@/components/empty-state";
import { CardSkeletonList, QueryState } from "@/components/query-state";
import { SectionCard } from "@/components/section-card";
import { ThreadCard } from "@/components/thread-card";
import { Button } from "@/components/ui/button";
import { highlightMatches } from "@/lib/highlight";
import { channelQueries, searchQueries } from "@/lib/queries";
import { threadCardDataFromSearchResult } from "@/lib/thread-card-props";
import { useQuery } from "@tanstack/react-query";
import { useNavigate, useSearch } from "@tanstack/react-router";
import { CalendarRange, Search, SlidersHorizontal } from "lucide-react";
import { type ReactNode, useDeferredValue } from "react";

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
			<section className="surface-panel p-3.5 sm:p-4">
				<p className="text-eyebrow text-[0.62rem] font-medium uppercase tracking-[0.28em]">
					Search
				</p>
				<h2 className="mt-2 text-[1.45rem] font-semibold tracking-tight text-white sm:text-[1.65rem]">
					Search public-channel history without losing the thread context.
				</h2>
				<div className="mt-4 grid gap-2 xl:grid-cols-[minmax(0,1fr)_200px_200px_200px]">
					<label className="sm:col-span-2">
						<span className="text-copy-quiet mb-1.5 block text-[0.62rem] font-medium uppercase tracking-[0.24em]">
							Query
						</span>
						<div className="surface-input flex items-center gap-2.5 px-3 py-2.5 focus-within:border-(--color-border-accent)">
							<Search className="text-copy-quiet size-3.5 shrink-0" />
							<input
								className="w-full bg-transparent text-[0.92rem] text-white outline-none placeholder:text-(--color-text-quiet)"
								value={query}
								onChange={(e) =>
									updateSearch({ q: e.target.value || undefined })
								}
								placeholder="Search for a question, project, or decision"
								inputMode="search"
								enterKeyHint="search"
							/>
						</div>
					</label>
					<FilterSelect
						label="Channel"
						value={channelId}
						onChange={(v) => updateSearch({ channel_id: v || undefined })}
						disabled={channelsQuery.isPending || channelsQuery.isError}
						options={[
							{ value: "", label: "All channels" },
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
						<FilterField
							label="From"
							type="date"
							value={dateFrom}
							icon={<CalendarRange className="text-copy-quiet size-4" />}
							onChange={(v) => updateSearch({ date_from: v || undefined })}
						/>
						<FilterField
							label="To"
							type="date"
							value={dateTo}
							icon={<CalendarRange className="text-copy-quiet size-4" />}
							onChange={(v) => updateSearch({ date_to: v || undefined })}
						/>
					</div>
				</div>
				<div className="mt-3 flex flex-wrap items-center gap-1.5">
					<span className="inline-flex items-center gap-2 rounded-[0.7rem] border border-(--color-border-subtle) bg-white/[0.03] px-2.5 py-1.25 text-[0.62rem] uppercase tracking-[0.18em] text-(--color-text-muted)">
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
									: "border-(--color-border-strong) bg-white/[0.03] text-white hover:border-(--color-border-accent) hover:bg-white/[0.06]"
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
						: "Start typing to search"
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
								description="Type a phrase above and the results will stream in with channel and date filters applied."
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
							<ThreadCard
								key={item.id}
								{...threadCardDataFromSearchResult(item, {
									title: highlightMatches(item.title, deferredQuery),
									preview: highlightMatches(item.snippet, deferredQuery),
								})}
							/>
						))}
					</div>
				</QueryState>
			</SectionCard>
		</div>
	);
}

function FilterSelect({
	label,
	value,
	onChange,
	options,
	disabled = false,
}: {
	label: string;
	value: string;
	onChange: (value: string) => void;
	options: { value: string; label: string }[];
	disabled?: boolean;
}) {
	return (
		<label>
			<span className="text-copy-quiet mb-1.5 block text-[0.62rem] font-medium uppercase tracking-[0.24em]">
				{label}
			</span>
			<div className="surface-input flex items-center gap-2 px-3 py-2.5">
				<select
					value={value}
					onChange={(e) => onChange(e.target.value)}
					disabled={disabled}
					className="w-full bg-transparent text-[0.88rem] text-white outline-none disabled:cursor-not-allowed disabled:text-(--color-text-quiet)"
				>
					{options.map((option) => (
						<option
							key={option.value || "__all_channels__"}
							value={option.value}
							className="bg-(--color-bg-input) text-white"
						>
							{option.label}
						</option>
					))}
				</select>
			</div>
		</label>
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

function FilterField({
	label,
	value,
	onChange,
	type = "text",
	placeholder,
	icon,
}: {
	label: string;
	value: string;
	onChange: (value: string) => void;
	type?: "text" | "date";
	placeholder?: string;
	icon?: ReactNode;
}) {
	return (
		<label>
			<span className="text-copy-quiet mb-1.5 block text-[0.62rem] font-medium uppercase tracking-[0.24em]">
				{label}
			</span>
			<div className="surface-input flex items-center gap-2 px-3 py-2.5">
				{icon}
				<input
					type={type}
					value={value}
					onChange={(e) => onChange(e.target.value)}
					placeholder={placeholder}
					className="w-full bg-transparent text-[0.88rem] text-white outline-none placeholder:text-(--color-text-quiet)"
				/>
			</div>
		</label>
	);
}
