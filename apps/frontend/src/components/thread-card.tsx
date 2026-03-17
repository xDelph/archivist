import { ChannelBadge } from "@/components/channel-badge";
import { IdentityAvatar } from "@/components/identity-avatar";
import { ThreadMetrics } from "@/components/thread-metrics";
import { formatSlackTimestamp } from "@/lib/format";
import {
	type ThreadAuthor,
	displayAuthorName,
	renderSlackTextWithoutLinks,
} from "@/lib/thread-display";
import {
	clampSwipeOffset,
	getThreadSwipeRailWidth,
	resolveSwipeOffset,
} from "@/lib/thread-swipe";
import { cn } from "@/lib/utils";
import { Link, useRouterState } from "@tanstack/react-router";
import { BookmarkCheck, CloudOff, Sparkles } from "lucide-react";
import {
	Children,
	type ReactNode,
	type TouchEvent,
	isValidElement,
	useEffect,
	useId,
	useRef,
	useState,
} from "react";

const OPEN_THREAD_SWIPE_EVENT = "archivist:open-thread-swipe";

interface ThreadCardProps {
	threadId: string;
	channelName?: string | null;
	author?: ThreadAuthor | null;
	authorFallback?: string | null;
	title: ReactNode;
	preview: ReactNode;
	lastActivityTs: string;
	replyCount: number;
	participantCount: number;
	reactionCount: number;
	fileCount?: number;
	score?: number;
	rank?: number;
	className?: string;
	action?: ReactNode;
	isActionActive?: boolean;
	savedState?: "saved" | "offline";
	isHighlighted?: boolean;
	leadingSwipeActions?: ThreadCardSwipeAction[];
	trailingSwipeActions?: ThreadCardSwipeAction[];
}

interface ThreadCardSwipeAction {
	label: string;
	icon: ReactNode;
	onAction: () => void;
	disabled?: boolean;
	tone?: "accent" | "danger";
}

