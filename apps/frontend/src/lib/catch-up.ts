import type { CatchUpChannel, CatchUpThread } from "@/lib/api";

export interface CatchUpFeedThread extends CatchUpThread {
	channelName: string;
}

export function flattenCatchUpThreads(
	channels: CatchUpChannel[],
): CatchUpFeedThread[] {
	return channels
		.flatMap((channel) =>
			channel.threads.map((thread) => ({
				...thread,
				channelName: channel.name || channel.id,
			})),
		)
		.sort((left, right) => compareCatchUpThreads(left, right));
}

function compareCatchUpThreads(
	left: CatchUpFeedThread,
	right: CatchUpFeedThread,
) {
	return (
		parseSlackTs(right.last_activity_ts) -
			parseSlackTs(left.last_activity_ts) ||
		right.reply_count - left.reply_count ||
		right.reaction_count - left.reaction_count ||
		right.file_count - left.file_count ||
		left.id.localeCompare(right.id)
	);
}

function parseSlackTs(value: string) {
	return Number.parseFloat(value) || 0;
}
