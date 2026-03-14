import { initials } from "@/lib/format";
import { get as getEmoji } from "node-emoji";
import type { ReactNode } from "react";

export interface ThreadAuthor {
	slack_user_id: string;
	display_name: string | null;
	avatar_url: string | null;
}

interface ThreadStats {
	replyCount: number;
	participantCount: number;
	reactionCount: number;
	fileCount: number;
}

const CHANNEL_TONES = [
	"border-[#3b7f44]/40 bg-[#0f2915] text-[#4ade80]",
	"border-[#8f5a22]/40 bg-[#281707] text-[#fb923c]",
	"border-[#0f6f87]/40 bg-[#071f27] text-[#22d3ee]",
	"border-[#7c2552]/40 bg-[#240914] text-[#f472b6]",
	"border-[#5b6318]/40 bg-[#1a1e08] text-[#bef264]",
];

const AVATAR_TONES = [
	"from-[#102d17] to-[#08110b] text-[#61d985]",
	"from-[#2b1708] to-[#120b06] text-[#fb923c]",
	"from-[#102735] to-[#09131a] text-[#7dd3fc]",
	"from-[#2a1022] to-[#14070f] text-[#f9a8d4]",
	"from-[#2c2c0d] to-[#141406] text-[#fde047]",
];

const SLACK_EMOJI_ALIASES: Record<string, string> = {
	thumbsup: "+1",
	thumbsdown: "-1",
	simple_smile: "slightly_smiling_face",
	slightly_smiling_face: "slightly_smiling_face",
	slightly_frowning_face: "slightly_frowning_face",
	open_mouth: "open_mouth",
};

const SKIN_TONE_MODIFIERS: Record<string, string> = {
	"2": "🏻",
	"3": "🏼",
	"4": "🏽",
	"5": "🏾",
	"6": "🏿",
};

const RICH_TOKEN_PATTERN =
	/(<<?(?:https?|mailto):[^\s>|]+(?:\|[^>\n]+)?(?:>>?)?|<(?:https?|mailto):[^\s>|]+(?:\|[^>\n]+)?>|(?:https?|mailto):[^\s<>()]+|:[a-z0-9_+\-]+:(?::skin-tone-[2-6]:)?)/gi;
