import type { CatchUpChannel, CatchUpResponse, CatchUpThread } from "@/lib/api";

export const CATCH_UP_PAGE_SIZE = 20;

export function flattenCatchUpPages(
	pages: Pick<CatchUpResponse, "items">[],
): CatchUpThread[] {
	return pages.flatMap((page) => page.items);
}

export function getCatchUpChannelLabel(channel: {
	id: string;
	name: string | null;
}) {
	return channel.name || channel.id;
}

export function getCatchUpThreadChannelName(thread: CatchUpThread) {
	return thread.channel_name || thread.channel_id;
}

export function pickChannelHighlights(channels: CatchUpChannel[]) {
	return [...channels]
		.sort((left, right) => right.thread_count - left.thread_count)
		.slice(0, 5);
}
