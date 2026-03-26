import { Button } from "@/components/ui/button";
import { formatSlackTimestamp } from "@/lib/format";
import { isImageFile } from "@/lib/thread-files";
import {
	ChevronLeft,
	ChevronRight,
	ExternalLink,
	Paperclip,
	X,
} from "lucide-react";
import { useEffect, useId, useRef } from "react";

export interface ThreadViewerFile {
	id: string;
	name: string;
	permalink: string | null;
	mimetype: string | null;
	size: number | null;
}

interface ThreadFileViewerProps {
	files: ThreadViewerFile[];
	currentIndex: number;
	messageTs: string;
	onClose: () => void;
	onChangeIndex: (index: number) => void;
}

export function ThreadFileViewer({
	files,
	currentIndex,
	messageTs,
	onClose,
	onChangeIndex,
}: ThreadFileViewerProps) {
	const currentFile = files[currentIndex];
	const dialogRef = useRef<HTMLDialogElement | null>(null);
	const closeButtonRef = useRef<HTMLButtonElement | null>(null);
	const previouslyFocusedElementRef = useRef<HTMLElement | null>(null);
	const touchStateRef = useRef<{ startX: number; startY: number } | null>(null);
	const titleId = useId();
	const descriptionId = useId();

	useEffect(() => {
		if (typeof document === "undefined") {
			return;
		}

		const { body, activeElement } = document;
		const previousOverflow = body.style.overflow;
		body.style.overflow = "hidden";
		previouslyFocusedElementRef.current =
			activeElement instanceof HTMLElement ? activeElement : null;
		closeButtonRef.current?.focus();

		return () => {
			body.style.overflow = previousOverflow;
			previouslyFocusedElementRef.current?.focus();
		};
	}, []);

	useEffect(() => {
		function handleKeyDown(event: KeyboardEvent) {
			if (event.key === "Escape") {
				event.preventDefault();
				onClose();
				return;
			}

			if (event.key === "ArrowLeft" && currentIndex > 0) {
				event.preventDefault();
				onChangeIndex(currentIndex - 1);
				return;
			}

			if (event.key === "ArrowRight" && currentIndex < files.length - 1) {
				event.preventDefault();
				onChangeIndex(currentIndex + 1);
				return;
			}

			if (event.key === "Tab") {
				trapDialogFocus(event, dialogRef.current);
			}
		}

		window.addEventListener("keydown", handleKeyDown);
		return () => window.removeEventListener("keydown", handleKeyDown);
	}, [currentIndex, files.length, onChangeIndex, onClose]);

	if (!currentFile) {
		return null;
	}

	return (
		<div
			className="fixed inset-0 z-50 bg-(--color-overlay) backdrop-blur-md"
			onPointerUp={(event) => {
				if (event.target !== event.currentTarget) {
					return;
				}
				event.preventDefault();
				event.stopPropagation();
				onClose();
			}}
		>
			<dialog
				open
				ref={dialogRef}
				aria-modal="true"
				aria-labelledby={titleId}
				aria-describedby={descriptionId}
				tabIndex={-1}
				className="flex h-full w-full max-w-none flex-col border-0 bg-transparent p-0 text-inherit"
			>
				<div className="flex items-start justify-between gap-3 border-b border-(--color-border-subtle) px-4 py-3 sm:px-5">
					<div className="min-w-0">
						<p id={descriptionId} className="sr-only">
							Preview attached files. Use the arrow keys or the previous and
							next buttons to move between files, and press Escape to close.
						</p>
						<p className="text-eyebrow text-[0.62rem] font-medium uppercase tracking-[0.22em]">
							File preview
						</p>
						<h2
							id={titleId}
							className="mt-1 truncate text-[0.95rem] font-medium text-(--color-text-primary)"
						>
							{currentFile.name}
						</h2>
						<p className="text-copy-soft mt-1 text-[0.72rem]">
							{formatSlackTimestamp(messageTs)}
							{files.length > 1 ? ` · ${currentIndex + 1}/${files.length}` : ""}
						</p>
					</div>
					<div className="flex items-center gap-2">
						{currentFile.permalink ? (
							<a
								href={currentFile.permalink}
								target="_blank"
								rel="noreferrer"
								aria-label={`Open ${currentFile.name}`}
								className="viewer-action-button inline-flex h-9 items-center gap-1.5 rounded-full px-3 text-[0.74rem]"
							>
								<ExternalLink className="size-3.5" />
								<span className="hidden sm:inline">Open</span>
							</a>
						) : null}
						<Button
							ref={closeButtonRef}
							type="button"
							variant="secondary"
							size="sm"
							className="viewer-action-button size-9 rounded-full px-0"
							aria-label="Close file preview"
							onClick={onClose}
						>
							<X className="size-4" />
						</Button>
					</div>
				</div>

				<div className="flex min-h-0 flex-1 flex-col px-3 py-3 sm:px-5">
					<div className="flex min-h-0 flex-1 items-center justify-center gap-2">
						{currentIndex > 0 ? (
							<NavButton
								direction="prev"
								fileName={currentFile.name}
								onClick={() => onChangeIndex(currentIndex - 1)}
							/>
						) : (
							<NavButtonSpacer />
						)}

						<div
							className="viewer-stage flex min-h-0 flex-1 items-center justify-center overflow-hidden"
							onTouchStart={(event) => {
								const touch = event.touches[0];
								touchStateRef.current = {
									startX: touch.clientX,
									startY: touch.clientY,
								};
							}}
							onTouchEnd={(event) => {
								if (!touchStateRef.current) {
									return;
								}

								const touch = event.changedTouches[0];
								const direction = resolveFileViewerSwipeDirection({
									deltaX: touch.clientX - touchStateRef.current.startX,
									deltaY: touch.clientY - touchStateRef.current.startY,
									currentIndex,
									fileCount: files.length,
								});
								touchStateRef.current = null;

								if (direction === "prev") {
									onChangeIndex(currentIndex - 1);
									return;
								}

								if (direction === "next") {
									onChangeIndex(currentIndex + 1);
								}
							}}
							onTouchCancel={() => {
								touchStateRef.current = null;
							}}
						>
							{currentFile.permalink ? (
								renderFilePreview(currentFile)
							) : (
								<div className="viewer-empty flex h-full w-full flex-col items-center justify-center gap-3 px-6 text-center">
									<Paperclip className="size-8" />
									<p className="text-sm text-(--color-text-primary)">
										{currentFile.name}
									</p>
									<p className="text-[0.78rem]">
										Preview unavailable for this file in the current view.
									</p>
								</div>
							)}
						</div>

						{currentIndex < files.length - 1 ? (
							<NavButton
								direction="next"
								fileName={currentFile.name}
								onClick={() => onChangeIndex(currentIndex + 1)}
							/>
						) : (
							<NavButtonSpacer />
						)}
					</div>
					{files.length > 1 ? (
						<MobileViewerControls
							currentIndex={currentIndex}
							fileCount={files.length}
							onPrevious={() => onChangeIndex(currentIndex - 1)}
							onNext={() => onChangeIndex(currentIndex + 1)}
						/>
					) : null}
				</div>
			</dialog>
		</div>
	);
}

