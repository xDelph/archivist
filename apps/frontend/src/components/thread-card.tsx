import { ChannelBadge } from "@/components/channel-badge";
import { IdentityAvatar } from "@/components/identity-avatar";
import { ThreadMetrics } from "@/components/thread-metrics";
import { formatSlackTimestamp } from "@/lib/format";
import {
	type ThreadAuthor,
	displayAuthorName,
	renderSlackText,
} from "@/lib/thread-display";
import { cn } from "@/lib/utils";
import { Link } from "@tanstack/react-router";
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
	return (
		<article
			className={cn(
				"group rounded-[0.82rem] border border-white/8 bg-[#07090b] px-2.5 py-2.5 shadow-[0_8px_24px_rgba(0,0,0,0.16)] transition-colors hover:border-[#22c55e]/28 hover:bg-[#090b0d]",
				className,
			)}
		>
			<div className="flex items-start gap-2">
				{typeof rank === "number" ? (
					<div className="hidden min-w-5 justify-center pt-0.5 text-[1.15rem] font-semibold leading-none text-[#22c55e] xl:flex">
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
					<div className="flex items-start justify-between gap-2.5">
						<div className="min-w-0">
							<div className="flex flex-wrap items-center gap-1.5">
								<p className="truncate text-[0.82rem] font-medium text-white">
									{displayAuthorName(author, authorFallback || channelName)}
								</p>
								<ChannelBadge name={channelName} />
							</div>
							<div className="mt-1 break-words text-[0.84rem] leading-snug font-normal text-[#eef0f2]">
								{renderRichNode(title)}
							</div>
							{previewIsDuplicate ? null : (
								<div className="mt-0.5 line-clamp-2 break-words text-[0.74rem] leading-relaxed text-[#a6a9b1]">
									{renderRichNode(preview)}
								</div>
							)}
						</div>

						<div className="hidden shrink-0 text-right lg:block">
							<time className="block text-[0.7rem] text-[#83868e]">
								{formatSlackTimestamp(lastActivityTs)}
							</time>
							{typeof score === "number" ? (
								<div className="mt-2 rounded-[0.65rem] border border-[#1fc86f]/20 bg-[#112017] px-2 py-0.5 text-[0.7rem] font-medium text-[#29d779]">
									{score}
								</div>
							) : null}
						</div>
					</div>

					<ThreadMetrics
						className="mt-2.5"
						replyCount={replyCount}
						reactionCount={reactionCount}
						participantCount={participantCount}
						fileCount={fileCount}
					/>

					<div className="mt-1.5 flex items-center justify-between gap-3 lg:hidden">
						<time className="text-[0.68rem] text-[#8b8b93]">
							{formatSlackTimestamp(lastActivityTs)}
						</time>
						{typeof score === "number" ? (
							<span className="rounded-[0.65rem] border border-[#1fc86f]/20 bg-[#112017] px-2 py-0.5 text-[0.7rem] font-medium text-[#29d779]">
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

function renderRichNode(content: ReactNode) {
	return typeof content === "string" ? renderSlackText(content) : content;
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
