import { ThreadCard } from "@/components/thread-card";
import { ThreadCardActionMenu } from "@/components/thread-card-action-menu";
import type {
	HighlightedItem,
	SavedItem,
	ThreadDetailResponse,
} from "@/lib/api";
import { authQueries, highlightQueries } from "@/lib/queries";
import { buildThreadCardActionKinds } from "@/lib/thread-card-actions";
import type { ThreadCardData } from "@/lib/thread-card-props";
import { useThreadHighlightAction } from "@/lib/thread-highlight";
import { useThreadSaveAction } from "@/lib/thread-save";
import { useQuery } from "@tanstack/react-query";
import { Bookmark, BookmarkX, Pin, PinOff } from "lucide-react";
import { useState } from "react";

interface SavableThreadCardProps extends ThreadCardData {
	savedItem?: SavedItem | null;
	highlightedItem?: HighlightedItem | null;
	isOnline: boolean;
	threadDetail?: ThreadDetailResponse | null;
	savedState?: "saved" | "offline";
	className?: string;
}

export function SavableThreadCard({
	savedItem,
	highlightedItem,
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
	const highlightMutation = useThreadHighlightAction({
		threadId: card.threadId,
		highlightedItem,
	});
	const currentUserQuery = useQuery({
		...authQueries.me(),
		enabled: isOnline,
	});
	const allHighlightsQuery = useQuery({
		...highlightQueries.list(),
		enabled: isOnline && highlightedItem === undefined,
	});
	const resolvedHighlightedItem =
		highlightedItem ??
		allHighlightsQuery.data?.items.find(
			(item) => item.thread_id === card.threadId,
		) ??
		null;
	const isSaved = Boolean(savedItem);
	const isHighlighted = Boolean(resolvedHighlightedItem);
	const isAdmin = currentUserQuery.data?.user.roles.includes("admin") ?? false;
	const resolvedSavedState = savedState ?? (isSaved ? "saved" : undefined);
	const actionKinds = buildThreadCardActionKinds({
		isOnline,
		isAdmin,
		isSaved,
		isHighlighted,
	});
	const menuActions = actionKinds.menu.map((kind) => {
		const action = buildCardAction(kind, {
			isSaved,
			isHighlighted,
			saveMutationPending: saveMutation.isPending,
			highlightMutationPending: highlightMutation.isPending,
			onSaveToggle: () => saveMutation.mutate(),
			onHighlightToggle: () => highlightMutation.mutate(),
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
			isHighlighted={isHighlighted}
			leadingSwipeActions={actionKinds.leading.map((kind) =>
				buildCardAction(kind, {
					isSaved,
					isHighlighted,
					saveMutationPending: saveMutation.isPending,
					highlightMutationPending: highlightMutation.isPending,
					onSaveToggle: () => saveMutation.mutate(),
					onHighlightToggle: () => highlightMutation.mutate(),
				}),
			)}
			trailingSwipeActions={actionKinds.trailing.map((kind) =>
				buildCardAction(kind, {
					isSaved,
					isHighlighted,
					saveMutationPending: saveMutation.isPending,
					highlightMutationPending: highlightMutation.isPending,
					onSaveToggle: () => saveMutation.mutate(),
					onHighlightToggle: () => highlightMutation.mutate(),
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
		isHighlighted,
		saveMutationPending,
		highlightMutationPending,
		onSaveToggle,
		onHighlightToggle,
	}: {
		isSaved: boolean;
		isHighlighted: boolean;
		saveMutationPending: boolean;
		highlightMutationPending: boolean;
		onSaveToggle: () => void;
		onHighlightToggle: () => void;
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
		case "unhighlight":
			return {
				label:
					highlightMutationPending && isHighlighted
						? "Removing"
						: "Remove highlight",
				icon: <PinOff className="size-4" />,
				onAction: onHighlightToggle,
				disabled: highlightMutationPending,
				tone: "danger" as const,
			};
		case "highlight":
			return {
				label:
					highlightMutationPending && !isHighlighted
						? "Pinning"
						: "Pin to highlights",
				icon: <Pin className="size-4" />,
				onAction: onHighlightToggle,
				disabled: highlightMutationPending,
				tone: "accent" as const,
			};
	}
}
