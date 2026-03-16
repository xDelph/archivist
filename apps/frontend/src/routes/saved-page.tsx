import { EmptyState } from "@/components/empty-state";
import { CardSkeletonList, QueryState } from "@/components/query-state";
import { SavableThreadCard } from "@/components/savable-thread-card";
import { SectionCard } from "@/components/section-card";
import {
	filterSavedItemsWithSnapshots,
	resolveSavedItemsForReading,
} from "@/lib/offline-reading";
import { offlineQueries, savedQueries } from "@/lib/queries";
import { threadCardDataFromSavedItem } from "@/lib/thread-card-props";
import { useNetworkStatus } from "@/lib/use-network-status";
import { useQuery } from "@tanstack/react-query";
import { Bookmark } from "lucide-react";

export function SavedPage() {
	const { isOnline } = useNetworkStatus();
	const savedQuery = useQuery({
		...savedQueries.list(),
		enabled: isOnline,
	});
	const offlineSavedQuery = useQuery(offlineQueries.saved());
	const offlineThreadIdsQuery = useQuery(offlineQueries.threadIds());
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
							<SavableThreadCard
								key={item.id}
								{...threadCardDataFromSavedItem(item)}
								savedItem={item}
								isOnline={isOnline}
								savedState={isOfflineReading ? "offline" : "saved"}
							/>
						))}
					</div>
				</QueryState>
			</SectionCard>
		</div>
	);
}
