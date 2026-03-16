import { ThreadCard } from "@/components/thread-card";
import { Button } from "@/components/ui/button";
import type { SavedItem, ThreadDetailResponse } from "@/lib/api";
import type { ThreadCardData } from "@/lib/thread-card-props";
import { useThreadSaveAction } from "@/lib/thread-save";
import { Bookmark, BookmarkCheck, BookmarkX } from "lucide-react";

interface SavableThreadCardProps extends ThreadCardData {
	savedItem?: SavedItem | null;
	isOnline: boolean;
	threadDetail?: ThreadDetailResponse | null;
	savedState?: "saved" | "offline";
	className?: string;
}

export function SavableThreadCard({
	savedItem,
	isOnline,
	threadDetail,
	savedState,
	className,
	...card
}: SavableThreadCardProps) {
	const saveMutation = useThreadSaveAction({
		threadId: card.threadId,
		savedItem,
		threadDetail,
	});
	const isSaved = Boolean(savedItem);
	const resolvedSavedState = savedState ?? (isSaved ? "saved" : undefined);
	const actionLabel = !isOnline
		? isSaved
			? "Saved offline"
			: "Offline"
		: saveMutation.isPending
			? isSaved
				? "Removing"
				: "Saving"
			: isSaved
				? "Unsave"
				: "Save";

	return (
		<ThreadCard
			{...card}
			className={className}
			savedState={resolvedSavedState}
			leadingSwipeAction={
				isOnline && isSaved
					? {
							label: actionLabel,
							icon: <BookmarkX className="size-4" />,
							onAction: () => saveMutation.mutate(),
							disabled: saveMutation.isPending,
							tone: "danger",
						}
					: undefined
			}
			trailingSwipeAction={
				isOnline && !isSaved
					? {
							label: actionLabel,
							icon: <Bookmark className="size-4" />,
							onAction: () => saveMutation.mutate(),
							disabled: saveMutation.isPending,
							tone: "accent",
						}
					: undefined
			}
			action={
				isOnline ? (
					<Button
						type="button"
						variant={isSaved ? "default" : "secondary"}
						size="sm"
						className={
							isSaved
								? "hidden size-9 rounded-full bg-(--color-accent) px-0 text-black hover:bg-(--color-accent-strong) sm:inline-flex"
								: "button-ghost hidden size-9 rounded-full px-0 sm:inline-flex"
						}
						onClick={() => saveMutation.mutate()}
						disabled={saveMutation.isPending}
						title={actionLabel}
					>
						{isSaved ? (
							<BookmarkCheck className="size-4" />
						) : (
							<Bookmark className="size-4" />
						)}
					</Button>
				) : null
			}
		/>
	);
}
