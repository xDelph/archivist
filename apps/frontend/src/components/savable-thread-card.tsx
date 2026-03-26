import { ThreadCard } from "@/components/thread-card";
import { ThreadCardActionMenu } from "@/components/thread-card-action-menu";
import { ThreadCardQuickActions } from "@/components/thread-card-quick-actions";
import type { SavedItem, StarredItem, ThreadDetailResponse } from "@/lib/api";
import { authQueries, starredQueries } from "@/lib/queries";
import {
	buildThreadCardActionKinds,
	buildThreadShareUrl,
} from "@/lib/thread-card-actions";
import type { ThreadCardData } from "@/lib/thread-card-props";
import { useThreadSaveAction } from "@/lib/thread-save";
import { useThreadStarAction } from "@/lib/thread-star";
import { useQuery } from "@tanstack/react-query";
import { Bookmark, BookmarkX, Copy, Star, StarOff } from "lucide-react";
import { useEffect, useState } from "react";

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
	const [shareStatus, setShareStatus] = useState<"idle" | "copied" | "failed">(
		"idle",
	);
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

	useEffect(() => {
		if (shareStatus === "idle") {
			return;
		}

		const timeoutId = window.setTimeout(() => setShareStatus("idle"), 1800);
		return () => window.clearTimeout(timeoutId);
	}, [shareStatus]);

	const resolveAction = (
		kind: ReturnType<typeof buildThreadCardActionKinds>["menu"][number],
	) => {
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
			tone:
				action.tone === "danger" ? ("danger" as const) : ("default" as const),
		};
	};
	const quickActions = actionKinds.quick.map(resolveAction);
	const menuActions = [
		{
			key: "share-link",
			label:
				shareStatus === "copied"
					? "Link copied"
					: shareStatus === "failed"
						? "Copy failed"
						: "Copy link",
			icon: <Copy className="size-4" />,
			onSelect: () => {
				void copyThreadShareLink(card.threadId).then((success) => {
					setShareStatus(success ? "copied" : "failed");
				});
			},
			tone: "default" as const,
		},
	];
	const desktopAction =
		menuActions.length || quickActions.length ? (
			<div className="flex items-center gap-1.5">
				<ThreadCardQuickActions actions={quickActions} variant="desktop" />
				{menuActions.length ? (
					<ThreadCardActionMenu
						actions={menuActions}
						onOpenChange={setIsMenuOpen}
					/>
				) : null}
			</div>
		) : null;

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
			action={desktopAction}
		/>
	);
}

async function copyThreadShareLink(threadId: string) {
	if (typeof window === "undefined") {
		return false;
	}

	const shareUrl = buildThreadShareUrl(window.location.origin, threadId);

	if (navigator.clipboard?.writeText) {
		try {
			await navigator.clipboard.writeText(shareUrl);
			return true;
		} catch {
			// Fall back to a temporary textarea when clipboard permissions are denied.
		}
	}

	return copyTextWithSelectionFallback(shareUrl);
}

function copyTextWithSelectionFallback(value: string) {
	if (typeof document === "undefined") {
		return false;
	}

	const element = document.createElement("textarea");
	element.value = value;
	element.setAttribute("readonly", "");
	element.style.position = "absolute";
	element.style.left = "-9999px";
	document.body.appendChild(element);
	element.select();

	try {
		return document.execCommand("copy");
	} catch {
		return false;
	} finally {
		document.body.removeChild(element);
	}
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
				label: starMutationPending && isStarred ? "Removing" : "Unstar thread",
				icon: <StarOff className="size-4" />,
				onAction: onStarToggle,
				disabled: starMutationPending,
				tone: "danger" as const,
			};
		case "star":
			return {
				label: starMutationPending && !isStarred ? "Starring" : "Star thread",
				icon: <Star className="size-4" />,
				onAction: onStarToggle,
				disabled: starMutationPending,
				tone: "accent" as const,
			};
	}
}
