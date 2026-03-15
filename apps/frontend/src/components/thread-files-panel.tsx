import type { ThreadViewerFile } from "@/components/thread-file-viewer";
import { Paperclip } from "lucide-react";

import { isImageFile } from "@/lib/thread-files";

export interface ThreadListedFile extends ThreadViewerFile {
	messageTs: string;
	messageFileIndex: number;
}

interface ThreadFilesPanelProps {
	files: ThreadListedFile[];
	onOpenFile: (messageTs: string, fileIndex: number) => void;
}

export function ThreadFilesPanel({ files, onOpenFile }: ThreadFilesPanelProps) {
	return (
		<ul className="space-y-2">
			{files.map((file) => (
				<li
					key={`${file.messageTs}-${file.id}-${file.name}`}
					className="surface-subpanel p-3"
				>
					<button
						type="button"
						className="flex w-full items-center gap-3 text-left"
						onClick={() => onOpenFile(file.messageTs, file.messageFileIndex)}
					>
						<FileThumbnail file={file} />
						<div className="min-w-0 flex-1">
							<p className="truncate text-[0.82rem] font-medium text-white">
								{file.name}
							</p>
							<p className="text-copy-soft mt-1 text-[0.72rem]">
								{file.mimetype || "unknown type"}
							</p>
							{file.permalink ? (
								<span className="link-accent mt-2 inline-flex items-center gap-1.5 text-[0.8rem]">
									<Paperclip className="size-4" />
									Open file
								</span>
							) : null}
						</div>
					</button>
				</li>
			))}
		</ul>
	);
}

function FileThumbnail({ file }: { file: ThreadViewerFile }) {
	if (file.permalink && isImageFile(file.mimetype)) {
		return (
			<div className="surface-thumbnail relative h-16 w-16 shrink-0 overflow-hidden">
				<img
					src={file.permalink}
					alt={file.name}
					className="h-full w-full object-cover"
				/>
			</div>
		);
	}

	return (
		<div className="surface-thumbnail text-copy-soft flex h-16 w-16 shrink-0 items-center justify-center">
			<Paperclip className="size-5" />
		</div>
	);
}
