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

	const query = search.q ?? "";
	const channelId = search.channel_id ?? "";
	const dateFrom = search.date_from ?? "";
	const dateTo = search.date_to ?? "";
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
		<div className="space-y-6">
			<section className="rounded-[1.9rem] border border-white/8 bg-[#090b0d] p-5 shadow-[0_24px_80px_rgba(0,0,0,0.42)] sm:p-6">
				<p className="text-[0.72rem] font-medium uppercase tracking-[0.32em] text-[#20cb74]">
					Search
				</p>
				<h2 className="mt-3 text-3xl font-semibold tracking-tight text-white">
					Search public-channel history without losing the thread context.
				</h2>
				<div className="mt-6 grid gap-3 xl:grid-cols-[minmax(0,1fr)_220px_220px_220px]">
					<label className="sm:col-span-2">
						<span className="mb-2 block text-[0.68rem] font-medium uppercase tracking-[0.26em] text-[#70737b]">
							Query
						</span>
						<div className="flex items-center gap-3 rounded-[1.25rem] border border-white/8 bg-[#121417] px-4 py-3.5 focus-within:border-[#1fc86f]/28">
							<Search className="size-4 shrink-0 text-[#6c7078]" />
							<input
								className="w-full bg-transparent text-lg text-white outline-none placeholder:text-[#6f7279]"
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
					<div className="grid grid-cols-2 gap-3">
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
				<div className="mt-4 flex flex-wrap items-center gap-2.5">
					<span className="inline-flex items-center gap-2 rounded-full border border-white/8 bg-white/[0.03] px-3 py-2 text-xs uppercase tracking-[0.2em] text-[#757983]">
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
				description="Results stay thread-shaped so you can jump directly into the full conversation with its author, channel, and message context."
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
			<span className="mb-2 block text-[0.68rem] font-medium uppercase tracking-[0.26em] text-[#70737b]">
				{label}
			</span>
			<div className="flex items-center gap-3 rounded-[1.25rem] border border-white/8 bg-[#121417] px-4 py-3.5">
				{icon}
				<input
					type={type}
					value={value}
					onChange={(e) => onChange(e.target.value)}
					placeholder={placeholder}
					className="w-full bg-transparent text-base text-white outline-none placeholder:text-[#6f7279]"
				/>
			</div>
		</label>
	);
}