const MENTION_PATTERN =
	/(?:^|(?<leading>[\s([{"'`]))(?<mention>[@#][^\s.,!?;:()[\]{}<>"']+)/g;

export interface SlackLink {
	href: string;
	label: string | null;
	appHref?: string;
}

export function threadMomentumScore(stats: ThreadStats) {
	return (
		stats.replyCount * 3 +
		stats.participantCount * 2 +
		stats.reactionCount * 2 +
		stats.fileCount * 4
	);
}

export function displayAuthorName(
	author: ThreadAuthor | null | undefined,
	fallback: string | null | undefined,
) {
	return author?.display_name || fallback || "Unknown member";
}

export function authorAvatarTone(key: string | null | undefined) {
	return AVATAR_TONES[hashValue(key || "archivist") % AVATAR_TONES.length];
}

export function channelTone(name: string | null | undefined) {
	return CHANNEL_TONES[hashValue(name || "channel") % CHANNEL_TONES.length];
}

export function authorInitials(
	author: ThreadAuthor | null | undefined,
	fallback: string | null | undefined,
) {
	return initials(displayAuthorName(author, fallback));
}

export function channelLabel(
	name: string | null | undefined,
	fallback = "channel",
) {
	return name?.trim() || fallback;
}

export function extractLinks(text: string) {
	const normalizedText = decodeHtmlEntities(text);
	const links = new Map<string, SlackLink>();
	for (const segment of normalizedText.match(RICH_TOKEN_PATTERN) ?? []) {
		const link = parseSlackLink(segment);
		if (!link || links.has(link.href)) {
			continue;
		}

		links.set(link.href, link);
	}

	return Array.from(links.values());
}

export function renderSlackText(text: string): ReactNode {
	return renderSlackTextWithHighlights(text);
}

export function renderSlackTextWithHighlights(
	text: string,
	query?: string,
): ReactNode {
	const normalizedText = decodeHtmlEntities(text);
	const segments = normalizedText.split(RICH_TOKEN_PATTERN);
	let cursor = 0;

	return segments.map((segment) => {
		if (!segment) {
			return null;
		}

		const key = `${cursor}-${segment}`;
		cursor += segment.length;

		const slackLink = parseSlackLink(segment);
		if (slackLink) {
			const href = slackLink.appHref ?? slackLink.href;
			const isExternal = !href.startsWith("/") && !href.startsWith("mailto:");
			return (
				<a
					key={`link-${key}`}
					href={href}
					target={isExternal ? "_blank" : undefined}
					rel={isExternal ? "noreferrer" : undefined}
					className="break-all [overflow-wrap:anywhere] text-[#5ea7ff] underline decoration-[#2d5cc2] underline-offset-3 transition-colors hover:text-[#89bbff]"
				>
					{renderHighlightedText(slackLink.label || slackLink.href, query, key)}
				</a>
			);
		}

		const emoji = slackEmojiToUnicode(segment);
		if (emoji) {
			return <span key={`emoji-${key}`}>{emoji}</span>;
		}

		return (
			<span
				key={`text-${key}`}
				className="break-words [overflow-wrap:anywhere]"
			>
				{renderMentionText(segment, query, key)}
			</span>
		);
	});
}

function parseSlackLink(value: string): SlackLink | undefined {
	const normalizedValue = value.trim();
	const slackMatch = normalizedValue.match(
		/^<<?(?<href>(?:https?|mailto):[^\s>|]+)(?:\|(?<label>[^>\n]+))?(?:>>?)?$/i,
	);
	if (slackMatch?.groups?.href) {
		const href = sanitizeUrl(slackMatch.groups.href);
		const label = sanitizeSlackLabel(slackMatch.groups.label);
		if (!href) {
			return undefined;
		}

		return {
			href,
			label,
			appHref: slackPermalinkToThreadHref(href),
		};
	}

	if (/^(?:https?|mailto):/i.test(normalizedValue)) {
		const href = sanitizeUrl(normalizedValue);
		if (!href) {
			return undefined;
		}

		return {
			href,
			label: null,
			appHref: slackPermalinkToThreadHref(href),
		};
	}

	return undefined;
}

function slackPermalinkToThreadHref(value: string): string | undefined {
	const url = safeParseUrl(value);
	if (!url || !url.hostname.endsWith(".slack.com")) {
		return undefined;
	}

	const match = url.pathname.match(
		/^\/archives\/(?<channel>[^/]+)\/p(?<ts>\d+)\/?$/,
	);
	const channelId = match?.groups?.channel || url.searchParams.get("cid");
	const rootTs =
		normalizeSlackTimestamp(url.searchParams.get("thread_ts")) ||
		normalizeSlackTimestamp(match?.groups?.ts);
	if (!channelId || !rootTs) {
		return undefined;
	}

	return `/threads/${encodeURIComponent(`${channelId}:${rootTs}`)}`;
}

function normalizeSlackTimestamp(value: string | undefined | null) {
	if (!value) {
		return undefined;
	}

	const trimmed = value.trim();
	if (trimmed.includes(".")) {
		return trimmed;
	}
	if (!/^\d+$/.test(trimmed) || trimmed.length <= 6) {
		return undefined;
	}

	return `${trimmed.slice(0, -6)}.${trimmed.slice(-6)}`;
}

function safeParseUrl(value: string) {
	try {
		return new URL(value);
	} catch {
		return undefined;
	}
}

export function groupReactions(reactions: { name: string; user_id: string }[]) {
	const grouped = new Map<string, number>();
	for (const reaction of reactions) {
		grouped.set(reaction.name, (grouped.get(reaction.name) ?? 0) + 1);
	}

	return Array.from(grouped.entries())
		.map(([name, count]) => ({
			name,
			count,
			emoji: slackEmojiToUnicode(`:${name}:`) ?? slackEmojiToUnicode(name),
		}))
		.sort(
			(left, right) =>
				right.count - left.count || left.name.localeCompare(right.name),
		);
}

export function slackEmojiToUnicode(value: string) {
	const match = value.match(
		/^:?(?<base>[a-z0-9_+\-]+):?(?::skin-tone-(?<tone>[2-6]):?)?$/i,
	);
	if (!match?.groups?.base) {
		return undefined;
	}

	const baseName = match.groups.base.toLowerCase();
	const normalizedBase = SLACK_EMOJI_ALIASES[baseName] ?? baseName;
	const baseEmoji = getEmoji(`:${normalizedBase}:`) ?? getEmoji(normalizedBase);
	if (!baseEmoji) {
		return undefined;
	}

	const tone = match.groups.tone;
	if (!tone) {
		return baseEmoji;
	}

	return `${baseEmoji}${SKIN_TONE_MODIFIERS[tone] ?? ""}`;
}

function renderHighlightedText(
	text: string,
	query: string | undefined,
	keySeed: string,
): ReactNode {
	const normalizedQuery = query?.trim();
	if (!normalizedQuery) {
		return text;
	}

	const matcher = new RegExp(`(${escapeRegExp(normalizedQuery)})`, "ig");
	const parts = text.split(matcher);
	let cursor = 0;

	return parts.map((part) => {
		if (!part) {
			return null;
		}

		const partKey = `${keySeed}-${cursor}-${part}`;
		cursor += part.length;

		if (part.toLowerCase() === normalizedQuery.toLowerCase()) {
			return (
				<mark
					key={partKey}
					className="rounded bg-(--color-accent-soft)/20 px-0.5 text-(--color-accent-soft)"
				>
					{part}
				</mark>
			);
		}

		return <span key={partKey}>{part}</span>;
	});
}

function renderMentionText(
	text: string,
	query: string | undefined,
	keySeed: string,
): ReactNode {
	const parts: ReactNode[] = [];
	let cursor = 0;
	let mentionIndex = 0;

	for (const match of text.matchAll(MENTION_PATTERN)) {
		const leading = match.groups?.leading ?? "";
		const mention = match.groups?.mention;
		if (!mention) {
			continue;
		}
		const fullMatch = match[0];
		const start = match.index ?? 0;
		const mentionStart = start + fullMatch.length - mention.length;

		if (start > cursor) {
			parts.push(
				<span key={`${keySeed}-text-${mentionIndex}-${cursor}`}>
					{renderHighlightedText(
						text.slice(cursor, start),
						query,
						`${keySeed}-${cursor}`,
					)}
				</span>,
			);
		}

		if (leading) {
			const leadingStart = mentionStart - leading.length;
			parts.push(
				<span key={`${keySeed}-lead-${mentionIndex}-${leadingStart}`}>
					{renderHighlightedText(
						text.slice(leadingStart, mentionStart),
						query,
						`${keySeed}-${leadingStart}`,
					)}
				</span>,
			);
		}

		parts.push(
			<span
				key={`${keySeed}-mention-${mentionIndex}-${mentionStart}`}
				className="font-medium text-[#20cb74]"
			>
				{renderHighlightedText(mention, query, `${keySeed}-${mentionStart}`)}
			</span>,
		);

		cursor = mentionStart + mention.length;
		mentionIndex += 1;
	}

	if (cursor < text.length) {
		parts.push(
			<span key={`${keySeed}-tail-${cursor}`}>
				{renderHighlightedText(
					text.slice(cursor),
					query,
					`${keySeed}-${cursor}`,
				)}
			</span>,
		);
	}

	return parts.length ? parts : text;
}

function sanitizeUrl(value: string) {
	return value.replace(/[),.!?]+$/, "");
}

function sanitizeSlackLabel(value: string | undefined) {
	return value?.trim().replace(/>$/, "") || null;
}

function decodeHtmlEntities(value: string) {
	if (!value.includes("&")) {
		return value;
	}

	if (typeof document !== "undefined") {
		const textarea = document.createElement("textarea");
		textarea.innerHTML = value;
		return textarea.value;
	}

	return value
		.replace(/&amp;/g, "&")
		.replace(/&lt;/g, "<")
		.replace(/&gt;/g, ">")
		.replace(/&quot;/g, '"')
		.replace(/&#39;/g, "'")
		.replace(/&#x27;/gi, "'")
		.replace(/&nbsp;/g, " ");
}

function escapeRegExp(value: string) {
	return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function hashValue(value: string) {
	let hash = 0;
	for (const character of value) {
		hash = (hash * 31 + character.charCodeAt(0)) >>> 0;
	}
	return hash;
}
