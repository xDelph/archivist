import { EmptyState } from "@/components/empty-state";
import { SectionCard } from "@/components/section-card";
import { ThreadCard } from "@/components/thread-card";
import { Button } from "@/components/ui/button";
import { deleteSavedThread, fetchSavedItems } from "@/lib/api";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { BookmarkX } from "lucide-react";

export function SavedPage() {
	const queryClient = useQueryClient();
	const savedQuery = useQuery({
		queryKey: ["saved"],
		queryFn: fetchSavedItems,
	});
	const deleteMutation = useMutation({
		mutationFn: deleteSavedThread,
		onSuccess: async () => {
			await queryClient.invalidateQueries({ queryKey: ["saved"] });
		},
	});

	return (
		<div className="space-y-5">
			<SectionCard
				eyebrow="Saved"
				title="Bookmarked threads"
				description="Threads you explicitly saved are kept here so you can return without rerunning search or catch-up."
			>
				{savedQuery.isPending ? (
					<div className="space-y-3">
						{["saved-skeleton-1", "saved-skeleton-2"].map((key) => (
							<div
								key={key}
								className="h-36 animate-pulse rounded-[1.5rem] border border-white/8 bg-white/[0.04]"
							/>
						))}
					</div>
				) : savedQuery.isError ? (
					<EmptyState
						title="Saved threads are unavailable"
						description="The API call for `/api/saved` failed. Retry once the local API is healthy again."
					/>
				) : savedQuery.data?.items.length ? (
					<div className="space-y-3">
						{savedQuery.data.items.map((item) => (
							<ThreadCard
								key={item.id}
								threadId={item.thread_id}
								channelName={item.channel_name || item.channel_id}
								title={item.title}
								preview={item.preview}
								lastActivityTs={item.last_activity_ts}
								replyCount={0}
								participantCount={0}
								reactionCount={0}
								action={
									<Button
										type="button"
										variant="secondary"
										size="sm"
										onClick={() => deleteMutation.mutate(item.thread_id)}
										disabled={
											deleteMutation.isPending &&
											deleteMutation.variables === item.thread_id
										}
									>
										<BookmarkX className="mr-2 size-4" />
										Unsave
									</Button>
								}
							/>
						))}
					</div>
				) : (
					<EmptyState
						title="Save your first thread"
						description="Open a thread and use the save control. It will appear here immediately for quick revisit."
					/>
				)}
			</SectionCard>
		</div>
	);
}