export function ThreadCard({
	threadId,
	channelName,
	author,
	authorFallback,
	title,
	preview,
	lastActivityTs,
	replyCount,
	participantCount,
	reactionCount,
	fileCount = 0,
	score,
	rank,
	className,
	action,
	isActionActive = false,
	savedState,
	isHighlighted = false,
	leadingSwipeActions = [],
	trailingSwipeActions = [],
}: ThreadCardProps) {
	const previewIsDuplicate = !shouldRenderThreadPreview(title, preview);
	const displayMessage = previewIsDuplicate ? title : preview;
	const leadingWidth = getThreadSwipeRailWidth(leadingSwipeActions.length);
	const trailingWidth = getThreadSwipeRailWidth(trailingSwipeActions.length);
	const instanceId = useId();
	const locationKey = useRouterState({
		select: (state) => `${state.location.pathname}${state.location.searchStr}`,
	});
	const previousLocationKeyRef = useRef(locationKey);
	const [swipeOffset, setSwipeOffset] = useState(0);
	const [isDragging, setIsDragging] = useState(false);
	const touchStateRef = useRef<{
		startX: number;
		startY: number;
		startOffset: number;
		axis: "x" | "y" | null;
	} | null>(null);

	function closeSwipeActions() {
		setSwipeOffset(0);
		setIsDragging(false);
	}

	function handleTouchStart(event: TouchEvent<HTMLElement>) {
		if (!leadingSwipeActions.length && !trailingSwipeActions.length) {
			return;
		}

		announceOpenThreadSwipe(instanceId);
		const touch = event.touches[0];
		touchStateRef.current = {
			startX: touch.clientX,
			startY: touch.clientY,
			startOffset: swipeOffset,
			axis: null,
		};
	}

	function handleTouchMove(event: TouchEvent<HTMLElement>) {
		if (!touchStateRef.current) {
			return;
		}

		const touch = event.touches[0];
		const deltaX = touch.clientX - touchStateRef.current.startX;
		const deltaY = touch.clientY - touchStateRef.current.startY;

		if (touchStateRef.current.axis === null) {
			if (Math.abs(deltaX) < 8 && Math.abs(deltaY) < 8) {
				return;
			}

			touchStateRef.current.axis =
				Math.abs(deltaX) > Math.abs(deltaY) ? "x" : "y";
		}

		if (touchStateRef.current.axis !== "x") {
			return;
		}

		setIsDragging(true);
		setSwipeOffset(
			clampSwipeOffset(
				touchStateRef.current.startOffset + deltaX,
				leadingWidth,
				trailingWidth,
			),
		);
	}

	function handleTouchEnd() {
		if (!touchStateRef.current) {
			return;
		}

		if (touchStateRef.current.axis === "x") {
			setSwipeOffset(
				resolveSwipeOffset(swipeOffset, leadingWidth, trailingWidth),
			);
		}

		touchStateRef.current = null;
		setIsDragging(false);
	}

	useEffect(() => {
		if (typeof window === "undefined") {
			return;
		}

		function handleOpen(event: Event) {
			const nextId = (event as CustomEvent<{ instanceId: string }>).detail
				?.instanceId;
			if (nextId && nextId !== instanceId) {
				setSwipeOffset(0);
				setIsDragging(false);
			}
		}

		window.addEventListener(OPEN_THREAD_SWIPE_EVENT, handleOpen);
		return () =>
			window.removeEventListener(OPEN_THREAD_SWIPE_EVENT, handleOpen);
	}, [instanceId]);

	useEffect(() => {
		if (swipeOffset === 0) {
			return;
		}

		function closeSwipeOnScroll() {
			setSwipeOffset(0);
			setIsDragging(false);
		}

		window.addEventListener("scroll", closeSwipeOnScroll, { passive: true });
		return () => window.removeEventListener("scroll", closeSwipeOnScroll);
	}, [swipeOffset]);

	useEffect(() => {
		if (previousLocationKeyRef.current === locationKey) {
			return;
		}

		previousLocationKeyRef.current = locationKey;
		setSwipeOffset(0);
		setIsDragging(false);
	}, [locationKey]);

	return (
		<div
			className={cn(
				"relative overflow-visible isolate",
				(swipeOffset !== 0 || isDragging || isActionActive) && "z-40 sm:z-20",
			)}
		>
			{leadingSwipeActions.length ? (
				<SwipeActionSurface
					side="leading"
					actions={leadingSwipeActions}
					onAction={closeSwipeActions}
				/>
			) : null}
			{trailingSwipeActions.length ? (
				<SwipeActionSurface
					side="trailing"
					actions={trailingSwipeActions}
					onAction={closeSwipeActions}
				/>
			) : null}

			<article
				className={cn(
					"surface-panel surface-panel-soft group relative px-3 py-3 transition-[background-color,border-color,box-shadow,transform] duration-200 hover:border-(--color-border-accent) hover:bg-(--color-bg-base) hover:shadow-[0_18px_40px_rgba(0,0,0,0.24)] focus-within:border-(--color-border-accent)",
					isDragging ? "duration-0" : "ease-[cubic-bezier(0.22,1,0.36,1)]",
					className,
				)}
				style={{
					transform:
						swipeOffset === 0
							? undefined
							: `translate3d(${swipeOffset}px, 0, 0)`,
					touchAction:
						leadingSwipeActions.length || trailingSwipeActions.length
							? "pan-y"
							: undefined,
				}}
				onTouchStart={handleTouchStart}
				onTouchMove={handleTouchMove}
				onTouchEnd={handleTouchEnd}
				onTouchCancel={handleTouchEnd}
			>
				<div className="flex items-start gap-2">
					{typeof rank === "number" ? (
						<div className="hidden min-w-5 justify-center pt-0.5 text-[1.15rem] font-semibold leading-none text-(--color-accent) xl:flex">
							{rank}
						</div>
					) : null}

					<IdentityAvatar
						author={author}
						fallback={authorFallback || channelName}
						size="md"
					/>

					<Link
						to="/threads/$threadId"
						params={{ threadId }}
						onClickCapture={(event) => {
							if (swipeOffset !== 0) {
								event.preventDefault();
								closeSwipeActions();
							}
						}}
						className={cn(
							"min-w-0 flex-1 rounded-[0.75rem] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-(--color-accent-soft)/40",
							action ? "sm:pr-16" : "",
						)}
					>
						<div className="flex items-start justify-between gap-2.5">
							<div className="min-w-0">
								<div className="flex flex-wrap items-center gap-1.5">
									<p className="truncate text-[0.88rem] font-medium text-white">
										{displayAuthorName(author, authorFallback || channelName)}
									</p>
									<ChannelBadge name={channelName} />
									{savedState ? <SavedStateBadge state={savedState} /> : null}
									{isHighlighted ? <HighlightStateBadge /> : null}
								</div>
								<div className="text-copy-bright mt-1.5 max-h-[3.9rem] overflow-hidden break-words whitespace-pre-wrap text-[0.88rem] leading-[1.45] font-normal [display:-webkit-box] [-webkit-box-orient:vertical] [-webkit-line-clamp:3] [tab-size:4]">
									{renderRichNode(displayMessage)}
								</div>
							</div>

							<div className="hidden shrink-0 text-right lg:block">
								<time className="text-copy-soft block text-[0.76rem]">
									{formatSlackTimestamp(lastActivityTs)}
								</time>
								{typeof score === "number" ? (
									<div className="accent-pill mt-2 rounded-[0.65rem] px-2 py-0.5 text-[0.7rem] font-medium">
										{score}
									</div>
								) : null}
							</div>
						</div>

						<ThreadMetrics
							className="mt-2.5"
							replyCount={replyCount}
							reactionCount={reactionCount}
							participantCount={participantCount}
							fileCount={fileCount}
						/>

						<div className="mt-2 flex items-center justify-between gap-3 lg:hidden">
							<time className="text-copy-soft text-[0.76rem]">
								{formatSlackTimestamp(lastActivityTs)}
							</time>
							{typeof score === "number" ? (
								<span className="accent-pill rounded-[0.65rem] px-2 py-0.5 text-[0.7rem] font-medium">
									{score}
								</span>
							) : null}
						</div>
					</Link>

					{action ? (
						<div className="absolute top-3 right-3 z-10 shrink-0">{action}</div>
					) : null}
				</div>
			</article>
		</div>
	);
}

