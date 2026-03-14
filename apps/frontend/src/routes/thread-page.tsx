import { EmptyState } from "@/components/empty-state";
import { SectionCard } from "@/components/section-card";
import { ThreadFileViewer } from "@/components/thread-file-viewer";
import { ThreadFilesPanel } from "@/components/thread-files-panel";
import { ThreadLinksPanel } from "@/components/thread-links-panel";
import { ThreadMessageCard } from "@/components/thread-message-card";
import { ThreadSummaryPanel } from "@/components/thread-summary-panel";
import { Button } from "@/components/ui/button";
import { deleteSavedThread, saveThread } from "@/lib/api";
import { savedQueries, threadQueries } from "@/lib/queries";
import { extractLinks } from "@/lib/thread-display";
import { cn } from "@/lib/utils";
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
	const [selectedFileState, setSelectedFileState] = useState<{
		messageTs: string;
		index: number;
	} | null>(null);
	const [activeTab, setActiveTab] = useState<
		"highlights" | "transcript" | "links" | "files"
	>("highlights");

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
			<div className="mx-auto w-full max-w-3xl space-y-4 pb-8">
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

	const thread = threadQuery.data;
	const { messages } = thread;
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
	const selectedFileGroup = selectedFileState
		? fileGroups.find(
				(group) => group.messageTs === selectedFileState.messageTs,
			)
		: null;

	return (
		<div className="space-y-4">
			<ThreadSummaryPanel
				thread={thread}
				action={
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
			/>

			<div className="flex w-full items-center gap-1 rounded-[0.82rem] border border-white/8 bg-[#07090b] p-1.5 shadow-[0_8px_24px_rgba(0,0,0,0.16)] sm:gap-2 sm:p-2">
				<TabButton
					isActive={activeTab === "highlights"}
					onClick={() => setActiveTab("highlights")}
					icon={<Sparkles className="size-3.5 shrink-0 sm:size-4" />}
					label="Highlights"
				/>
				<TabButton
					isActive={activeTab === "transcript"}
					onClick={() => setActiveTab("transcript")}
					icon={<MessageSquare className="size-3.5 shrink-0 sm:size-4" />}
					label="Timeline"
				/>
				<TabButton
					isActive={activeTab === "links"}
					onClick={() => setActiveTab("links")}
					icon={<Link2 className="size-3.5 shrink-0 sm:size-4" />}
					label="Links"
					disabled={links.length === 0}
				/>
				<TabButton
					isActive={activeTab === "files"}
					onClick={() => setActiveTab("files")}
					icon={<Paperclip className="size-3.5 shrink-0 sm:size-4" />}
					label="Files"
					disabled={allFiles.length === 0}
				/>
			</div>

			<div className="mt-2">
				{activeTab === "highlights" && (
					<SectionCard eyebrow="Highlights" title="Important moments">
						<div className="space-y-2.5">
							{highlightedMessages.map((message) => (
								<ThreadMessageCard
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
				)}

				{activeTab === "transcript" && (
					<SectionCard eyebrow="Full transcript" title="Conversation timeline">
						<div className="space-y-2.5">
							{visibleMessages.map((message) => (
								<ThreadMessageCard
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
				)}

				{activeTab === "links" && (
					<SectionCard eyebrow="Links" title="Linked references">
						<ThreadLinksPanel links={links} />
					</SectionCard>
				)}

				{activeTab === "files" && (
					<SectionCard eyebrow="Files" title="Attached files">
						{allFiles.length ? (
							<ThreadFilesPanel
								files={allFiles}
								onOpenFile={(messageTs, index) =>
									setSelectedFileState({ messageTs, index })
								}
							/>
						) : (
							<EmptyState
								title="No files attached"
								description="When Slack file shares are present in the thread, they will show up here."
								icon={<Paperclip className="size-5" />}
							/>
						)}
					</SectionCard>
				)}
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

function TabButton({
	isActive,
	onClick,
	icon,
	label,
	disabled,
}: {
	isActive: boolean;
	onClick: () => void;
	icon: ReactNode;
	label: string;
	disabled?: boolean;
}) {
	return (
		<button
			type="button"
			onClick={onClick}
			disabled={disabled}
			className={cn(
				"flex min-w-0 flex-1 items-center justify-center gap-1 rounded-[0.6rem] px-1 py-1.5 text-[0.62rem] font-medium transition-colors sm:gap-2 sm:px-3 sm:py-2 sm:text-[0.8rem] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[#20cb74]/40",
				isActive
					? "bg-[#1fc86f]/15 text-[#29d779]"
					: "text-[#9da0a8] hover:bg-white/[0.05] hover:text-white",
				disabled &&
					"cursor-not-allowed opacity-40 hover:bg-transparent hover:text-[#9da0a8]",
			)}
		>
			{icon}
			<span className="truncate">{label}</span>
		</button>
	);
}
