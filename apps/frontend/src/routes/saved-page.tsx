import { EmptyState } from "@/components/empty-state";
import { CardSkeletonList, QueryState } from "@/components/query-state";
import { SectionCard } from "@/components/section-card";
import { ThreadCard } from "@/components/thread-card";
import { Button } from "@/components/ui/button";
import { deleteSavedThread } from "@/lib/api";
import { savedQueries } from "@/lib/queries";
import { threadCardDataFromSavedItem } from "@/lib/thread-card-props";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Bookmark, BookmarkX } from "lucide-react";

export function SavedPage() {
	const queryClient = useQueryClient();
	const savedQuery = useQuery(savedQueries.list());
	const deleteMutation = useMutation({
		mutationFn: deleteSavedThread,
		onSuccess: async () => {
			await queryClient.invalidateQueries({ queryKey: ["saved"] });
		},
	});

	return (
		<div className="space-y-5">
			<SectionCard eyebrow="Saved" title="Bookmarked threads">
				<QueryState
					isPending={savedQuery.isPending}
					isError={savedQuery.isError}
					isEmpty={(savedQuery.data?.items.length ?? 0) === 0}
					loading={
						<CardSkeletonList
							count={2}
							cardClassName="h-32 rounded-(--radius-card) border border-(--color-border-subtle) bg-(--color-bg-surface)/40"
							className="space-y-3"
						/>
					}
					error={
						<EmptyState
							title="Saved threads are unavailable"
							description="The API call for `/api/saved` failed. Retry once the local API is healthy again."
						/>
					}
					empty={
						<EmptyState
							title="Save your first thread"
							description="Open a thread and use the save control. It will appear here immediately for quick revisit."
							icon={<Bookmark className="size-5" />}
						/>
					}
				>
					<div className="space-y-3">
						{savedQuery.data?.items.map((item) => (
							<ThreadCard
								key={item.id}
								{...threadCardDataFromSavedItem(item)}
								action={
									<Button
										type="button"
										variant="ghost"
										size="sm"
										onClick={() => deleteMutation.mutate(item.thread_id)}
										disabled={
											deleteMutation.isPending &&
											deleteMutation.variables === item.thread_id
										}
									>
										<BookmarkX className="size-4" />
										Unsave
									</Button>
								}
							/>
						))}
					</div>
				</QueryState>
			</SectionCard>
		</div>
	);
}
