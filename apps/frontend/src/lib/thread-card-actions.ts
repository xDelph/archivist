export type ThreadCardActionKind =
	| "save"
	| "unsave"
	| "highlight"
	| "unhighlight";

export function buildThreadCardActionKinds({
	isOnline,
	isAdmin,
	isSaved,
	isHighlighted,
}: {
	isOnline: boolean;
	isAdmin: boolean;
	isSaved: boolean;
	isHighlighted: boolean;
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
		if (isHighlighted) {
			leading.push("unhighlight");
			menu.push("unhighlight");
		} else {
			trailing.push("highlight");
			menu.push("highlight");
		}
	}

	return { leading, trailing, menu };
}
