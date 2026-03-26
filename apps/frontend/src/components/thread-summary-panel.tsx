import { ChannelBadge } from "@/components/channel-badge";
import { IdentityAvatar } from "@/components/identity-avatar";
import { SectionCard } from "@/components/section-card";
import { ThreadMetrics } from "@/components/thread-metrics";
import { ThreadSummaryMarkdown } from "@/components/thread-summary-markdown";
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
			title={null}
			titleClassName="hidden"
			overlayActions
			actions={action}
			className="overflow-hidden"
		>
			<div className="space-y-3">
				<div className="flex flex-wrap items-center gap-2.5 text-[0.76rem] text-(--color-text-secondary)">
					<div className="inline-flex items-center gap-2">
						<IdentityAvatar
							author={rootMessage?.author}
							fallback={rootMessage?.user_id ?? thread.channel_name}
							size="sm"
						/>
						<span className="text-(--color-text-bright)">
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
						<time className="text-copy-soft tabular-nums">
							Active {formatSlackTimestamp(lastActivityTs)}
						</time>
					) : null}
				</div>

				{summary.body ? (
					summary.bodyFormat === "markdown" ? (
						<ThreadSummaryMarkdown content={summary.body} />
					) : (
						<p className="max-w-2xl text-[0.88rem] leading-6 text-(--color-text-secondary)">
							{renderSlackText(summary.body)}
						</p>
					)
				) : null}

				{summary.secondary ? (
					<div className="space-y-1.5">
						<p className="text-[0.68rem] font-semibold uppercase tracking-[0.12em] text-(--color-text-faint)">
							Why It Mattered
						</p>
						<p className="text-[0.84rem] leading-6 text-(--color-text-secondary)">
							{renderSlackText(summary.secondary)}
						</p>
					</div>
				) : null}

				{status || topicTags.length ? (
					<div className="flex flex-wrap items-center gap-1.5">
						{thread.summary.is_stale ? (
							<SummaryChip
								label="May be outdated"
								className="summary-chip-stale"
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
					className="pt-1 text-[0.75rem]"
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
				"summary-chip",
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
			return "summary-chip-answered";
		case "announcement":
			return "summary-chip-announcement";
		case "resource":
			return "summary-chip-resource";
		case "unresolved":
			return "summary-chip-unresolved";
		case "debate":
			return "summary-chip-debate";
		default:
			return "summary-chip-default";
	}
}

function normalizeLabel(value: string | null | undefined) {
	const trimmed = value?.trim();
	return trimmed ? trimmed.replace(/[_-]+/g, " ") : null;
}
