import { IdentityAvatar } from "@/components/identity-avatar";
import type { ThreadViewerFile } from "@/components/thread-file-viewer";
import { formatSlackTimestamp } from "@/lib/format";
import {
	displayAuthorName,
	groupReactions,
	renderSlackText,
} from "@/lib/thread-display";
import { Paperclip } from "lucide-react";
import type { ReactNode } from "react";

interface ThreadMessageCardProps {
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

export function ThreadMessageCard({
	message,
	onOpenFile,
}: ThreadMessageCardProps) {
	const reactions = groupReactions(message.reactions);

	return (
		<article className="surface-subpanel p-3.5">
			<div className="flex items-start gap-2.5">
				<IdentityAvatar
					author={message.author}
					fallback={message.user_id}
					size="sm"
				/>
				<div className="min-w-0 flex-1">
					<div className="text-copy-soft flex flex-wrap items-center gap-1.5 text-[0.78rem]">
						<span className="text-[0.9rem] font-medium text-white">
							{displayAuthorName(message.author, message.user_id)}
						</span>
						<span className="separator-faint">&middot;</span>
						<time className="tabular-nums">
							{formatSlackTimestamp(message.ts)}
						</time>
						{message.thread_ts ? (
							<>
								<span className="separator-faint">&middot;</span>
								<span>Reply</span>
							</>
						) : null}
					</div>
					<div className="text-copy-bright mt-2 whitespace-pre-wrap text-[0.92rem] leading-7 font-normal">
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
						className="subtle-chip inline-flex min-h-9 items-center gap-1 rounded-full px-2.5 py-1 text-[0.78rem] hover:border-(--color-border-accent) hover:text-white focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-(--color-accent-soft)/40"
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
		<span className="subtle-chip inline-flex min-h-8 items-center gap-1 rounded-full px-2.5 py-1 text-[0.76rem]">
			{icon}
			{label}
		</span>
	);
}
