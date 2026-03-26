import { renderSlackTextWithHighlights } from "@/lib/thread-display";

export function highlightMatches(
	text: string | null | undefined,
	query: string,
) {
	return renderSlackTextWithHighlights(text ?? "", query);
}
