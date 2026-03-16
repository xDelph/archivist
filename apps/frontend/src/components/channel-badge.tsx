import { channelLabel, channelTone } from "@/lib/thread-display";
import { cn } from "@/lib/utils";

export function ChannelBadge({
	name,
	className,
}: {
	name: string | null | undefined;
	className?: string;
}) {
	return (
		<span
			className={cn(
				"inline-flex max-w-full items-center truncate rounded-[0.8rem] border px-2.5 py-1 text-[0.72rem] font-medium leading-none shadow-[inset_0_1px_0_rgba(255,255,255,0.04)]",
				channelTone(name),
				className,
			)}
		>
			#{channelLabel(name)}
		</span>
	);
}
