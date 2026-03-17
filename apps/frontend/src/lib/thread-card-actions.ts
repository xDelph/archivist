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

	if (!isOnline) {
		return { leading, trailing, menu };
	}

	if (isSaved) {
		leading.push("unsave");
		menu.push("unsave");
	} else {
		trailing.push("save");
		menu.push("save");
	}

	if (isAdmin) {
		if (isStarred) {
			leading.push("unstar");
			menu.push("unstar");
		} else {
			trailing.push("star");
			menu.push("star");
		}
	}

	return { leading, trailing, menu };
}
