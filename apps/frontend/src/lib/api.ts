import type { SearchThreadSort, ThreadListSort } from "@/lib/thread-list-sort";

export interface ApiHealth {
	service: string;
	version: string;
	workspace_mode?: string;
	repository_mode?: string;
	search_backend?: string;
}

export interface CurrentUser {
	slack_user_id: string;
	email: string | null;
	display_name: string | null;
	avatar_url: string | null;
	roles: string[];
	is_active: boolean;
	is_anonymized: boolean;
}

export interface AdminUser {
	slack_user_id: string;
	email: string | null;
	display_name: string | null;
	is_active: boolean;
	is_anonymized: boolean;
	roles: string[];
}

export interface AdminUsersResponse {
	ok: boolean;
	users: AdminUser[];
}

export interface ThreadAuthor {
	slack_user_id: string;
	display_name: string | null;
	avatar_url: string | null;
}

export interface CurrentUserResponse {
	ok: boolean;
	user: CurrentUser;
}

export interface CatchUpResponse {
	window: "24h" | "7d";
	channels: CatchUpChannel[];
	items: CatchUpThread[];
	next_cursor: string | null;
}

export interface CatchUpChannel {
	id: string;
	name: string | null;
	kind: string;
	is_archived: boolean;
	thread_count: number;
}

export interface CatchUpThread {
	id: string;
	channel_id: string;
	channel_name: string | null;
	root_ts: string;
	author: ThreadAuthor | null;
	title: string;
	preview: string;
	summary_preview: string | null;
	preview_source: ThreadPreviewSource;
	reply_count: number;
	participant_count: number;
	reaction_count: number;
	file_count: number;
	last_activity_ts: string;
}

export interface ChannelSummary {
	id: string;
	name: string | null;
	kind: string;
	is_archived: boolean;
	message_count: number;
	reaction_count: number;
	file_count: number;
	last_message_ts: string | null;
}

export interface SearchResponse {
	query: string;
	items: SearchResult[];
	next_cursor: string | null;
}

export interface SearchResult {
	id: string;
	thread_id: string;
	channel_id: string;
	channel_name: string | null;
	author: ThreadAuthor | null;
	root_ts: string;
	message_ts: string;
	title: string;
	preview: string;
	snippet: string;
	summary_preview: string | null;
	preview_source: ThreadPreviewSource;
	last_activity_ts: string;
	reply_count: number;
	participant_count: number;
	reaction_count: number;
	file_count: number;
	score: number;
}

export interface LinkMetadata {
	url: string;
	title: string | null;
	description: string | null;
	site_name: string | null;
	image: string | null;
}

export type ThreadSummarySource = "ai" | "fallback" | "none";
export type ThreadPreviewSource = Extract<
	ThreadSummarySource,
	"ai" | "fallback"
>;

export interface ThreadSummaryBlock {
	text: string | null;
	full_summary: string | null;
	why_it_mattered: string | null;
	status: string | null;
	topic_tags: string[];
	model: string | null;
	generated_at: number | null;
	is_stale: boolean;
	source: ThreadSummarySource;
}

export interface ThreadDetailResponse {
	id: string;
	channel_id: string;
	channel_name: string | null;
	root_ts: string;
	title: string | null;
	preview: string | null;
	last_activity_ts: string | null;
	reply_count: number;
	participant_count: number;
	reaction_count: number;
	file_count: number;
	summary: ThreadSummaryBlock;
	messages: ThreadMessage[];
}

export interface ThreadMessage {
	ts: string;
	thread_ts: string | null;
	user_id: string | null;
	author: ThreadAuthor | null;
	text: string;
	reactions: ThreadReaction[];
	files: ThreadFile[];
}

export interface ThreadReaction {
	user_id: string;
	name: string;
}

export interface SavedItemsResponse {
	items: SavedItem[];
}

export interface StarredResponse {
	items: StarredItem[];
}

