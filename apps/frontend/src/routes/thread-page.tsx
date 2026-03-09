import { EmptyState } from "@/components/empty-state";
import { SectionCard } from "@/components/section-card";
import { Button } from "@/components/ui/button";
import { deleteSavedThread, saveThread } from "@/lib/api";
import { formatSlackTimestamp, formatStatLabel } from "@/lib/format";
import { savedQueries, threadQueries } from "@/lib/queries";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useParams } from "@tanstack/react-router";
import {
	Bookmark,
	BookmarkCheck,
	ChevronDown,
	Link2,
	MessageSquare,
	Paperclip,
	Sparkles,
} from "lucide-react";
import type { ReactNode } from "react";
import { useState } from "react";

const INITIAL_MESSAGE_COUNT = 4;

export function ThreadPage() {
	const { threadId } = useParams({ from: "/app/threads/$threadId" });
	const queryClient = useQueryClient();
	const [isExpanded, setIsExpanded] = useState(false);

	const threadQuery = useQuery(threadQueries.detail(threadId));
	const savedQuery = useQuery(savedQueries.list());

	const savedItem = savedQuery.data?.items.find(
		(item) => item.thread_id === threadId,
	);

	const saveMutation = useMutation({
		mutationFn: () =>
			savedItem ? deleteSavedThread(threadId) : saveThread(threadId),
		onSuccess: async () => {
			await queryClient.invalidateQueries({ queryKey: ["saved"] });
		},
	});

	if (threadQuery.isPending) {
		return (
			<div className="space-y-4">
				{["skel-a", "skel-b", "skel-c"].map((id) => (
					<div
						key={id}
						className="h-36 animate-pulse rounded-(--radius-section) border border-(--color-border-subtle) bg-(--color-bg-surface)/40"
					/>
				))}
			</div>
		);
	}

	if (threadQuery.isError || !threadQuery.data) {
		return (
			<EmptyState
				title="Thread unavailable"
				description="The requested thread could not be loaded from the local API."
			/>
		);
	}

	const { messages, reply_count: replyCount } = threadQuery.data;
	const rootMessage = messages[0];
	const visibleMessages =
		isExpanded || messages.length <= INITIAL_MESSAGE_COUNT
			? messages
			: messages.slice(0, INITIAL_MESSAGE_COUNT);
	const highlightedMessages = messages.filter(
		(message, index) =>
			index === 0 || message.reactions.length > 0 || message.files.length > 0,
	);
	const links = extractLinks(
		messages.map((message) => message.text).join("\n"),
	);
	const allFiles = messages.flatMap((message) => message.files);
	const participantCount = new Set(
		messages
			.map((message) => message.user_id)
			.filter((userId): userId is string => Boolean(userId)),
	).size;
	const reactionCount = messages.reduce(
		(total, message) => total + message.reactions.length,
		0,
	);

	return (
		<div className="space-y-5">
			<SectionCard
				eyebrow="Thread detail"
				title={rootMessage?.text || "Untitled thread"}
				description="A focused view of the conversation, with the highest-signal messages pulled to the top."
				actions={
					<Button
						type="button"
						variant={savedItem ? "default" : "secondary"}
						onClick={() => saveMutation.mutate()}
						disabled={saveMutation.isPending}
					>
						{savedItem ? (
							<BookmarkCheck className="size-4" />
						) : (
							<Bookmark className="size-4" />
						)}
						{saveMutation.isPending
							? savedItem
								? "Removing"
								: "Saving"
							: savedItem
								? "Saved"
								: "Save thread"}
					</Button>
				}
			>
				<div className="flex flex-wrap gap-2 text-xs text-(--color-text-secondary)">
					<MetaChip label={formatStatLabel(replyCount, "reply", "replies")} />
					<MetaChip
						label={formatStatLabel(
							participantCount,
							"participant",
							"participants",
						)}
					/>
					<MetaChip
						label={formatStatLabel(reactionCount, "reaction", "reactions")}
					/>
					{allFiles.length > 0 && (
						<MetaChip
							label={formatStatLabel(allFiles.length, "file", "files")}
						/>
					)}
				</div>
			</SectionCard>

			<div className="grid gap-5 xl:grid-cols-[1.1fr_0.9fr]">
				<div className="space-y-5">
					<SectionCard
						eyebrow="Key messages"
						title="Important moments"
						description="Root messages, replies with reactions, and posts that carried files."
					>
						<div className="space-y-3">
							{highlightedMessages.map((message) => (
								<MessageCard key={message.ts} message={message} />
							))}
						</div>
					</SectionCard>

					<SectionCard
						eyebrow="Full transcript"
						title="Conversation timeline"
						description="The full thread stays collapsed for long conversations until you ask for the rest."
					>
						<div className="space-y-3">
							{visibleMessages.map((message) => (
								<MessageCard key={message.ts} message={message} />
							))}
						</div>
						{messages.length > INITIAL_MESSAGE_COUNT && (
							<div className="mt-4">
								<Button
									type="button"
									variant="secondary"
									onClick={() => setIsExpanded((current) => !current)}
								>
									<ChevronDown className="size-4" />
									{isExpanded
										? "Collapse transcript"
										: `Show all ${messages.length} messages`}
								</Button>
							</div>
						)}
					</SectionCard>
				</div>

				<div className="space-y-5">
					<SectionCard
						eyebrow="Links"
						title="Linked references"
						description="URLs mentioned in the thread are extracted here."
					>
						{links.length ? (
							<ul className="space-y-2 text-sm">
								{links.map((link) => (
									<li
										key={link}
										className="rounded-(--radius-card) border border-(--color-border-subtle) bg-(--color-bg-base)/60 p-3"
									>
										<a
											href={link}
											target="_blank"
											rel="noreferrer"
											className="flex items-center gap-2 break-all text-(--color-accent-soft) hover:underline"
										>
											<Link2 className="size-4 shrink-0" />
											{link}
										</a>
									</li>
								))}
							</ul>
						) : (
							<EmptyState
								title="No links found"
								description="This thread does not contain extractable URLs."
								icon={<Link2 className="size-5" />}
							/>
						)}
					</SectionCard>

					<SectionCard
						eyebrow="Files"
						title="Attached files"
						description="Files shared in the thread, grouped for easy access."
					>
						{allFiles.length ? (
							<ul className="space-y-2">
								{allFiles.map((file) => (
									<li
										key={file.id}
										className="rounded-(--radius-card) border border-(--color-border-subtle) bg-(--color-bg-base)/60 p-4"
									>
										<p className="text-sm font-medium text-(--color-text-primary)">
											{file.name}
										</p>
										<p className="mt-1 text-xs text-(--color-text-muted)">
											{file.mimetype || "unknown type"}
										</p>
										{file.permalink && (
											<a
												className="mt-2 inline-flex items-center gap-2 text-sm text-(--color-accent-soft) hover:underline"
												href={file.permalink}
												target="_blank"
												rel="noreferrer"
											>
												<Paperclip className="size-4" />
												Open file
											</a>
										)}
									</li>
								))}
							</ul>
						) : (
							<EmptyState
								title="No files attached"
								description="When Slack file shares are present in the thread, they will show up here."
								icon={<Paperclip className="size-5" />}
							/>
						)}
					</SectionCard>
				</div>
			</div>
		</div>
	);
}

