import { EmptyState } from "@/components/empty-state";
import { CardSkeletonList } from "@/components/query-state";
import { SectionCard } from "@/components/section-card";
import { ThreadFileViewer } from "@/components/thread-file-viewer";
import { ThreadFilesPanel } from "@/components/thread-files-panel";
import { ThreadLinksPanel } from "@/components/thread-links-panel";
import { ThreadMessageCard } from "@/components/thread-message-card";
import { ThreadSummaryPanel } from "@/components/thread-summary-panel";
import { Button } from "@/components/ui/button";
import { SegmentedTabs } from "@/components/ui/segmented-tabs";
import { deleteSavedThread, saveThread } from "@/lib/api";
import {
	removeOfflineSavedThread,
	storeOfflineSavedThread,
	storeOfflineThreadDetail,
} from "@/lib/offline-library";
import {
	findSavedItemForReading,
	resolveThreadForReading,
} from "@/lib/offline-reading";
import { offlineQueries, savedQueries, threadQueries } from "@/lib/queries";
import { extractLinks } from "@/lib/thread-display";
import { useNetworkStatus } from "@/lib/use-network-status";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useParams } from "@tanstack/react-router";
import {
	Bookmark,
	BookmarkCheck,
	ChevronDown,
	CloudOff,
	Link2,
	MessageSquare,
	Paperclip,
	Sparkles,
} from "lucide-react";
import { useEffect, useState } from "react";

const INITIAL_MESSAGE_COUNT = 4;

export function ThreadPage() {
	const { threadId } = useParams({ from: "/app/threads/$threadId" });
	const { isOnline } = useNetworkStatus();
	const queryClient = useQueryClient();
	const [isExpanded, setIsExpanded] = useState(false);
	const [selectedFileState, setSelectedFileState] = useState<{
		messageTs: string;
		index: number;
	} | null>(null);
	const [activeTab, setActiveTab] = useState<
		"highlights" | "transcript" | "links" | "files"
	>("highlights");

	const threadQuery = useQuery({
		...threadQueries.detail(threadId),
		enabled: isOnline,
	});
	const offlineThreadQuery = useQuery(offlineQueries.thread(threadId));
	const savedQuery = useQuery({
		...savedQueries.list(),
		enabled: isOnline,
	});
	const offlineSavedQuery = useQuery(offlineQueries.saved());

	const savedItem = findSavedItemForReading(
		threadId,
		savedQuery.data?.items,
		offlineSavedQuery.data,
	);
	const thread = resolveThreadForReading(
		threadQuery.data,
		offlineThreadQuery.data,
	);
	const isOfflineSnapshot = !threadQuery.data && Boolean(thread);

	const saveMutation = useMutation({
		mutationFn: async () => {
			if (savedItem) {
				await deleteSavedThread(threadId);
				return { kind: "removed" } as const;
			}

			return {
				kind: "saved",
				result: await saveThread(threadId),
			} as const;
		},
		onSuccess: async (result) => {
			if (!thread) {
				return;
			}

			if (result.kind === "removed") {
				await removeOfflineSavedThread(threadId);
			} else {
				await storeOfflineSavedThread(result.result.item, thread);
			}
			await queryClient.invalidateQueries({ queryKey: ["saved"] });
			await queryClient.invalidateQueries({ queryKey: ["offline"] });
		},
	});

	useEffect(() => {
		if (!isOnline || !threadQuery.data || !savedItem) {
			return;
		}

		void storeOfflineThreadDetail(threadQuery.data).then(async () => {
			await queryClient.invalidateQueries({
				queryKey: ["offline", "thread", threadId],
			});
		});
	}, [isOnline, queryClient, savedItem, threadId, threadQuery.data]);

	if (
		(isOnline && threadQuery.isPending) ||
		(!isOnline && offlineThreadQuery.isPending)
	) {
		return (
			<div className="mx-auto w-full max-w-3xl pb-8">
				<CardSkeletonList
					cardClassName="surface-frost h-36 rounded-[1.75rem]"
					className="space-y-4"
				/>
			</div>
		);
	}

	if (!thread) {
		return (
			<EmptyState
				title={isOnline ? "Thread unavailable" : "Thread unavailable offline"}
				description={
					isOnline
						? "The requested thread could not be loaded from the local API."
						: "This thread has not been downloaded for offline reading yet. Reopen it while connected after saving it."
				}
			/>
		);
	}

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
	const tabItems = [
		{
			key: "highlights",
			label: "Highlights",
			icon: <Sparkles className="size-3.5 shrink-0 sm:size-4" />,
		},
		{
			key: "transcript",
			label: "Timeline",
			icon: <MessageSquare className="size-3.5 shrink-0 sm:size-4" />,
		},
		{
			key: "links",
			label: "Links",
			icon: <Link2 className="size-3.5 shrink-0 sm:size-4" />,
			disabled: links.length === 0,
		},
		{
			key: "files",
			label: "Files",
			icon: <Paperclip className="size-3.5 shrink-0 sm:size-4" />,
			disabled: allFiles.length === 0,
		},
	] as const;

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
								? "size-9 rounded-full bg-(--color-accent) px-0 text-black hover:bg-(--color-accent-strong) sm:h-8 sm:w-auto sm:rounded-lg sm:px-3 sm:text-[0.74rem]"
								: "button-ghost size-9 rounded-full px-0 sm:h-8 sm:w-auto sm:rounded-lg sm:px-3 sm:text-[0.74rem]"
						}
						onClick={() => saveMutation.mutate()}
						disabled={saveMutation.isPending || !isOnline}
						title={isOnline ? undefined : "Reconnect to manage saved threads"}
					>
						{savedItem ? (
							<BookmarkCheck className="size-4 sm:size-3.5" />
						) : (
							<Bookmark className="size-4 sm:size-3.5" />
						)}
						<span className="hidden sm:inline">
							{!isOnline
								? savedItem
									? "Saved offline"
									: "Offline"
								: saveMutation.isPending
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

			<SegmentedTabs
				items={tabItems}
				value={activeTab}
				onChange={setActiveTab}
			/>

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
									className="button-ghost px-3 py-2 text-[0.78rem]"
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
