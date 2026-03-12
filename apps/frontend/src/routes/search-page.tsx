import { EmptyState } from "@/components/empty-state";
import { SectionCard } from "@/components/section-card";
import { ThreadCard } from "@/components/thread-card";
import { Button } from "@/components/ui/button";
import { highlightMatches } from "@/lib/highlight";
import { searchQueries } from "@/lib/queries";
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

	return (
		<div className="space-y-4">
			<section className="rounded-[0.9rem] border border-white/8 bg-[#07090b] p-3.5 shadow-[0_10px_30px_rgba(0,0,0,0.18)] sm:p-4">
				<p className="text-[0.62rem] font-medium uppercase tracking-[0.28em] text-[#20cb74]">
					Search
				</p>
				<h2 className="mt-2 text-[1.45rem] font-semibold tracking-tight text-white sm:text-[1.65rem]">
					Search public-channel history without losing the thread context.
				</h2>
				<div className="mt-4 grid gap-2 xl:grid-cols-[minmax(0,1fr)_200px_200px_200px]">
					<label className="sm:col-span-2">
						<span className="mb-1.5 block text-[0.62rem] font-medium uppercase tracking-[0.24em] text-[#70737b]">
							Query
						</span>
						<div className="flex items-center gap-2.5 rounded-[0.8rem] border border-white/8 bg-[#121417] px-3 py-2.5 focus-within:border-[#1fc86f]/28">
							<Search className="size-3.5 shrink-0 text-[#6c7078]" />
							<input
								className="w-full bg-transparent text-[0.92rem] text-white outline-none placeholder:text-[#6f7279]"
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
					<FilterField
						label="Channel"
						value={channelId}
						placeholder="C123 or channel id"
						onChange={(v) => updateSearch({ channel_id: v || undefined })}
					/>
					<div className="grid grid-cols-2 gap-2">
						<FilterField
							label="From"
							type="date"
							value={dateFrom}
							icon={<CalendarRange className="size-4 text-[#6c7078]" />}
							onChange={(v) => updateSearch({ date_from: v || undefined })}
						/>
						<FilterField
							label="To"
							type="date"
							value={dateTo}
							icon={<CalendarRange className="size-4 text-[#6c7078]" />}
							onChange={(v) => updateSearch({ date_to: v || undefined })}
						/>
					</div>
				</div>
				<div className="mt-3 flex flex-wrap items-center gap-1.5">
					<span className="inline-flex items-center gap-2 rounded-[0.7rem] border border-white/8 bg-white/[0.03] px-2.5 py-1.25 text-[0.62rem] uppercase tracking-[0.18em] text-[#757983]">
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
									? "bg-[#18cc77] text-black hover:bg-[#2ae38a]"
									: "border-white/10 bg-white/[0.03] text-white hover:border-[#1fc86f]/30 hover:bg-white/[0.06]"
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
				{searchQuery.isPending && deferredQuery.length > 0 ? (
					<div className="space-y-3">
						{["skel-a", "skel-b", "skel-c", "skel-d"].map((id) => (
							<div
								key={id}
								className="h-28 animate-pulse rounded-(--radius-card) border border-(--color-border-subtle) bg-(--color-bg-surface)/40"
							/>
						))}
					</div>
				) : searchQuery.isError ? (
					<EmptyState
						title="Search is unavailable"
						description="The API search route failed for this query. Check the local API session and try again."
					/>
				) : deferredQuery.length === 0 ? (
					<EmptyState
						title="Search needs a query"
						description="Type a phrase above and the results will stream in with channel and date filters applied."
						icon={<Search className="size-5" />}
					/>
				) : searchQuery.data?.items.length ? (
					<div className="space-y-3">
						{searchQuery.data.items.map((item) => (
							<ThreadCard
								key={item.id}
								threadId={item.thread_id}
								channelName={item.channel_name || item.channel_id}
								author={item.author}
								title={highlightMatches(item.title, deferredQuery)}
								preview={highlightMatches(item.snippet, deferredQuery)}
								lastActivityTs={item.message_ts}
								replyCount={0}
								participantCount={0}
								reactionCount={0}
							/>
						))}
					</div>
				) : (
					<EmptyState
						title="No matches yet"
						description="Try a broader term, remove the channel filter, or widen the date range."
					/>
				)}
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
			<span className="mb-1.5 block text-[0.62rem] font-medium uppercase tracking-[0.24em] text-[#70737b]">
				{label}
			</span>
			<div className="flex items-center gap-2 rounded-[0.8rem] border border-white/8 bg-[#121417] px-3 py-2.5">
				{icon}
				<input
					type={type}
					value={value}
					onChange={(e) => onChange(e.target.value)}
					placeholder={placeholder}
					className="w-full bg-transparent text-[0.88rem] text-white outline-none placeholder:text-[#6f7279]"
				/>
			</div>
		</label>
	);
}
