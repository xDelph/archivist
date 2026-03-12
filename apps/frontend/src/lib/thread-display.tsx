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
	/(<https?:\/\/[^\s>|]+(?:\|[^>\n]+)?(?:>)?|https?:\/\/[^\s<>()]+|:[a-z0-9_+\-]+:(?::skin-tone-[2-6]:)?)/gi;

export interface SlackLink {
	href: string;
	label: string | null;
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
	const links = new Map<string, SlackLink>();
	for (const segment of text.match(RICH_TOKEN_PATTERN) ?? []) {
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
	const segments = text.split(RICH_TOKEN_PATTERN);
	let cursor = 0;

	return segments.map((segment) => {
		if (!segment) {
			return null;
		}

		const key = `${cursor}-${segment}`;
		cursor += segment.length;

		const slackLink = parseSlackLink(segment);
		if (slackLink) {
			return (
				<a
					key={`link-${key}`}
					href={slackLink.href}
					target="_blank"
					rel="noreferrer"
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
			<span key={`text-${key}`}>
				{renderHighlightedText(segment, query, key)}
			</span>
		);
	});
}

function parseSlackLink(value: string): SlackLink | undefined {
	const normalizedValue = value.trim();
	const slackMatch = normalizedValue.match(
		/^<(?<href>https?:\/\/[^\s>|]+)(?:\|(?<label>[^>\n]+))?>?$/i,
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
		};
	}

	if (/^https?:\/\//i.test(normalizedValue)) {
		const href = sanitizeUrl(normalizedValue);
		if (!href) {
			return undefined;
		}

		return {
			href,
			label: null,
		};
	}

	return undefined;
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

function sanitizeUrl(value: string) {
	return value.replace(/[),.!?]+$/, "");
}

function sanitizeSlackLabel(value: string | undefined) {
	return value?.trim().replace(/>$/, "") || null;
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
