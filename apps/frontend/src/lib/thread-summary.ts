import type { ThreadDetailResponse, ThreadSummarySource } from "@/lib/api";

interface ThreadSummaryInput {
	title: string | null;
	preview: string | null;
	summary: ThreadDetailResponse["summary"];
	messages: Array<{ text: string }>;
}

export interface ThreadSummaryPresentation {
	primary: string | null;
	secondary: string | null;
	source: ThreadSummarySource;
}

export function resolveThreadSummaryPresentation(
	thread: ThreadSummaryInput,
): ThreadSummaryPresentation {
	const primary = firstNonEmpty(
		thread.summary.text,
		thread.title,
		thread.messages[0]?.text,
	);
	const secondary = firstNonEmpty(
		thread.summary.why_it_mattered,
		distinctFromPrimary(thread.preview, primary),
	);

	return {
		primary,
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

function distinctFromPrimary(
	value: string | null | undefined,
	primary: string | null,
) {
	if (!value) {
		return null;
	}

	return normalizeText(value) === normalizeText(primary) ? null : value;
}

function normalizeText(value: string | null | undefined) {
	return value?.replace(/\s+/g, " ").trim().toLowerCase() ?? "";
}
