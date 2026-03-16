import { EmptyState } from "@/components/empty-state";
import { CardSkeletonList, QueryState } from "@/components/query-state";
import { SectionCard } from "@/components/section-card";
import { ThreadCard } from "@/components/thread-card";
import { Button } from "@/components/ui/button";
import { deleteSavedThread } from "@/lib/api";
import { removeOfflineSavedThread } from "@/lib/offline-library";
import {
	filterSavedItemsWithSnapshots,
	resolveSavedItemsForReading,
} from "@/lib/offline-reading";
import { offlineQueries, savedQueries } from "@/lib/queries";
import { threadCardDataFromSavedItem } from "@/lib/thread-card-props";
import { useNetworkStatus } from "@/lib/use-network-status";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { Bookmark, BookmarkX } from "lucide-react";

export function SavedPage() {
	const { isOnline } = useNetworkStatus();
	const queryClient = useQueryClient();
	const savedQuery = useQuery({
		...savedQueries.list(),
		enabled: isOnline,
	});
	const offlineSavedQuery = useQuery(offlineQueries.saved());
	const offlineThreadIdsQuery = useQuery(offlineQueries.threadIds());
	const deleteMutation = useMutation({
		mutationFn: deleteSavedThread,
		onSuccess: async (_data, threadId) => {
			await removeOfflineSavedThread(threadId);
			await queryClient.invalidateQueries({ queryKey: ["saved"] });
			await queryClient.invalidateQueries({ queryKey: ["offline"] });
		},
	});
	const availableItems = resolveSavedItemsForReading(
		savedQuery.data?.items,
		offlineSavedQuery.data,
	);
	const items = isOnline
		? availableItems
		: filterSavedItemsWithSnapshots(availableItems, offlineThreadIdsQuery.data);
	const isOfflineReading = !savedQuery.data?.items.length && items.length > 0;
	const shouldShowError = isOnline && savedQuery.isError && items.length === 0;
	const isPending =
		(isOnline && savedQuery.isPending && offlineSavedQuery.isPending) ||
		(!isOnline &&
			(offlineSavedQuery.isPending || offlineThreadIdsQuery.isPending));

	return (
		<div className="space-y-5">
			<SectionCard eyebrow="Saved" title="Bookmarked threads">
				<QueryState
					isPending={isPending}
					isError={shouldShowError}
					isEmpty={items.length === 0}
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
							title={
								isOnline
									? "Save your first thread"
									: "No offline saved threads yet"
							}
							description={
								isOnline
									? "Open a thread and use the save control. It will appear here immediately for quick revisit."
									: "Save threads while connected and Archivist will keep them available for metro-style offline reading."
							}
							icon={<Bookmark className="size-5" />}
						/>
					}
				>
					<div className="space-y-3">
						{items.map((item) => (
							<ThreadCard
								key={item.id}
								{...threadCardDataFromSavedItem(item)}
								action={
									<Button
										type="button"
										variant="secondary"
										size="sm"
										className="button-ghost size-9 rounded-full px-0 sm:h-8 sm:w-auto sm:rounded-lg sm:px-3 sm:text-[0.74rem]"
										onClick={() => deleteMutation.mutate(item.thread_id)}
										title={
											isOnline ? undefined : "Reconnect to manage saved threads"
										}
										disabled={
											!isOnline ||
											(deleteMutation.isPending &&
												deleteMutation.variables === item.thread_id)
										}
									>
										<BookmarkX className="size-4" />
										<span className="hidden sm:inline">
											{isOfflineReading ? "Saved offline" : "Unsave"}
										</span>
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
