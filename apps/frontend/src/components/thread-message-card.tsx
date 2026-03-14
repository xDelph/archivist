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
