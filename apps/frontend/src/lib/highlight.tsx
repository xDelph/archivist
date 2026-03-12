import { renderSlackTextWithHighlights } from "@/lib/thread-display";

export function highlightMatches(text: string, query: string) {
	return renderSlackTextWithHighlights(text, query);
}
