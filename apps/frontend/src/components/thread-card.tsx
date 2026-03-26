import { ChannelBadge } from "@/components/channel-badge";
import { IdentityAvatar } from "@/components/identity-avatar";
import { AiStateBadge, SavedStateBadge } from "@/components/thread-card-badges";
import { ThreadMetrics } from "@/components/thread-metrics";
import type { ThreadPreviewSource } from "@/lib/api";
import { formatSlackTimestamp } from "@/lib/format";
import {
	type ThreadAuthor,
	displayAuthorName,
	renderSlackText,
} from "@/lib/thread-display";
import {
	clampSwipeOffset,
	getThreadSwipeRailWidth,
	resolveSwipeOffset,
} from "@/lib/thread-swipe";
import { cn } from "@/lib/utils";
import { Link, useRouterState } from "@tanstack/react-router";
import { Star } from "lucide-react";
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

const OPEN_THREAD_SWIPE_EVENT = "arkivist:open-thread-swipe";

interface ThreadCardProps {
	threadId: string;
	channelName?: string | null;
	author?: ThreadAuthor | null;
	authorFallback?: string | null;
	title: ReactNode;
	preview: ReactNode;
	previewSource?: ThreadPreviewSource;
	lastActivityTs: string;
	replyCount: number;
	participantCount: number;
	reactionCount: number;
	fileCount?: number;
	score?: number;
	rank?: number;
	className?: string;
	action?: ReactNode;
	mobileAction?: ReactNode;
	isActionActive?: boolean;
	savedState?: "saved" | "offline";
	isStarred?: boolean;
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
	previewSource = "fallback",
	lastActivityTs,
	replyCount,
	participantCount,
	reactionCount,
	fileCount = 0,
	score,
	rank,
	className,
	action,
	mobileAction,
	isActionActive = false,
	savedState,
	isStarred = false,
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
					previewSource === "ai" && "border-(--color-accent-soft)/55",
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
				<Link
					to="/threads/$threadId"
					params={{ threadId }}
					onClickCapture={(event) => {
						if (swipeOffset !== 0) {
							event.preventDefault();
							closeSwipeActions();
						}
					}}
					className="absolute inset-0 z-0 rounded-[0.75rem] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-(--color-accent-soft)/40"
				>
					<span className="sr-only">
						Open thread from{" "}
						{displayAuthorName(author, authorFallback || channelName)}
					</span>
				</Link>

				<div className="relative z-10 flex items-start gap-2 pointer-events-none">
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

					<div className="min-w-0 flex-1 rounded-[0.75rem]">
						<div className="flex items-start justify-between gap-3">
							<div className="min-w-0 flex-1">
								<div className="flex flex-wrap items-center gap-1.5">
									<p className="truncate text-[0.88rem] font-medium text-(--color-text-primary)">
										{displayAuthorName(author, authorFallback || channelName)}
									</p>
									<ChannelBadge name={channelName} />
									{previewSource === "ai" ? <AiStateBadge /> : null}
									{savedState ? <SavedStateBadge state={savedState} /> : null}
									{isStarred ? <StarStateBadge /> : null}
								</div>
							</div>

							<div className="hidden shrink-0 text-right sm:block">
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

						<div className="text-copy-bright mt-1.5 max-h-[3.9rem] overflow-hidden break-words whitespace-pre-wrap text-[0.88rem] leading-[1.45] font-normal pointer-events-none [display:-webkit-box] [-webkit-box-orient:vertical] [-webkit-line-clamp:3] sm:max-h-[5.2rem] sm:[-webkit-line-clamp:4] [tab-size:4] [&_a]:pointer-events-auto">
							{renderRichNode(displayMessage)}
						</div>

						<div className="mt-2.5 flex items-center justify-between gap-3">
							<div className="min-w-0 flex flex-1 items-center gap-3">
								<ThreadMetrics
									className="min-w-0 flex-1"
									replyCount={replyCount}
									reactionCount={reactionCount}
									participantCount={participantCount}
									fileCount={fileCount}
								/>
								<time className="text-copy-soft shrink-0 text-[0.76rem] sm:hidden">
									{formatSlackTimestamp(lastActivityTs)}
								</time>
							</div>
							{action ? (
								<div
									className={cn(
										"hidden shrink-0 items-center gap-1.5 transition-[opacity,transform] duration-200 ease-[cubic-bezier(0.16,1,0.3,1)] sm:flex",
										isActionActive
											? "pointer-events-auto translate-y-0 opacity-100"
											: "pointer-events-auto translate-y-0 opacity-100 lg:pointer-events-none lg:translate-y-1.5 lg:opacity-0 lg:group-hover:pointer-events-auto lg:group-hover:translate-y-0 lg:group-hover:opacity-100 lg:group-focus-within:pointer-events-auto lg:group-focus-within:translate-y-0 lg:group-focus-within:opacity-100",
									)}
								>
									{action}
								</div>
							) : null}
						</div>
						{mobileAction ? (
							<div className="mt-2.5 sm:hidden">{mobileAction}</div>
						) : null}

						{typeof score === "number" ? (
							<div className="mt-2 flex justify-end sm:hidden">
								<span className="accent-pill rounded-[0.65rem] px-2 py-0.5 text-[0.7rem] font-medium">
									{score}
								</span>
							</div>
						) : null}
					</div>
				</div>
			</article>
		</div>
	);
}

function StarStateBadge() {
	return (
		<span className="inline-flex items-center gap-1 rounded-full border border-(--color-border-accent) bg-(--color-accent)/12 px-2 py-0.5 text-[0.67rem] font-semibold tracking-[0.02em] text-(--color-accent-soft)">
			<Star className="size-3" />
			Star
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
							? "bg-[color-mix(in_srgb,var(--color-destructive)_72%,var(--color-bg-surface))] text-(--color-on-destructive)"
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
	return typeof content === "string" ? renderSlackText(content) : content;
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
