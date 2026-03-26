export type ThreadCardActionKind = "save" | "unsave" | "star" | "unstar";

export function buildThreadCardActionKinds({
	isOnline,
	isAdmin,
	isSaved,
	isStarred,
}: {
	isOnline: boolean;
	isAdmin: boolean;
	isSaved: boolean;
	isStarred: boolean;
}) {
	const leading: ThreadCardActionKind[] = [];
	const trailing: ThreadCardActionKind[] = [];
	const menu: ThreadCardActionKind[] = [];
	const quick: ThreadCardActionKind[] = [];

	if (!isOnline) {
		return { leading, trailing, menu, quick };
	}

	if (isSaved) {
		leading.push("unsave");
		menu.push("unsave");
		quick.push("unsave");
	} else {
		trailing.push("save");
		menu.push("save");
		quick.push("save");
	}

	if (isAdmin) {
		if (isStarred) {
			leading.push("unstar");
			menu.push("unstar");
			quick.push("unstar");
		} else {
			trailing.push("star");
			menu.push("star");
			quick.push("star");
		}
	}

	return { leading, trailing, menu, quick };
}

export function buildThreadShareUrl(origin: string, threadId: string) {
	return new URL(`/threads/${encodeURIComponent(threadId)}`, origin).toString();
}