export interface SavedItem {
	id: string;
	thread_id: string;
	channel_id: string;
	channel_name: string | null;
	author: ThreadAuthor | null;
	root_ts: string;
	title: string;
	preview: string;
	summary_preview: string | null;
	preview_source: ThreadPreviewSource;
	reply_count: number;
	participant_count: number;
	reaction_count: number;
	file_count: number;
	last_activity_ts: string;
	saved_at: string;
}

export interface StarredItem {
	id: string;
	thread_id: string;
	channel_id: string;
	channel_name: string | null;
	author: ThreadAuthor | null;
	root_ts: string;
	title: string;
	preview: string;
	summary_preview: string | null;
	preview_source: ThreadPreviewSource;
	reply_count: number;
	participant_count: number;
	reaction_count: number;
	file_count: number;
	last_activity_ts: string;
	pinned_at: string;
}

export type CatchUpWindow = "24h" | "7d";
export type CatchUpSort = ThreadListSort;

export interface CatchUpParams {
	window: CatchUpWindow;
	channelId?: string;
	cursor?: string;
	limit?: number;
	sort?: CatchUpSort;
}

export interface ThreadFile {
	id: string;
	name: string;
	mimetype: string | null;
	permalink: string | null;
	size: number | null;
}

interface ErrorPayload {
	error?: string;
}

const DEFAULT_API_BASE_URL = "";

export class ApiError extends Error {
	status: number;
	code?: string;

	constructor(message: string, status: number, code?: string) {
		super(message);
		this.name = "ApiError";
		this.status = status;
		this.code = code;
	}
}

export function resolveApiBaseUrl(baseUrl = import.meta.env.VITE_API_BASE_URL) {
	const candidate = baseUrl?.trim() ?? DEFAULT_API_BASE_URL;
	return candidate.replace(/\/+$/, "");
}

export function buildApiUrl(path: string, baseUrl = resolveApiBaseUrl()) {
	const normalizedPath = path.startsWith("/") ? path : `/${path}`;
	return baseUrl ? `${baseUrl}${normalizedPath}` : normalizedPath;
}

export function isApiErrorWithStatus(
	error: unknown,
	status: number,
): error is ApiError {
	return error instanceof ApiError && error.status === status;
}

export function slackAuthStartUrl(baseUrl = resolveApiBaseUrl()) {
	return buildApiUrl("/api/auth/slack/start", baseUrl);
}

export async function fetchApiHealth() {
	return apiRequest<ApiHealth>("/health", { credentials: "omit" });
}

export async function fetchCurrentUser() {
	return apiRequest<CurrentUserResponse>("/api/auth/me");
}

export async function logoutCurrentUser() {
	return apiRequest<{ ok: boolean }>("/api/auth/logout", {
		method: "POST",
	});
}

export async function anonymizeCurrentUser() {
	return apiRequest<{ ok: boolean }>("/api/auth/anonymize", {
		method: "POST",
		headers: {
			"Content-Type": "application/json",
		},
		body: JSON.stringify({ confirm: true }),
	});
}

export async function deAnonymizeCurrentUser() {
	return apiRequest<{ ok: boolean }>("/api/auth/de-anonymize", {
		method: "POST",
		headers: {
			"Content-Type": "application/json",
		},
		body: JSON.stringify({ confirm: true }),
	});
}

export async function fetchAdminUsers(query?: string) {
	const params = new URLSearchParams();
	if (query?.trim()) {
		params.set("query", query.trim());
	}
	const search = params.toString();
	return apiRequest<AdminUsersResponse>(
		search ? `/api/admin/users?${search}` : "/api/admin/users",
	);
}

export async function adminAnonymizeUser(slackUserId: string) {
	return adminUserAction(slackUserId, "anonymize");
}

export async function adminDeAnonymizeUser(slackUserId: string) {
	return adminUserAction(slackUserId, "de-anonymize");
}

export async function adminDeactivateUser(slackUserId: string) {
	return adminUserAction(slackUserId, "deactivate");
}

export async function adminReactivateUser(slackUserId: string) {
	return adminUserAction(slackUserId, "reactivate");
}

async function adminUserAction(slackUserId: string, action: string) {
	return apiRequest<{ ok: boolean }>(
		`/api/admin/users/${encodeURIComponent(slackUserId)}/${action}`,
		{
			method: "POST",
			headers: {
				"Content-Type": "application/json",
			},
			body: JSON.stringify({ confirm: true }),
		},
	);
}

