import type {
	CatchUpResponse,
	SavedItem,
	StarredItem,
	ThreadDetailResponse,
} from "@/lib/api";
import type { InfiniteData } from "@tanstack/react-query";

export function getFreshestKnownThreadActivityTs(
	threadId: string,
	snapshots: unknown[],
) {
	let freshest: string | null = null;

	for (const snapshot of snapshots) {
		for (const timestamp of extractThreadActivityTimestamps(
			threadId,
			snapshot,
		)) {
			if (!freshest || compareSlackTimestamps(timestamp, freshest) > 0) {
				freshest = timestamp;
			}
		}
	}

	return freshest;
}

export function shouldRefreshThreadDetail(
	thread: ThreadDetailResponse | undefined,
	freshestKnownActivityTs: string | null | undefined,
) {
	if (!thread || !freshestKnownActivityTs) {
		return false;
	}

	return (
		compareSlackTimestamps(
			freshestKnownActivityTs,
			getThreadObservedActivityTs(thread),
		) > 0
	);
}

export function getThreadObservedActivityTs(thread: ThreadDetailResponse) {
	const latestMessageTs = thread.messages.reduce<string | null>(
		(currentLatest, message) =>
			!currentLatest || compareSlackTimestamps(message.ts, currentLatest) > 0
				? message.ts
				: currentLatest,
		null,
	);

	return thread.last_activity_ts || latestMessageTs || thread.root_ts;
}

export function compareSlackTimestamps(left: string, right: string) {
	const [leftSeconds, leftFraction] = normalizeSlackTimestamp(left);
	const [rightSeconds, rightFraction] = normalizeSlackTimestamp(right);

	if (leftSeconds !== rightSeconds) {
		return leftSeconds - rightSeconds;
	}

	return leftFraction - rightFraction;
}

function extractThreadActivityTimestamps(threadId: string, snapshot: unknown) {
	if (isCatchUpInfiniteData(snapshot)) {
		return snapshot.pages.flatMap((page) =>
			page.items
				.filter((item) => item.id === threadId)
				.map((item) => item.last_activity_ts),
		);
	}

	if (isSavedItemsResponse(snapshot) || isHighlightsResponse(snapshot)) {
		return snapshot.items
			.filter((item) => item.thread_id === threadId)
			.map((item) => item.last_activity_ts);
	}

	return [];
}

function normalizeSlackTimestamp(value: string) {
	const [seconds, fraction = "0"] = value.trim().split(".", 2);
	return [Number(seconds || 0), Number(fraction.padEnd(6, "0"))] as const;
}

function isCatchUpInfiniteData(
	value: unknown,
): value is InfiniteData<CatchUpResponse> {
	return Boolean(
		value &&
			typeof value === "object" &&
			Array.isArray((value as InfiniteData<CatchUpResponse>).pages),
	);
}

function isSavedItemsResponse(value: unknown): value is {
	items: SavedItem[];
} {
	return hasLastActivityItems(value);
}

function isHighlightsResponse(value: unknown): value is {
	items: StarredItem[];
} {
	return hasLastActivityItems(value);
}

function hasLastActivityItems(value: unknown): value is {
	items: Array<{
		thread_id: string;
		last_activity_ts: string;
	}>;
} {
	return Boolean(
		value &&
			typeof value === "object" &&
			Array.isArray(
				(
					value as {
						items?: unknown[];
					}
				).items,
			),
	);
}
