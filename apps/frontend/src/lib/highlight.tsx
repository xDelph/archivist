import type { ReactNode } from "react";

export function highlightMatches(text: string, query: string): ReactNode {
	const normalizedQuery = query.trim();
	if (!normalizedQuery) {
		return text;
	}

	const matcher = new RegExp(`(${escapeRegExp(normalizedQuery)})`, "ig");
	const parts = text.split(matcher);
	let cursor = 0;

	return parts.map((part) => {
		if (!part) {
			return null;
		}
		const key = `${part}-${cursor}`;
		cursor += part.length;

		if (part.toLowerCase() === normalizedQuery.toLowerCase()) {
			return (
				<mark
					key={key}
					className="rounded bg-[var(--accent-soft)]/20 px-1 text-[var(--accent-soft)]"
				>
					{part}
				</mark>
			);
		}

		return <span key={key}>{part}</span>;
	});
}

function escapeRegExp(value: string) {
	return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}
