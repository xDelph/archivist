import { cn } from "@/lib/utils";
import { BookmarkCheck, CloudOff, Sparkles } from "lucide-react";

export function SavedStateBadge({
	state,
}: {
	state: "saved" | "offline";
}) {
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

export function HighlightStateBadge() {
	return (
		<span className="inline-flex items-center gap-1 rounded-full border border-(--color-border-accent) bg-(--color-accent)/12 px-2 py-0.5 text-[0.67rem] font-semibold tracking-[0.02em] text-(--color-accent-soft)">
			<Sparkles className="size-3" />
			Highlight
		</span>
	);
}
