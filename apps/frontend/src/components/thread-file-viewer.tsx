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
import { useEffect } from "react";

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

	useEffect(() => {
		function handleKeyDown(event: KeyboardEvent) {
			if (event.key === "Escape") {
				onClose();
				return;
			}

			if (event.key === "ArrowLeft" && currentIndex > 0) {
				onChangeIndex(currentIndex - 1);
			}

			if (event.key === "ArrowRight" && currentIndex < files.length - 1) {
				onChangeIndex(currentIndex + 1);
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
			className="fixed inset-0 z-50 bg-black/92 backdrop-blur-md"
			onPointerDown={onClose}
		>
			<div className="flex h-full flex-col">
				<div className="flex items-start justify-between gap-3 border-b border-white/8 px-4 py-3 sm:px-5">
					<div className="min-w-0">
						<p className="text-eyebrow text-[0.62rem] font-medium uppercase tracking-[0.22em]">
							File preview
						</p>
						<h3 className="mt-1 truncate text-[0.95rem] font-medium text-white">
							{currentFile.name}
						</h3>
						<p className="text-copy-soft mt-1 text-[0.72rem]">
							{formatSlackTimestamp(messageTs)}
							{files.length > 1 ? ` · ${currentIndex + 1}/${files.length}` : ""}
						</p>
					</div>
					<div
						className="flex items-center gap-2"
						onPointerDown={(event) => event.stopPropagation()}
					>
						{currentFile.permalink ? (
							<a
								href={currentFile.permalink}
								target="_blank"
								rel="noreferrer"
								className="viewer-action-button inline-flex h-9 items-center gap-1.5 rounded-full px-3 text-[0.74rem]"
							>
								<ExternalLink className="size-3.5" />
								<span className="hidden sm:inline">Open</span>
							</a>
						) : null}
						<Button
							type="button"
							variant="secondary"
							size="sm"
							className="viewer-action-button size-9 rounded-full px-0"
							onClick={onClose}
						>
							<X className="size-4" />
						</Button>
					</div>
				</div>

				<div className="flex min-h-0 flex-1 items-center justify-center gap-2 px-3 py-3 sm:px-5">
					{currentIndex > 0 ? (
						<NavButton
							direction="prev"
							onClick={() => onChangeIndex(currentIndex - 1)}
						/>
					) : null}

					<div
						className="viewer-stage flex min-h-0 flex-1 items-center justify-center overflow-hidden"
						onPointerDown={(event) => event.stopPropagation()}
					>
						{currentFile.permalink ? (
							renderFilePreview(currentFile)
						) : (
							<div className="viewer-empty flex h-full w-full flex-col items-center justify-center gap-3 px-6 text-center">
								<Paperclip className="size-8" />
								<p className="text-sm text-white">{currentFile.name}</p>
								<p className="text-[0.78rem]">
									Preview unavailable for this file in the current view.
								</p>
							</div>
						)}
					</div>

					{currentIndex < files.length - 1 ? (
						<NavButton
							direction="next"
							onClick={() => onChangeIndex(currentIndex + 1)}
						/>
					) : null}
				</div>
			</div>
		</div>
	);
}

function NavButton({
	direction,
	onClick,
}: {
	direction: "prev" | "next";
	onClick: () => void;
}) {
	const Icon = direction === "prev" ? ChevronLeft : ChevronRight;

	return (
		<Button
			type="button"
			variant="secondary"
			size="sm"
			className="viewer-action-button hidden size-10 shrink-0 rounded-full px-0 sm:inline-flex"
			onPointerDown={(event) => event.stopPropagation()}
			onClick={onClick}
		>
			<Icon className="size-4" />
		</Button>
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
