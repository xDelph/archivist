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
				"inline-flex items-center rounded-full border px-2.5 py-1 text-[0.8rem] font-medium leading-none",
				channelTone(name),
				className,
			)}
		>
			#{channelLabel(name)}
		</span>
	);
}
