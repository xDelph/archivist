import type { ThreadDetailResponse, ThreadSummarySource } from "@/lib/api";

interface ThreadSummaryInput {
	title: string | null;
	preview: string | null;
	summary: ThreadDetailResponse["summary"];
	messages: Array<{ text: string }>;
}

export interface ThreadSummaryPresentation {
	body: string | null;
	bodyFormat: "markdown" | "plain";
	secondary: string | null;
	source: ThreadSummarySource;
}

export function resolveThreadSummaryPresentation(
	thread: ThreadSummaryInput,
): ThreadSummaryPresentation {
	const fullSummary = firstNonEmpty(thread.summary.full_summary);
	const body = fullSummary
		? fullSummary
		: firstNonEmpty(
				thread.summary.text,
				thread.preview,
				thread.messages[0]?.text,
			);
	const secondary = firstNonEmpty(thread.summary.why_it_mattered);

	return {
		body,
		bodyFormat: fullSummary ? "markdown" : "plain",
		secondary,
		source: thread.summary.source,
	};
}

function firstNonEmpty(...values: Array<string | null | undefined>) {
	for (const value of values) {
		if (typeof value !== "string") {
			continue;
		}

		const trimmed = value.trim();
		if (trimmed.length > 0) {
			return trimmed;
		}
	}

	return null;
}
