import { formatSlackTimestamp, formatStatLabel } from "@/lib/format";
import { cn } from "@/lib/utils";
import { Link } from "@tanstack/react-router";
import { MessageSquare, Paperclip, Sparkles, Users } from "lucide-react";
import type { ReactNode } from "react";

interface ThreadCardProps {
	threadId: string;
	channelName?: string | null;
	title: ReactNode;
	preview: ReactNode;
	lastActivityTs: string;
	replyCount: number;
	participantCount: number;
	reactionCount: number;
	fileCount?: number;
	className?: string;
	action?: ReactNode;
}

export function ThreadCard({
	threadId,
	channelName,
	title,
	preview,
	lastActivityTs,
	replyCount,
	participantCount,
	reactionCount,
	fileCount = 0,
	className,
	action,
}: ThreadCardProps) {
	return (
		<article
			className={cn(
				"group rounded-(--radius-card) border border-(--color-border-subtle) bg-(--color-bg-base)/60 p-4 transition-colors hover:border-(--color-border-accent) hover:bg-(--color-bg-surface)/80",
				className,
			)}
		>
			<div className="flex items-start gap-3">
				<Link
					to="/threads/$threadId"
					params={{ threadId }}
					className="min-w-0 flex-1"
				>
					<div className="flex items-start justify-between gap-3">
						<div className="min-w-0">
							<p className="text-[0.6rem] font-medium uppercase tracking-[0.24em] text-(--color-text-muted)">
								# {channelName || "public-channel"}
							</p>
							<h3 className="mt-1.5 line-clamp-2 text-[0.94rem] font-semibold leading-snug text-(--color-text-primary)">
								{title}
							</h3>
						</div>
						<time className="shrink-0 text-xs tabular-nums text-(--color-text-muted)">
							{formatSlackTimestamp(lastActivityTs)}
						</time>
					</div>
					<p className="mt-2.5 line-clamp-2 text-sm leading-relaxed text-(--color-text-secondary)">
						{preview}
					</p>
					<div className="mt-3 flex flex-wrap gap-1.5 text-xs text-(--color-text-secondary)">
						{replyCount > 0 && (
							<StatChip
								icon={<MessageSquare className="size-3.5" />}
								label={formatStatLabel(replyCount, "reply", "replies")}
							/>
						)}
						{participantCount > 0 && (
							<StatChip
								icon={<Users className="size-3.5" />}
								label={formatStatLabel(participantCount, "person", "people")}
							/>
						)}
						{reactionCount > 0 && (
							<StatChip
								icon={<Sparkles className="size-3.5" />}
								label={formatStatLabel(reactionCount, "reaction", "reactions")}
							/>
						)}
						{fileCount > 0 && (
							<StatChip
								icon={<Paperclip className="size-3.5" />}
								label={formatStatLabel(fileCount, "file", "files")}
							/>
						)}
					</div>
				</Link>
				{action ? <div className="shrink-0">{action}</div> : null}
			</div>
		</article>
	);
}

function StatChip({ icon, label }: { icon: ReactNode; label: string }) {
	return (
		<span className="inline-flex items-center gap-1.5 rounded-(--radius-pill) border border-(--color-border-subtle) bg-(--color-bg-surface)/50 px-2.5 py-0.5">
			{icon}
			{label}
		</span>
	);
}