function SavedStateBadge({ state }: { state: "saved" | "offline" }) {
	return (
		<span
			className={cn(
				"inline-flex items-center gap-1 rounded-full border px-2 py-0.5 text-[0.67rem] font-semibold tracking-[0.02em]",
				state === "offline"
					? "border-(--color-border-accent) bg-(--color-accent)/10 text-(--color-accent-soft)"
					: "border-(--color-border-default) bg-(--color-bg-elevated) text-(--color-text-secondary)",
			)}
		>
			{state === "offline" ? (
				<CloudOff className="size-3" />
			) : (
				<BookmarkCheck className="size-3" />
			)}
			{state === "offline" ? "Offline" : "Saved"}
		</span>
	);
}

function HighlightStateBadge() {
	return (
		<span className="inline-flex items-center gap-1 rounded-full border border-(--color-border-accent) bg-(--color-accent)/12 px-2 py-0.5 text-[0.67rem] font-semibold tracking-[0.02em] text-(--color-accent-soft)">
			<Sparkles className="size-3" />
			Highlight
		</span>
	);
}

function SwipeActionSurface({
	side,
	actions,
	onAction,
}: {
	side: "leading" | "trailing";
	actions: ThreadCardSwipeAction[];
	onAction: () => void;
}) {
	return (
		<div
			className={cn(
				"absolute inset-y-0 z-0 flex items-stretch gap-1.5 p-1.5 sm:hidden",
				side === "leading" ? "left-0 justify-start" : "right-0 justify-end",
			)}
		>
			{actions.map((action, index) => (
				<button
					key={`${action.label}-${index}`}
					type="button"
					className={cn(
						"flex w-[92px] flex-col items-center justify-center gap-1 rounded-[1.35rem] px-2 text-[0.72rem] font-semibold tracking-[0.01em] transition-[background-color,color,transform] duration-200 active:scale-[0.98] disabled:opacity-55",
						action.tone === "danger"
							? "bg-[color-mix(in_srgb,var(--color-destructive)_72%,var(--color-bg-surface))] text-white"
							: "bg-[color-mix(in_srgb,var(--color-accent)_26%,var(--color-bg-surface))] text-(--color-accent-soft)",
					)}
					disabled={action.disabled}
					onClick={() => {
						onAction();
						action.onAction();
					}}
				>
					{action.icon}
					<span>{action.label}</span>
				</button>
			))}
		</div>
	);
}

function renderRichNode(content: ReactNode) {
	return typeof content === "string"
		? renderSlackTextWithoutLinks(content)
		: content;
}

function announceOpenThreadSwipe(instanceId: string) {
	if (typeof window === "undefined") {
		return;
	}

	window.dispatchEvent(
		new CustomEvent(OPEN_THREAD_SWIPE_EVENT, {
			detail: { instanceId },
		}),
	);
}

export function shouldRenderThreadPreview(
	title: ReactNode,
	preview: ReactNode,
) {
	const normalizedTitle = normalizeRichText(title);
	const normalizedPreview = normalizeRichText(preview);
	return normalizedPreview.length > 0 && normalizedTitle !== normalizedPreview;
}

function normalizeRichText(content: ReactNode): string {
	return flattenText(content).replace(/\s+/g, " ").trim().toLowerCase();
}

function flattenText(content: ReactNode): string {
	if (
		content === null ||
		content === undefined ||
		typeof content === "boolean"
	) {
		return "";
	}

	if (typeof content === "string" || typeof content === "number") {
		return String(content);
	}

	if (Array.isArray(content)) {
		return content.map((item) => flattenText(item)).join("");
	}

	if (isValidElement<{ children?: ReactNode }>(content)) {
		return flattenText(content.props.children);
	}

	return Children.toArray(content)
		.map((item) => flattenText(item))
		.join("");
}
