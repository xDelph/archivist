import { ThreadCard } from "@/components/thread-card";
import { ThreadCardActionMenu } from "@/components/thread-card-action-menu";
import type {
	SavedItem,
	StarredItem,
	ThreadDetailResponse,
} from "@/lib/api";
import { authQueries, starredQueries } from "@/lib/queries";
import { buildThreadCardActionKinds } from "@/lib/thread-card-actions";
import type { ThreadCardData } from "@/lib/thread-card-props";
import { useThreadStarAction } from "@/lib/thread-star";
import { useThreadSaveAction } from "@/lib/thread-save";
import { useQuery } from "@tanstack/react-query";
import { Bookmark, BookmarkX, Star, StarOff } from "lucide-react";
import { useState } from "react";

interface SavableThreadCardProps extends ThreadCardData {
	savedItem?: SavedItem | null;
	starredItem?: StarredItem | null;
	isOnline: boolean;
	threadDetail?: ThreadDetailResponse | null;
	savedState?: "saved" | "offline";
	className?: string;
}

export function SavableThreadCard({
	savedItem,
	starredItem,
	isOnline,
	threadDetail,
	savedState,
	className,
	...card
}: SavableThreadCardProps) {
	const [isMenuOpen, setIsMenuOpen] = useState(false);
	const saveMutation = useThreadSaveAction({
		threadId: card.threadId,
		savedItem,
		threadDetail,
	});
	const starMutation = useThreadStarAction({
		threadId: card.threadId,
		starredItem,
	});
	const currentUserQuery = useQuery({
		...authQueries.me(),
		enabled: isOnline,
	});
	const allStarredQuery = useQuery({
		...starredQueries.list(),
		enabled: isOnline && starredItem === undefined,
	});
	const resolvedStarredItem =
		starredItem ??
		allStarredQuery.data?.items.find(
			(item) => item.thread_id === card.threadId,
		) ??
		null;
	const isSaved = Boolean(savedItem);
	const isStarred = Boolean(resolvedStarredItem);
	const isAdmin = currentUserQuery.data?.user.roles.includes("admin") ?? false;
	const resolvedSavedState = savedState ?? (isSaved ? "saved" : undefined);
	const actionKinds = buildThreadCardActionKinds({
		isOnline,
		isAdmin,
		isSaved,
		isStarred,
	});
	const menuActions = actionKinds.menu.map((kind) => {
		const action = buildCardAction(kind, {
			isSaved,
			isStarred,
			saveMutationPending: saveMutation.isPending,
			starMutationPending: starMutation.isPending,
			onSaveToggle: () => saveMutation.mutate(),
			onStarToggle: () => starMutation.mutate(),
		});
		return {
			key: kind,
			label: action.label,
			icon: action.icon,
			onSelect: action.onAction,
			disabled: action.disabled,
			tone: action.tone === "danger" ? "danger" : "default",
		};
	});

	return (
		<ThreadCard
			{...card}
			className={className}
			isActionActive={isMenuOpen}
			savedState={resolvedSavedState}
			isStarred={isStarred}
			leadingSwipeActions={actionKinds.leading.map((kind) =>
				buildCardAction(kind, {
					isSaved,
					isStarred,
					saveMutationPending: saveMutation.isPending,
					starMutationPending: starMutation.isPending,
					onSaveToggle: () => saveMutation.mutate(),
					onStarToggle: () => starMutation.mutate(),
				}),
			)}
			trailingSwipeActions={actionKinds.trailing.map((kind) =>
				buildCardAction(kind, {
					isSaved,
					isStarred,
					saveMutationPending: saveMutation.isPending,
					starMutationPending: starMutation.isPending,
					onSaveToggle: () => saveMutation.mutate(),
					onStarToggle: () => starMutation.mutate(),
				}),
			)}
			action={
				menuActions.length ? (
					<ThreadCardActionMenu
						actions={menuActions}
						onOpenChange={setIsMenuOpen}
					/>
				) : null
			}
		/>
	);
}

function buildCardAction(
	kind: ReturnType<typeof buildThreadCardActionKinds>["menu"][number],
	{
		isSaved,
		isStarred,
		saveMutationPending,
		starMutationPending,
		onSaveToggle,
		onStarToggle,
	}: {
		isSaved: boolean;
		isStarred: boolean;
		saveMutationPending: boolean;
		starMutationPending: boolean;
		onSaveToggle: () => void;
		onStarToggle: () => void;
	},
) {
	switch (kind) {
		case "unsave":
			return {
				label: saveMutationPending && isSaved ? "Removing" : "Unsave",
				icon: <BookmarkX className="size-4" />,
				onAction: onSaveToggle,
				disabled: saveMutationPending,
				tone: "danger" as const,
			};
		case "save":
			return {
				label: saveMutationPending && !isSaved ? "Saving" : "Save",
				icon: <Bookmark className="size-4" />,
				onAction: onSaveToggle,
				disabled: saveMutationPending,
				tone: "accent" as const,
			};
		case "unstar":
			return {
				label:
					starMutationPending && isStarred
						? "Removing"
						: "Unstar thread",
				icon: <StarOff className="size-4" />,
				onAction: onStarToggle,
				disabled: starMutationPending,
				tone: "danger" as const,
			};
		case "star":
			return {
				label:
					starMutationPending && !isStarred
						? "Starring"
						: "Star thread",
				icon: <Star className="size-4" />,
				onAction: onStarToggle,
				disabled: starMutationPending,
				tone: "accent" as const,
			};
	}
}
