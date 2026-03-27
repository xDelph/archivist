import { z } from "zod";

export const threadListSortValues = [
	"date",
	"replies",
	"reactions",
	"people",
] as const;

export const threadListSortSchema = z.enum(threadListSortValues);

export type ThreadListSort = (typeof threadListSortValues)[number];

export const searchThreadSortValues = [
	"relevance",
	...threadListSortValues,
] as const;

export const searchThreadSortSchema = z.enum(searchThreadSortValues);

export type SearchThreadSort = (typeof searchThreadSortValues)[number];

export interface ThreadListSortOption<T extends string> {
	label: string;
	value: T;
}

export const threadListSortOptions: readonly ThreadListSortOption<ThreadListSort>[] =
	[
		{ value: "date", label: "Date" },
		{ value: "replies", label: "Replies" },
		{ value: "reactions", label: "Reactions" },
		{ value: "people", label: "People" },
	] as const;

export const searchThreadSortOptions: readonly ThreadListSortOption<SearchThreadSort>[] =
	[
		{ value: "relevance", label: "Relevance" },
		...threadListSortOptions,
	] as const;

export function normalizeThreadListSort(value?: string): ThreadListSort {
	return threadListSortSchema.catch("date").parse(value);
}

export function toThreadListSortSearch(sort: ThreadListSort) {
	return sort === "date" ? undefined : sort;
}

export function normalizeSearchThreadSort(value?: string): SearchThreadSort {
	return searchThreadSortSchema.catch("relevance").parse(value);
}

export function toSearchThreadSortSearch(sort: SearchThreadSort) {
	return sort === "relevance" ? undefined : sort;
}

export interface ThreadListSortMetrics {
	id: string;
	lastActivityTs: string;
	participantCount: number;
	reactionCount: number;
	replyCount: number;
	rootTs?: string | null;
}

export function sortThreadList<T>(
	items: readonly T[],
	sort: ThreadListSort,
	getMetrics: (item: T) => ThreadListSortMetrics,
) {
	return [...items].sort((left, right) =>
		compareThreadListMetrics(getMetrics(left), getMetrics(right), sort),
	);
}

function compareThreadListMetrics(
	left: ThreadListSortMetrics,
	right: ThreadListSortMetrics,
	sort: ThreadListSort,
) {
	switch (sort) {
		case "date":
			return compareMetricTuples(
				[
					slackTsToSeconds(right.lastActivityTs),
					slackTsToSeconds(right.rootTs),
					right.replyCount,
					right.reactionCount,
					right.participantCount,
					left.id,
				],
				[
					slackTsToSeconds(left.lastActivityTs),
					slackTsToSeconds(left.rootTs),
					left.replyCount,
					left.reactionCount,
					left.participantCount,
					right.id,
				],
			);
		case "replies":
			return compareMetricTuples(
				[
					right.replyCount,
					right.reactionCount,
					right.participantCount,
					slackTsToSeconds(right.lastActivityTs),
					slackTsToSeconds(right.rootTs),
					left.id,
				],
				[
					left.replyCount,
					left.reactionCount,
					left.participantCount,
					slackTsToSeconds(left.lastActivityTs),
					slackTsToSeconds(left.rootTs),
					right.id,
				],
			);
		case "reactions":
			return compareMetricTuples(
				[
					right.reactionCount,
					right.replyCount,
					right.participantCount,
					slackTsToSeconds(right.lastActivityTs),
					slackTsToSeconds(right.rootTs),
					left.id,
				],
				[
					left.reactionCount,
					left.replyCount,
					left.participantCount,
					slackTsToSeconds(left.lastActivityTs),
					slackTsToSeconds(left.rootTs),
					right.id,
				],
			);
		case "people":
			return compareMetricTuples(
				[
					right.participantCount,
					right.replyCount,
					right.reactionCount,
					slackTsToSeconds(right.lastActivityTs),
					slackTsToSeconds(right.rootTs),
					left.id,
				],
				[
					left.participantCount,
					left.replyCount,
					left.reactionCount,
					slackTsToSeconds(left.lastActivityTs),
					slackTsToSeconds(left.rootTs),
					right.id,
				],
			);
	}
}

function compareMetricTuples(
	left: [number, number, number, number, number, string],
	right: [number, number, number, number, number, string],
) {
	return left[0] !== right[0]
		? left[0] - right[0]
		: left[1] !== right[1]
			? left[1] - right[1]
			: left[2] !== right[2]
				? left[2] - right[2]
				: left[3] !== right[3]
					? left[3] - right[3]
					: left[4] !== right[4]
						? left[4] - right[4]
						: left[5].localeCompare(right[5]);
}

function slackTsToSeconds(value?: string | null) {
	return value?.split(".").at(0)?.trim()
		? Number.parseInt(value.split(".")[0] ?? "0", 10) || 0
		: 0;
}
