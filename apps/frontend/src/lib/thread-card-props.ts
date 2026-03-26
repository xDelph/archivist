import type {
	CatchUpThread,
	SavedItem,
	SearchResult,
	StarredItem,
	ThreadAuthor,
	ThreadPreviewSource,
} from "@/lib/api";
import type { ReactNode } from "react";

export interface ThreadCardData {
	threadId: string;
	channelName?: string | null;
	author?: ThreadAuthor | null;
	authorFallback?: string | null;
	title: ReactNode;
	preview: ReactNode;
	previewSource?: ThreadPreviewSource;
	lastActivityTs: string;
	replyCount: number;
	participantCount: number;
	reactionCount: number;
	fileCount?: number;
}

export function threadCardDataFromCatchUpThread(
	thread: CatchUpThread,
	channelName?: string | null,
): ThreadCardData {
	return {
		threadId: thread.id,
		channelName: channelName ?? thread.channel_name ?? thread.channel_id,
		author: thread.author,
		title: thread.title,
		preview: thread.summary_preview ?? thread.preview,
		previewSource: thread.preview_source,
		lastActivityTs: thread.last_activity_ts,
		replyCount: thread.reply_count,
		participantCount: thread.participant_count,
		reactionCount: thread.reaction_count,
		fileCount: thread.file_count,
	};
}

export function threadCardDataFromSearchResult(
	item: SearchResult,
	content: {
		title?: ReactNode;
		preview?: ReactNode;
	} = {},
): ThreadCardData {
	const title = content.title ?? item.title;
	const preview = content.preview ?? item.preview;

	return {
		threadId: item.thread_id,
		channelName: item.channel_name || item.channel_id,
		author: item.author,
		title,
		preview: item.summary_preview ?? preview,
		previewSource: item.preview_source,
		lastActivityTs: item.message_ts,
		replyCount: item.reply_count ?? 0,
		participantCount: item.participant_count ?? 0,
		reactionCount: item.reaction_count ?? 0,
		fileCount: item.file_count ?? 0,
	};
}

export function threadCardDataFromSavedItem(item: SavedItem): ThreadCardData {
	return {
		threadId: item.thread_id,
		channelName: item.channel_name || item.channel_id,
		author: item.author,
		title: item.title,
		preview: item.summary_preview ?? item.preview,
		previewSource: item.preview_source,
		lastActivityTs: item.last_activity_ts,
		replyCount: item.reply_count,
		participantCount: item.participant_count,
		reactionCount: item.reaction_count,
		fileCount: item.file_count,
	};
}

export function threadCardDataFromStarredItem(
	item: StarredItem,
): ThreadCardData {
	return {
		threadId: item.thread_id,
		channelName: item.channel_name || item.channel_id,
		author: item.author,
		title: item.title,
		preview: item.summary_preview ?? item.preview,
		previewSource: item.preview_source,
		lastActivityTs: item.last_activity_ts,
		replyCount: item.reply_count,
		participantCount: item.participant_count,
		reactionCount: item.reaction_count,
		fileCount: item.file_count,
	};
}
