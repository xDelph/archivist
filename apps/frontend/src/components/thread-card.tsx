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
}: ThreadCardProps) {
	return (
		<Link
			to="/threads/$threadId"
			params={{ threadId }}
			className={cn(
				"block rounded-[1.5rem] border border-white/10 bg-slate-950/35 p-4 transition hover:border-[var(--accent-soft)]/40 hover:bg-slate-950/50",
				className,
			)}
		>
			<div className="flex items-start justify-between gap-4">
				<div>
					<p className="text-[0.65rem] uppercase tracking-[0.24em] text-slate-400">
						{channelName || "Public channel"}
					</p>
					<h3 className="mt-2 text-base font-semibold text-white">{title}</h3>
				</div>
				<p className="text-xs text-slate-400">
					{formatSlackTimestamp(lastActivityTs)}
				</p>
			</div>
			<p className="mt-3 text-sm leading-6 text-slate-300">{preview}</p>
			<div className="mt-4 flex flex-wrap gap-2 text-xs text-slate-300">
				<StatChip
					icon={<MessageSquare className="size-3.5" />}
					label={formatStatLabel(replyCount, "reply", "replies")}
				/>
				<StatChip
					icon={<Users className="size-3.5" />}
					label={formatStatLabel(participantCount, "person", "people")}
				/>
				<StatChip
					icon={<Sparkles className="size-3.5" />}
					label={formatStatLabel(reactionCount, "reaction", "reactions")}
				/>
				{fileCount ? (
					<StatChip
						icon={<Paperclip className="size-3.5" />}
						label={formatStatLabel(fileCount, "file", "files")}
					/>
				) : null}
			</div>
		</Link>
	);
}

function StatChip({ icon, label }: { icon: ReactNode; label: string }) {
	return (
		<span className="inline-flex items-center gap-1.5 rounded-full border border-white/10 bg-white/[0.04] px-3 py-1">
			{icon}
			{label}
		</span>
	);
}