export async function fetchCatchUp({
	window,
	channelId,
	cursor,
	limit,
	sort = "date",
}: CatchUpParams) {
	const params = new URLSearchParams({
		window,
		sort,
	});
	if (channelId) {
		params.set("channel_id", channelId);
	}
	if (cursor) {
		params.set("cursor", cursor);
	}
	if (limit !== undefined) {
		params.set("limit", String(limit));
	}

	return apiRequest<CatchUpResponse>(`/api/catch-up?${params.toString()}`);
}

export async function fetchChannels() {
	return apiRequest<ChannelSummary[]>("/api/channels");
}

export interface SearchParams {
	query: string;
	channelId?: string;
	dateFrom?: string;
	dateTo?: string;
	sort?: SearchThreadSort;
	cursor?: string;
	limit?: number;
}

export async function fetchSearchResults({
	query,
	channelId,
	dateFrom,
	dateTo,
	sort = "relevance",
	cursor,
	limit,
}: SearchParams) {
	const params = new URLSearchParams({
		q: query,
		sort,
	});
	if (channelId) {
		params.set("channel_id", channelId);
	}
	if (dateFrom) {
		params.set("date_from", dateFrom);
	}
	if (dateTo) {
		params.set("date_to", dateTo);
	}
	if (cursor) {
		params.set("cursor", cursor);
	}
	if (limit !== undefined) {
		params.set("limit", String(limit));
	}

	return apiRequest<SearchResponse>(`/api/search?${params.toString()}`);
}

export async function fetchLinkMetadata(url: string) {
	const params = new URLSearchParams({ url });
	return apiRequest<LinkMetadata>(`/api/link-metadata?${params.toString()}`);
}

export async function fetchThreadDetail(threadId: string) {
	return apiRequest<ThreadDetailResponse>(
		`/api/threads/${encodeURIComponent(threadId)}`,
	);
}

export async function fetchSavedItems() {
	return apiRequest<SavedItemsResponse>("/api/saved");
}

export async function fetchStarredItems(channelId?: string) {
	const params = new URLSearchParams();
	if (channelId) {
		params.set("channel_id", channelId);
	}
	const search = params.toString();
	return apiRequest<StarredResponse>(
		search ? `/api/highlights?${search}` : "/api/highlights",
	);
}

export async function starThread(threadId: string) {
	return apiRequest<{ ok: boolean; item: StarredItem }>("/api/highlights", {
		method: "POST",
		headers: {
			"Content-Type": "application/json",
		},
		body: JSON.stringify({ thread_id: threadId }),
	});
}

export async function unstarThread(threadId: string) {
	return apiRequest<{ ok: boolean }>(
		`/api/highlights/${encodeURIComponent(threadId)}`,
		{
			method: "DELETE",
		},
	);
}

export async function saveThread(threadId: string) {
	return apiRequest<{ ok: boolean; item: SavedItem }>("/api/saved", {
		method: "POST",
		headers: {
			"Content-Type": "application/json",
		},
		body: JSON.stringify({ thread_id: threadId }),
	});
}

export async function deleteSavedThread(threadId: string) {
	return apiRequest<{ ok: boolean }>(
		`/api/saved/${encodeURIComponent(threadId)}`,
		{
			method: "DELETE",
		},
	);
}

async function apiRequest<T>(path: string, init?: RequestInit): Promise<T> {
	const response = await fetch(buildApiUrl(path), {
		credentials: "include",
		...init,
		headers: {
			Accept: "application/json",
			...(init?.headers ?? {}),
		},
	});

	if (!response.ok) {
		let errorPayload: ErrorPayload | undefined;
		try {
			errorPayload = (await response.json()) as ErrorPayload;
		} catch {
			errorPayload = undefined;
		}

		throw new ApiError(
			errorPayload?.error ?? `Request failed with status ${response.status}`,
			response.status,
			errorPayload?.error,
		);
	}

	return (await response.json()) as T;
}
