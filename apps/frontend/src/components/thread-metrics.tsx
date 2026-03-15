import { cn } from "@/lib/utils";
import { Heart, MessageSquare, Paperclip, Users } from "lucide-react";

interface ThreadMetricsProps {
	replyCount?: number | null;
	reactionCount?: number | null;
	participantCount?: number | null;
	fileCount?: number | null;
	className?: string;
	iconClassName?: string;
}

interface ThreadMetricItem {
	key: string;
	icon: typeof MessageSquare;
	value: number;
}

export function ThreadMetrics({
	replyCount,
	reactionCount,
	participantCount,
	fileCount = 0,
	className,
	iconClassName,
}: ThreadMetricsProps) {
	const metrics = buildThreadMetrics(
		normalizeMetricCount(replyCount),
		normalizeMetricCount(reactionCount),
		normalizeMetricCount(participantCount),
		normalizeMetricCount(fileCount),
	);

	return (
		<div
			className={cn(
				"text-copy-soft flex flex-wrap items-center gap-2.5 text-[0.72rem]",
				className,
			)}
		>
			{metrics.map((metric) => {
				const Icon = metric.icon;
				return (
					<span key={metric.key} className="inline-flex items-center gap-1">
						<Icon className={cn("size-3.5", iconClassName)} />
						{metric.value}
					</span>
				);
			})}
		</div>
	);
}

export function buildThreadMetrics(
	replyCount: number,
	reactionCount: number,
	participantCount: number,
	fileCount: number,
): ThreadMetricItem[] {
	return [
		{
			key: "replies",
			icon: MessageSquare,
			value: replyCount,
		},
		reactionCount > 0
			? {
					key: "reactions",
					icon: Heart,
					value: reactionCount,
				}
			: null,
		participantCount > 0
			? {
					key: "participants",
					icon: Users,
					value: participantCount,
				}
			: null,
		fileCount > 0
			? {
					key: "files",
					icon: Paperclip,
					value: fileCount,
				}
			: null,
	].filter((metric) => metric !== null);
}

function normalizeMetricCount(value: number | null | undefined): number {
	return Number.isFinite(value) ? Math.max(0, value ?? 0) : 0;
}