function NavButton({
	direction,
	fileName,
	onClick,
}: {
	direction: "prev" | "next";
	fileName: string;
	onClick: () => void;
}) {
	const Icon = direction === "prev" ? ChevronLeft : ChevronRight;

	return (
		<Button
			type="button"
			variant="secondary"
			size="sm"
			className="viewer-action-button hidden size-10 shrink-0 rounded-full px-0 sm:inline-flex"
			aria-label={`${direction === "prev" ? "Previous" : "Next"} file from ${fileName}`}
			onPointerDown={(event) => event.stopPropagation()}
			onClick={onClick}
		>
			<Icon className="size-4" />
		</Button>
	);
}

function NavButtonSpacer() {
	return (
		<div aria-hidden="true" className="hidden size-10 shrink-0 sm:block" />
	);
}

function MobileViewerControls({
	currentIndex,
	fileCount,
	onPrevious,
	onNext,
}: {
	currentIndex: number;
	fileCount: number;
	onPrevious: () => void;
	onNext: () => void;
}) {
	return (
		<div className="mt-3 grid grid-cols-[minmax(0,1fr)_auto_minmax(0,1fr)] items-center gap-2 sm:hidden">
			<Button
				type="button"
				variant="secondary"
				size="sm"
				className="viewer-action-button h-11 justify-center rounded-full px-4"
				aria-label="Previous file"
				disabled={currentIndex === 0}
				onClick={onPrevious}
			>
				<ChevronLeft className="size-4" />
				Prev
			</Button>
			<p className="text-copy-soft px-2 text-[0.76rem] tabular-nums">
				{currentIndex + 1}/{fileCount}
			</p>
			<Button
				type="button"
				variant="secondary"
				size="sm"
				className="viewer-action-button h-11 justify-center rounded-full px-4"
				aria-label="Next file"
				disabled={currentIndex >= fileCount - 1}
				onClick={onNext}
			>
				Next
				<ChevronRight className="size-4" />
			</Button>
		</div>
	);
}

function renderFilePreview(file: ThreadViewerFile) {
	if (isImageFile(file.mimetype)) {
		return (
			<img
				src={file.permalink ?? undefined}
				alt={file.name}
				className="h-full max-h-full w-full object-contain"
			/>
		);
	}

	return (
		<iframe
			src={file.permalink ?? undefined}
			title={file.name}
			className="h-full w-full"
		/>
	);
}

function trapDialogFocus(
	event: KeyboardEvent,
	container: HTMLDialogElement | null,
) {
	if (!container || typeof document === "undefined") {
		return;
	}

	const focusableElements = Array.from(
		container.querySelectorAll<HTMLElement>(
			'a[href], button:not([disabled]), iframe, input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex="-1"])',
		),
	).filter((element) => element.tabIndex !== -1);

	if (!focusableElements.length) {
		event.preventDefault();
		container.focus();
		return;
	}

	const firstElement = focusableElements[0];
	const lastElement = focusableElements[focusableElements.length - 1];
	const activeElement =
		document.activeElement instanceof HTMLElement
			? document.activeElement
			: null;

	if (event.shiftKey) {
		if (!activeElement || activeElement === firstElement) {
			event.preventDefault();
			lastElement.focus();
		}
		return;
	}

	if (!activeElement || activeElement === lastElement) {
		event.preventDefault();
		firstElement.focus();
	}
}

export function resolveFileViewerSwipeDirection({
	deltaX,
	deltaY,
	currentIndex,
	fileCount,
}: {
	deltaX: number;
	deltaY: number;
	currentIndex: number;
	fileCount: number;
}) {
	if (Math.abs(deltaX) < 48 || Math.abs(deltaX) <= Math.abs(deltaY)) {
		return null;
	}

	if (deltaX > 0) {
		return currentIndex > 0 ? "prev" : null;
	}

	return currentIndex < fileCount - 1 ? "next" : null;
}
