import { ChannelBadge } from "@/components/channel-badge";
import { IdentityAvatar } from "@/components/identity-avatar";
import { SectionCard } from "@/components/section-card";
import { ThreadMetrics } from "@/components/thread-metrics";
import type { ThreadDetailResponse } from "@/lib/api";
import { formatSlackTimestamp } from "@/lib/format";
import { displayAuthorName, renderSlackText } from "@/lib/thread-display";
import { resolveThreadSummaryPresentation } from "@/lib/thread-summary";
import { cn } from "@/lib/utils";
import type { ReactNode } from "react";

interface ThreadSummaryPanelProps {
	thread: ThreadDetailResponse;
	action?: ReactNode;
}

export function ThreadSummaryPanel({
	thread,
	action,
}: ThreadSummaryPanelProps) {
	const rootMessage = thread.messages[0];
	const summary = resolveThreadSummaryPresentation(thread);
	const status = normalizeLabel(thread.summary.status);
	const topicTags = thread.summary.topic_tags
		.map(normalizeLabel)
		.filter((tag): tag is string => Boolean(tag));
	const lastActivityTs = thread.last_activity_ts ?? rootMessage?.ts ?? null;

	return (
		<SectionCard
			eyebrow={summary.source === "ai" ? "AI Summary" : "Thread Detail"}
			title={
				summary.primary ? (
					<span className="leading-tight text-[#f3f5f7]">
						{renderSlackText(summary.primary)}
					</span>
				) : (
					"Conversation overview"
				)
			}
			titleClassName="max-w-[calc(100%-3rem)] text-[1.08rem] font-medium sm:text-[1.28rem]"
			overlayActions
			actions={action}
			className="overflow-hidden"
		>
			<div className="space-y-3">
				<div className="flex flex-wrap items-center gap-2.5 text-[0.76rem] text-[#9aa0a8]">
					<div className="inline-flex items-center gap-2">
						<IdentityAvatar
							author={rootMessage?.author}
							fallback={rootMessage?.user_id ?? thread.channel_name}
							size="sm"
						/>
						<span className="text-[#eef0f2]">
							{displayAuthorName(
								rootMessage?.author,
								rootMessage?.user_id ?? thread.channel_name,
							)}
						</span>
					</div>
					{thread.channel_name ? (
						<ChannelBadge
							name={thread.channel_name}
							className="text-[0.7rem]"
						/>
					) : null}
					{lastActivityTs ? (
						<time className="tabular-nums text-[#7f8791]">
							Active {formatSlackTimestamp(lastActivityTs)}
						</time>
					) : null}
				</div>

				{summary.secondary ? (
					<p className="max-w-2xl text-[0.84rem] leading-6 text-[#aeb3ba]">
						{renderSlackText(summary.secondary)}
					</p>
				) : null}

				{status || topicTags.length ? (
					<div className="flex flex-wrap items-center gap-1.5">
						{thread.summary.is_stale ? (
							<SummaryChip
								label="May be outdated"
								className="border-[#6a5321] bg-[#211807] text-[#e9cf7a]"
							/>
						) : null}
						{status ? (
							<SummaryChip
								label={status}
								className={statusTone(thread.summary.status)}
							/>
						) : null}
						{topicTags.map((tag) => (
							<SummaryChip key={tag} label={tag} />
						))}
					</div>
				) : null}

				<ThreadMetrics
					replyCount={thread.reply_count}
					reactionCount={thread.reaction_count}
					participantCount={thread.participant_count}
					fileCount={thread.file_count}
					className="pt-1 text-[0.75rem] text-[#8d929a]"
				/>
			</div>
		</SectionCard>
	);
}

function SummaryChip({
	label,
	className,
}: {
	label: string;
	className?: string;
}) {
	return (
		<span
			className={cn(
				"inline-flex items-center rounded-full border px-2.5 py-1 text-[0.68rem] font-medium tracking-[0.02em]",
				"border-white/10 bg-white/[0.04] text-[#c9cfd6]",
				className,
			)}
		>
			{label}
		</span>
	);
}

function statusTone(status: string | null) {
	switch (normalizeLabel(status)) {
		case "answered":
			return "border-[#175d36] bg-[#0f2317] text-[#63d98e]";
		case "announcement":
			return "border-[#205b6d] bg-[#091b21] text-[#78daf2]";
		case "resource":
			return "border-[#5c5121] bg-[#1f1908] text-[#e8d47b]";
		case "unresolved":
			return "border-[#7a3f27] bg-[#26120a] text-[#f5a774]";
		case "debate":
			return "border-[#6a2550] bg-[#210914] text-[#f29ac9]";
		default:
			return "border-[#3a454c] bg-[#0f1418] text-[#bcc3cb]";
	}
}

function normalizeLabel(value: string | null | undefined) {
	const trimmed = value?.trim();
	return trimmed ? trimmed.replace(/[_-]+/g, " ") : null;
}
