import { EmptyState } from "@/components/empty-state";
import { SectionCard } from "@/components/section-card";
import { Button } from "@/components/ui/button";
import {
	deleteSavedThread,
	fetchSavedItems,
	fetchThreadDetail,
	saveThread,
} from "@/lib/api";
import { formatSlackTimestamp, formatStatLabel } from "@/lib/format";
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
import { useState } from "react";

const INITIAL_MESSAGE_COUNT = 4;

export function ThreadPage() {
	const { threadId } = useParams({ from: "/app/threads/$threadId" });
	const queryClient = useQueryClient();
	const [isExpanded, setIsExpanded] = useState(false);
	const threadQuery = useQuery({
		queryKey: ["thread", threadId],
		queryFn: () => fetchThreadDetail(threadId),
	});
	const savedQuery = useQuery({
		queryKey: ["saved"],
		queryFn: fetchSavedItems,
	});
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
				{["thread-skeleton-1", "thread-skeleton-2", "thread-skeleton-3"].map(
					(key) => (
						<div
							key={key}
							className="h-40 animate-pulse rounded-[1.75rem] border border-white/8 bg-white/[0.04]"
						/>
					),
				)}
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
							<BookmarkCheck className="mr-2 size-4" />
						) : (
							<Bookmark className="mr-2 size-4" />
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
				<div className="flex flex-wrap gap-2 text-xs text-slate-300">
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
					{allFiles.length ? (
						<MetaChip
							label={formatStatLabel(allFiles.length, "file", "files")}
						/>
					) : null}
				</div>
			</SectionCard>

			<div className="grid gap-5 xl:grid-cols-[1.1fr_0.9fr]">
				<div className="space-y-5">
					<SectionCard
						eyebrow="Key messages"
						title="Important moments"
						description="Root messages, replies with reactions, and posts that carried files stay visible even before you expand the full transcript."
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
						{messages.length > INITIAL_MESSAGE_COUNT ? (
							<div className="mt-4">
								<Button
									type="button"
									variant="secondary"
									onClick={() => setIsExpanded((current) => !current)}
								>
									<ChevronDown className="mr-2 size-4" />
									{isExpanded
										? "Collapse transcript"
										: `Show all ${messages.length} messages`}
								</Button>
							</div>
						) : null}
					</SectionCard>
				</div>

				<div className="space-y-5">
					<SectionCard
						eyebrow="Links"
						title="Linked references"
						description="URLs mentioned anywhere in the thread are extracted here for quicker review."
					>
						{links.length ? (
							<ul className="space-y-3 text-sm text-slate-200">
								{links.map((link) => (
									<li
										key={link}
										className="rounded-[1.25rem] border border-white/10 bg-slate-950/35 p-3"
									>
										<a
											href={link}
											target="_blank"
											rel="noreferrer"
											className="flex items-center gap-2 break-all text-[var(--accent-soft)]"
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
								description="This thread currently does not contain extractable URLs."
							/>
						)}
					</SectionCard>

					<SectionCard
						eyebrow="Files"
						title="Attached files"
						description="Files carried in the thread stay grouped here, even when they were shared deep in the reply chain."
					>
						{allFiles.length ? (
							<ul className="space-y-3">
								{allFiles.map((file) => (
									<li
										key={file.id}
										className="rounded-[1.25rem] border border-white/10 bg-slate-950/35 p-4"
									>
										<p className="text-sm font-medium text-white">
											{file.name}
										</p>
										<p className="mt-1 text-xs text-slate-400">
											{file.mimetype || "unknown type"}
										</p>
										{file.permalink ? (
											<a
												className="mt-3 inline-flex items-center gap-2 text-sm text-[var(--accent-soft)]"
												href={file.permalink}
												target="_blank"
												rel="noreferrer"
											>
												<Paperclip className="size-4" />
												Open file
											</a>
										) : null}
									</li>
								))}
							</ul>
						) : (
							<EmptyState
								title="No files attached"
								description="When Slack file shares are present in the thread, they will show up here."
							/>
						)}
					</SectionCard>
				</div>
			</div>
		</div>
	);
}

function MessageCard({
	message,
}: {
	message: Awaited<ReturnType<typeof fetchThreadDetail>>["messages"][number];
}) {
	return (
		<article className="rounded-[1.5rem] border border-white/10 bg-slate-950/35 p-4">
			<div className="flex flex-wrap items-center gap-2 text-xs text-slate-400">
				<span>{message.user_id || "Unknown member"}</span>
				<span>•</span>
				<span>{formatSlackTimestamp(message.ts)}</span>
			</div>
			<p className="mt-3 whitespace-pre-wrap text-sm leading-6 text-slate-200">
				{message.text}
			</p>
			<div className="mt-4 flex flex-wrap gap-2 text-xs text-slate-300">
				{message.reactions.length ? (
					<MetaChip
						icon={<Sparkles className="size-3.5" />}
						label={formatStatLabel(
							message.reactions.length,
							"reaction",
							"reactions",
						)}
					/>
				) : null}
				{message.files.length ? (
					<MetaChip
						icon={<Paperclip className="size-3.5" />}
						label={formatStatLabel(message.files.length, "file", "files")}
					/>
				) : null}
				{message.thread_ts ? (
					<MetaChip
						icon={<MessageSquare className="size-3.5" />}
						label="Reply"
					/>
				) : null}
			</div>
		</article>
	);
}

function MetaChip({
	label,
	icon,
}: {
	label: string;
	icon?: React.ReactNode;
}) {
	return (
		<span className="inline-flex items-center gap-1.5 rounded-full border border-white/10 bg-white/[0.05] px-3 py-1">
			{icon}
			{label}
		</span>
	);
}

function extractLinks(text: string) {
	return Array.from(text.matchAll(/https?:\/\/[^\s)]+/g), (match) => match[0]);
}
