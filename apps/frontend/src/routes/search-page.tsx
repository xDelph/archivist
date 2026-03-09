import { EmptyState } from "@/components/empty-state";
import { SectionCard } from "@/components/section-card";
import { ThreadCard } from "@/components/thread-card";
import { Button } from "@/components/ui/button";
import { fetchSearchResults } from "@/lib/api";
import { highlightMatches } from "@/lib/highlight";
import { useQuery } from "@tanstack/react-query";
import { Search } from "lucide-react";
import { startTransition, useDeferredValue, useState } from "react";

export function SearchPage() {
	const [query, setQuery] = useState("");
	const [channelId, setChannelId] = useState("");
	const [dateFrom, setDateFrom] = useState("");
	const [dateTo, setDateTo] = useState("");
	const [sort, setSort] = useState<"relevance" | "newest">("relevance");
	const deferredQuery = useDeferredValue(query.trim());
	const searchQuery = useQuery({
		queryKey: ["search", deferredQuery, channelId, dateFrom, dateTo, sort],
		queryFn: () =>
			fetchSearchResults({
				query: deferredQuery,
				channelId: channelId || undefined,
				dateFrom: dateFrom || undefined,
				dateTo: dateTo || undefined,
				sort,
			}),
		enabled: deferredQuery.length > 0,
	});

	return (
		<div className="space-y-5">
			<section className="rounded-[2rem] border border-white/10 bg-[linear-gradient(135deg,rgba(14,29,44,0.92),rgba(7,14,24,0.9))] p-5 sm:p-6">
				<p className="text-[0.68rem] uppercase tracking-[0.32em] text-[var(--accent-soft)]">
					Search
				</p>
				<h2 className="mt-3 text-3xl font-semibold text-white">
					Search public-channel history without losing the thread context.
				</h2>
				<div className="mt-5 grid gap-3 sm:grid-cols-2">
					<label className="sm:col-span-2">
						<span className="mb-2 block text-xs uppercase tracking-[0.24em] text-slate-400">
							Query
						</span>
						<div className="flex items-center gap-3 rounded-[1.25rem] border border-white/10 bg-slate-950/40 px-4 py-3">
							<Search className="size-4 text-slate-400" />
							<input
								className="w-full bg-transparent text-base text-white outline-none placeholder:text-slate-500"
								value={query}
								onChange={(event) => {
									startTransition(() => {
										setQuery(event.target.value);
									});
								}}
								placeholder="Search for a question, project, or decision"
							/>
						</div>
					</label>
					<FilterField
						label="Channel"
						value={channelId}
						placeholder="C123 or channel id"
						onChange={setChannelId}
					/>
					<div className="grid grid-cols-2 gap-3">
						<FilterField
							label="From"
							type="date"
							value={dateFrom}
							onChange={setDateFrom}
						/>
						<FilterField
							label="To"
							type="date"
							value={dateTo}
							onChange={setDateTo}
						/>
					</div>
				</div>
				<div className="mt-4 flex flex-wrap gap-2">
					{(["relevance", "newest"] as const).map((option) => (
						<Button
							key={option}
							type="button"
							variant={sort === option ? "default" : "secondary"}
							size="sm"
							onClick={() => setSort(option)}
						>
							{option}
						</Button>
					))}
				</div>
			</section>

			<SectionCard
				eyebrow="Results"
				title={
					deferredQuery
						? `Results for “${deferredQuery}”`
						: "Start typing to search"
				}
				description="Results stay thread-shaped so you can jump directly into the full conversation."
			>
				{searchQuery.isPending ? (
					<div className="space-y-3">
						{[
							"search-skeleton-1",
							"search-skeleton-2",
							"search-skeleton-3",
							"search-skeleton-4",
						].map((key) => (
							<div
								key={key}
								className="h-32 animate-pulse rounded-[1.5rem] border border-white/8 bg-white/[0.04]"
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
					/>
				) : searchQuery.data?.items.length ? (
					<div className="space-y-3">
						{searchQuery.data.items.map((item) => (
							<ThreadCard
								key={item.id}
								threadId={item.thread_id}
								channelName={item.channel_name || item.channel_id}
								title={highlightMatches(item.title, deferredQuery)}
								preview={highlightMatches(item.snippet, deferredQuery)}
								lastActivityTs={item.message_ts}
								replyCount={0}
								participantCount={0}
								reactionCount={item.score}
								className="border-[var(--accent-soft)]/10"
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
}: {
	label: string;
	value: string;
	onChange: (value: string) => void;
	type?: "text" | "date";
	placeholder?: string;
}) {
	return (
		<label>
			<span className="mb-2 block text-xs uppercase tracking-[0.24em] text-slate-400">
				{label}
			</span>
			<input
				type={type}
				value={value}
				onChange={(event) => {
					startTransition(() => {
						onChange(event.target.value);
					});
				}}
				placeholder={placeholder}
				className="w-full rounded-[1.1rem] border border-white/10 bg-slate-950/40 px-4 py-3 text-sm text-white outline-none placeholder:text-slate-500"
			/>
		</label>
	);
}
