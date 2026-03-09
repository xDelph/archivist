import { EmptyState } from "@/components/empty-state";
import { SectionCard } from "@/components/section-card";
import { ThreadCard } from "@/components/thread-card";
import { Button } from "@/components/ui/button";
import { highlightMatches } from "@/lib/highlight";
import { searchQueries } from "@/lib/queries";
import { useQuery } from "@tanstack/react-query";
import { useNavigate, useSearch } from "@tanstack/react-router";
import { Search } from "lucide-react";
import { useDeferredValue } from "react";

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
		<div className="space-y-5">
			<section className="rounded-(--radius-section) border border-(--color-border-subtle) bg-[linear-gradient(135deg,var(--color-bg-surface),var(--color-bg-base))] p-5 sm:p-6">
				<p className="text-[0.62rem] font-medium uppercase tracking-[0.32em] text-(--color-accent-soft)">
					Search
				</p>
				<h2 className="mt-3 text-2xl font-semibold text-(--color-text-primary) sm:text-3xl">
					Search public-channel history without losing the thread context.
				</h2>
				<div className="mt-5 grid gap-3 sm:grid-cols-2">
					<label className="sm:col-span-2">
						<span className="mb-2 block text-xs font-medium uppercase tracking-[0.24em] text-(--color-text-muted)">
							Query
						</span>
						<div className="flex items-center gap-3 rounded-(--radius-card) border border-(--color-border-subtle) bg-(--color-bg-base)/60 px-4 py-3 focus-within:border-(--color-border-accent)">
							<Search className="size-4 shrink-0 text-(--color-text-muted)" />
							<input
								className="w-full bg-transparent text-base text-(--color-text-primary) outline-none placeholder:text-(--color-text-muted)"
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
							onChange={(v) => updateSearch({ date_from: v || undefined })}
						/>
						<FilterField
							label="To"
							type="date"
							value={dateTo}
							onChange={(v) => updateSearch({ date_to: v || undefined })}
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
				eyebrow="Results"
				title={
					deferredQuery
						? `Results for "${deferredQuery}"`
						: "Start typing to search"
				}
				description="Results stay thread-shaped so you can jump directly into the full conversation."
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
								title={highlightMatches(item.title, deferredQuery)}
								preview={highlightMatches(item.snippet, deferredQuery)}
								lastActivityTs={item.message_ts}
								replyCount={0}
								participantCount={0}
								reactionCount={item.score}
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
			<span className="mb-2 block text-xs font-medium uppercase tracking-[0.24em] text-(--color-text-muted)">
				{label}
			</span>
			<input
				type={type}
				value={value}
				onChange={(e) => onChange(e.target.value)}
				placeholder={placeholder}
				className="w-full rounded-(--radius-card) border border-(--color-border-subtle) bg-(--color-bg-base)/60 px-4 py-3 text-sm text-(--color-text-primary) outline-none placeholder:text-(--color-text-muted) focus:border-(--color-border-accent)"
			/>
		</label>
	);
}
