import { EmptyState } from "@/components/empty-state";
import { IdentityAvatar } from "@/components/identity-avatar";
import { SectionCard } from "@/components/section-card";
import {
	ThreadFileViewer,
	type ThreadViewerFile,
} from "@/components/thread-file-viewer";
import { ThreadMetrics } from "@/components/thread-metrics";
import { Button } from "@/components/ui/button";
import { deleteSavedThread, saveThread } from "@/lib/api";
import { formatSlackTimestamp } from "@/lib/format";
import { savedQueries, threadQueries } from "@/lib/queries";
import {
	displayAuthorName,
	extractLinks,
	groupReactions,
	renderSlackText,
} from "@/lib/thread-display";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useParams } from "@tanstack/react-router";
import {
	Bookmark,
	BookmarkCheck,
	ChevronDown,
	ExternalLink,
	Paperclip,
} from "lucide-react";
import type { ReactNode } from "react";
import { useState } from "react";

const INITIAL_MESSAGE_COUNT = 4;

export function ThreadPage() {
	const { threadId } = useParams({ from: "/app/threads/$threadId" });
	const queryClient = useQueryClient();
	const [isExpanded, setIsExpanded] = useState(false);
	const [selectedFileState, setSelectedFileState] = useState<{
		messageTs: string;
		index: number;
	} | null>(null);

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
						className="h-36 animate-pulse rounded-[1.75rem] border border-white/8 bg-white/[0.03]"
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
	const fileGroups = messages
		.filter((message) => message.files.length > 0)
		.map((message) => ({
			messageTs: message.ts,
			files: message.files,
		}));
	const allFiles = fileGroups.flatMap((group) =>
		group.files.map((file, index) => ({
			...file,
			messageTs: group.messageTs,
			messageFileIndex: index,
		})),
	);
	const participantCount = new Set(
		messages
			.map((message) => message.user_id)
			.filter((userId): userId is string => Boolean(userId)),
	).size;
	const reactionCount = messages.reduce(
		(total, message) => total + message.reactions.length,
		0,
	);
	const selectedFileGroup = selectedFileState
		? fileGroups.find(
				(group) => group.messageTs === selectedFileState.messageTs,
			)
		: null;

	return (
		<div className="space-y-4">
			<SectionCard
				eyebrow="Thread detail"
				title={renderSlackText(rootMessage?.text || "Untitled thread")}
				titleClassName="font-normal"
				overlayActions
				actions={
					<Button
						type="button"
						variant={savedItem ? "default" : "secondary"}
						size="sm"
						className={
							savedItem
								? "size-9 rounded-full px-0 text-black hover:bg-[#2ae38a] sm:h-8 sm:w-auto sm:rounded-lg sm:px-3 sm:text-[0.74rem] bg-[#18cc77]"
								: "size-9 rounded-full border-white/10 bg-white/[0.03] px-0 text-white hover:border-[#1fc86f]/30 hover:bg-white/[0.06] sm:h-8 sm:w-auto sm:rounded-lg sm:px-3 sm:text-[0.74rem]"
						}
						onClick={() => saveMutation.mutate()}
						disabled={saveMutation.isPending}
					>
						{savedItem ? (
							<BookmarkCheck className="size-4 sm:size-3.5" />
						) : (
							<Bookmark className="size-4 sm:size-3.5" />
						)}
						<span className="hidden sm:inline">
							{saveMutation.isPending
								? savedItem
									? "Removing"
									: "Saving"
								: savedItem
									? "Saved"
									: "Save thread"}
						</span>
					</Button>
				}
			>
				<ThreadMetrics
					replyCount={replyCount}
					reactionCount={reactionCount}
					participantCount={participantCount}
					fileCount={allFiles.length}
				/>
			</SectionCard>

			<div className="grid gap-4 xl:grid-cols-[1.1fr_0.9fr]">
				<div className="space-y-4">
					<SectionCard eyebrow="Highlights" title="Important moments">
						<div className="space-y-2.5">
							{highlightedMessages.map((message) => (
								<MessageCard
									key={message.ts}
									message={message}
									onOpenFile={(fileIndex) =>
										setSelectedFileState({
											messageTs: message.ts,
											index: fileIndex,
										})
									}
								/>
							))}
						</div>
					</SectionCard>

					<SectionCard eyebrow="Full transcript" title="Conversation timeline">
						<div className="space-y-2.5">
							{visibleMessages.map((message) => (
								<MessageCard
									key={message.ts}
									message={message}
									onOpenFile={(fileIndex) =>
										setSelectedFileState({
											messageTs: message.ts,
											index: fileIndex,
										})
									}
								/>
							))}
						</div>
						{messages.length > INITIAL_MESSAGE_COUNT ? (
							<div className="mt-4">
								<Button
									type="button"
									variant="secondary"
									className="border-white/10 bg-white/[0.03] px-3 py-2 text-[0.78rem] text-white hover:border-[#1fc86f]/30 hover:bg-white/[0.06]"
									onClick={() => setIsExpanded((current) => !current)}
								>
									<ChevronDown className="size-3.5" />
									{isExpanded
										? "Collapse transcript"
										: `Show all ${messages.length} messages`}
								</Button>
							</div>
						) : null}
					</SectionCard>
				</div>

				<div className="space-y-4">
					<SectionCard eyebrow="Links" title="Linked references">
						{links.length ? (
							<ul className="space-y-2 text-[0.82rem]">
								{links.map((link) => (
									<li
										key={link.href}
										className="rounded-[0.9rem] border border-white/8 bg-[#0a0d0f] p-3"
									>
										<a
											href={link.href}
											target="_blank"
											rel="noreferrer"
											className="flex items-center gap-2 break-all text-[#5ea7ff] underline decoration-[#2d5cc2] underline-offset-3 hover:text-[#89bbff]"
										>
											<ExternalLink className="size-4 shrink-0" />
											{link.label || link.href}
										</a>
									</li>
								))}
							</ul>
						) : (
							<EmptyState
								title="No links found"
								description="This thread does not contain extractable URLs."
								icon={<ExternalLink className="size-5" />}
							/>
						)}
					</SectionCard>

					<SectionCard eyebrow="Files" title="Attached files">
						{allFiles.length ? (
							<ul className="space-y-2">
								{allFiles.map((file) => (
									<li
										key={`${file.messageTs}-${file.id}-${file.name}`}
										className="rounded-[0.9rem] border border-white/8 bg-[#0a0d0f] p-3"
									>
										<button
											type="button"
											className="w-full text-left"
											onClick={() =>
												setSelectedFileState({
													messageTs: file.messageTs,
													index: file.messageFileIndex,
												})
											}
										>
											<p className="text-[0.82rem] font-medium text-white">
												{file.name}
											</p>
											<p className="mt-1 text-[0.72rem] text-[#8f949b]">
												{file.mimetype || "unknown type"}
											</p>
											{file.permalink ? (
												<span className="mt-2 inline-flex items-center gap-1.5 text-[0.8rem] text-[#5ea7ff] underline decoration-[#2d5cc2] underline-offset-3 hover:text-[#89bbff]">
													<Paperclip className="size-4" />
													Open file
												</span>
											) : null}
										</button>
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

			{selectedFileGroup && selectedFileState ? (
				<ThreadFileViewer
					files={selectedFileGroup.files}
					currentIndex={Math.min(
						selectedFileState.index,
						selectedFileGroup.files.length - 1,
					)}
					messageTs={selectedFileGroup.messageTs}
					onClose={() => setSelectedFileState(null)}
					onChangeIndex={(index) =>
						setSelectedFileState({
							messageTs: selectedFileGroup.messageTs,
							index,
						})
					}
				/>
			) : null}
		</div>
	);
}

interface MessageProps {
	message: {
		ts: string;
		thread_ts: string | null;
		user_id: string | null;
		author: {
			slack_user_id: string;
			display_name: string | null;
			avatar_url: string | null;
		} | null;
		text: string;
		reactions: { user_id: string; name: string }[];
		files: ThreadViewerFile[];
	};
	onOpenFile: (fileIndex: number) => void;
}

function MessageCard({ message, onOpenFile }: MessageProps) {
	const reactions = groupReactions(message.reactions);

	return (
		<article className="rounded-[0.9rem] border border-white/8 bg-[#0a0d0f] p-3">
			<div className="flex items-start gap-2.5">
				<IdentityAvatar
					author={message.author}
					fallback={message.user_id}
					size="sm"
				/>
				<div className="min-w-0 flex-1">
					<div className="flex flex-wrap items-center gap-1.5 text-[0.72rem] text-[#868b93]">
						<span className="text-[0.82rem] font-medium text-white">
							{displayAuthorName(message.author, message.user_id)}
						</span>
						<span className="text-white/15">&middot;</span>
						<time className="tabular-nums">
							{formatSlackTimestamp(message.ts)}
						</time>
						{message.thread_ts ? (
							<>
								<span className="text-white/15">&middot;</span>
								<span>Reply</span>
							</>
						) : null}
					</div>
					<div className="mt-1.5 whitespace-pre-wrap text-[0.82rem] leading-6 font-normal text-[#eef0f2]">
						{renderSlackText(message.text)}
					</div>
				</div>
			</div>

			<div className="mt-3 flex flex-wrap gap-1.5">
				{reactions.map((reaction) => (
					<MetaChip
						key={reaction.name}
						label={`${reaction.emoji ?? `:${reaction.name}:`} ${reaction.count}`}
					/>
				))}
				{message.files.map((file, index) => (
					<button
						key={file.id}
						type="button"
						className="inline-flex items-center gap-1 rounded-full border border-white/10 bg-white/[0.03] px-2 py-0.5 text-[0.72rem] text-[#c4c8cf] hover:border-[#1fc86f]/22 hover:text-white"
						onClick={() => onOpenFile(index)}
					>
						<Paperclip className="size-3.5" />
						{file.name}
					</button>
				))}
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
		<span className="inline-flex items-center gap-1 rounded-full border border-white/10 bg-white/[0.03] px-2 py-0.5 text-[0.72rem] text-[#c3c8ce]">
			{icon}
			{label}
		</span>
	);
}
