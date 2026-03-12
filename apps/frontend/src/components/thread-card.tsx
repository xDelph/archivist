import { ChannelBadge } from "@/components/channel-badge";
import { IdentityAvatar } from "@/components/identity-avatar";
import { formatSlackTimestamp, formatStatLabel } from "@/lib/format";
import {
	type ThreadAuthor,
	displayAuthorName,
	renderSlackText,
} from "@/lib/thread-display";
import { cn } from "@/lib/utils";
import { Link } from "@tanstack/react-router";
import { Heart, MessageSquare, Paperclip, Users } from "lucide-react";
import { Children, type ReactNode, isValidElement } from "react";

interface ThreadCardProps {
	threadId: string;
	channelName?: string | null;
	author?: ThreadAuthor | null;
	authorFallback?: string | null;
	title: ReactNode;
	preview: ReactNode;
	lastActivityTs: string;
	replyCount: number;
	participantCount: number;
	reactionCount: number;
	fileCount?: number;
	score?: number;
	rank?: number;
	className?: string;
	action?: ReactNode;
}

export function ThreadCard({
	threadId,
	channelName,
	author,
	authorFallback,
	title,
	preview,
	lastActivityTs,
	replyCount,
	participantCount,
	reactionCount,
	fileCount = 0,
	score,
	rank,
	className,
	action,
}: ThreadCardProps) {
	const previewIsDuplicate = !shouldRenderThreadPreview(title, preview);
	const metrics = buildThreadCardMetrics(
		replyCount,
		reactionCount,
		participantCount,
		fileCount,
	);

	return (
		<article
			className={cn(
				"group rounded-[1.45rem] border border-white/8 bg-[#090b0d] px-4 py-4 shadow-[0_18px_56px_rgba(0,0,0,0.28)] transition-colors hover:border-[#22c55e]/28 hover:bg-[#0b0d10]",
				className,
			)}
		>
			<div className="flex items-start gap-4">
				{typeof rank === "number" ? (
					<div className="hidden min-w-8 justify-center pt-1 text-[2rem] font-semibold leading-none text-[#22c55e] xl:flex">
						{rank}
					</div>
				) : null}

				<IdentityAvatar
					author={author}
					fallback={authorFallback || channelName}
					size="md"
				/>

				<Link
					to="/threads/$threadId"
					params={{ threadId }}
					className="min-w-0 flex-1"
				>
					<div className="flex items-start justify-between gap-4">
						<div className="min-w-0">
							<div className="flex flex-wrap items-center gap-2">
								<p className="truncate text-[1.08rem] font-medium text-white">
									{displayAuthorName(author, authorFallback || channelName)}
								</p>
								<ChannelBadge name={channelName} />
							</div>
							<div className="mt-3 text-[1.06rem] leading-snug text-[#eef0f2]">
								{renderRichNode(title)}
							</div>
							{previewIsDuplicate ? null : (
								<div className="mt-2 line-clamp-2 text-[0.98rem] leading-relaxed text-[#a6a9b1]">
									{renderRichNode(preview)}
								</div>
							)}
						</div>

						<div className="hidden shrink-0 text-right lg:block">
							<time className="block text-[0.95rem] text-[#83868e]">
								{formatSlackTimestamp(lastActivityTs)}
							</time>
							{typeof score === "number" ? (
								<div className="mt-4 rounded-full border border-[#1fc86f]/20 bg-[#112017] px-3 py-1 text-sm font-medium text-[#29d779]">
									{score}
								</div>
							) : null}
						</div>
					</div>

					<div className="mt-4 flex flex-wrap items-center gap-4 text-[0.95rem] text-[#8f9299]">
						{metrics.map((metric) => (
							<Metric
								key={metric.label}
								icon={metric.icon}
								label={metric.label}
							/>
						))}
					</div>

					<div className="mt-3 flex items-center justify-between gap-3 lg:hidden">
						<time className="text-[0.9rem] text-[#8b8b93]">
							{formatSlackTimestamp(lastActivityTs)}
						</time>
						{typeof score === "number" ? (
							<span className="rounded-full border border-[#1fc86f]/20 bg-[#112017] px-3 py-1 text-sm font-medium text-[#29d779]">
								{score}
							</span>
						) : null}
					</div>
				</Link>

				{action ? <div className="shrink-0 pt-1">{action}</div> : null}
			</div>
		</article>
	);
}

function Metric({ icon, label }: { icon: ReactNode; label: string }) {
	return (
		<span className="inline-flex items-center gap-2">
			{icon}
			{label}
		</span>
	);
}

function renderRichNode(content: ReactNode) {
	return typeof content === "string" ? renderSlackText(content) : content;
}

export function buildThreadCardMetrics(
	replyCount: number,
	reactionCount: number,
	participantCount: number,
	fileCount: number,
) {
	return [
		{
			icon: <MessageSquare className="size-4" />,
			label: formatStatLabel(replyCount, "reply", "replies"),
		},
		reactionCount > 0
			? {
					icon: <Heart className="size-4" />,
					label: formatStatLabel(reactionCount, "reaction", "reactions"),
				}
			: null,
		participantCount > 0
			? {
					icon: <Users className="size-4" />,
					label: formatStatLabel(participantCount, "person", "people"),
				}
			: null,
		fileCount > 0
			? {
					icon: <Paperclip className="size-4" />,
					label: formatStatLabel(fileCount, "file", "files"),
				}
			: null,
	].filter((metric) => metric !== null);
}

export function shouldRenderThreadPreview(
	title: ReactNode,
	preview: ReactNode,
) {
	const normalizedTitle = normalizeRichText(title);
	const normalizedPreview = normalizeRichText(preview);
	return normalizedPreview.length > 0 && normalizedTitle !== normalizedPreview;
}

function normalizeRichText(content: ReactNode): string {
	return flattenText(content).replace(/\s+/g, " ").trim().toLowerCase();
}

function flattenText(content: ReactNode): string {
	if (
		content === null ||
		content === undefined ||
		typeof content === "boolean"
	) {
		return "";
	}

	if (typeof content === "string" || typeof content === "number") {
		return String(content);
	}

	if (Array.isArray(content)) {
		return content.map((item) => flattenText(item)).join("");
	}

	if (isValidElement<{ children?: ReactNode }>(content)) {
		return flattenText(content.props.children);
	}

	return Children.toArray(content)
		.map((item) => flattenText(item))
		.join("");
}