interface MessageProps {
	message: {
		ts: string;
		thread_ts: string | null;
		user_id: string | null;
		text: string;
		reactions: { user_id: string; name: string }[];
		files: { id: string; name: string }[];
	};
}

function MessageCard({ message }: MessageProps) {
	return (
		<article className="rounded-(--radius-card) border border-(--color-border-subtle) bg-(--color-bg-base)/60 p-4">
			<div className="flex flex-wrap items-center gap-2 text-xs text-(--color-text-muted)">
				<span>{message.user_id || "Unknown member"}</span>
				<span className="text-(--color-border-default)">&middot;</span>
				<time className="tabular-nums">{formatSlackTimestamp(message.ts)}</time>
			</div>
			<p className="mt-2.5 whitespace-pre-wrap text-sm leading-relaxed text-(--color-text-primary)">
				{message.text}
			</p>
			<div className="mt-3 flex flex-wrap gap-1.5 text-xs text-(--color-text-secondary)">
				{message.reactions.length > 0 && (
					<MetaChip
						icon={<Sparkles className="size-3.5" />}
						label={formatStatLabel(
							message.reactions.length,
							"reaction",
							"reactions",
						)}
					/>
				)}
				{message.files.length > 0 && (
					<MetaChip
						icon={<Paperclip className="size-3.5" />}
						label={formatStatLabel(message.files.length, "file", "files")}
					/>
				)}
				{message.thread_ts && (
					<MetaChip
						icon={<MessageSquare className="size-3.5" />}
						label="Reply"
					/>
				)}
			</div>
		</article>
	);
}

function MetaChip({
	label,
	icon,
}: {
	label: string;
	icon?: ReactNode;
}) {
	return (
		<span className="inline-flex items-center gap-1.5 rounded-(--radius-pill) border border-(--color-border-subtle) bg-(--color-bg-surface)/50 px-2.5 py-0.5">
			{icon}
			{label}
		</span>
	);
}

function extractLinks(text: string) {
	return Array.from(text.matchAll(/https?:\/\/[^\s)]+/g), (match) => match[0]);
}
