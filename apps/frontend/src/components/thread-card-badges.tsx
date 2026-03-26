import { cn } from "@/lib/utils";
import { BookmarkCheck, CloudOff, Sparkles } from "lucide-react";
import { type CSSProperties, useEffect, useState } from "react";

interface AiBadgeSparkle {
	driftX: string;
	driftY: string;
	delay: string;
	duration: string;
	id: string;
	left: string;
	opacity: string;
	rotate: string;
	size: string;
	top: string;
}

export function SavedStateBadge({ state }: { state: "saved" | "offline" }) {
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

export function AiStateBadge() {
	const [sparkles, setSparkles] = useState<AiBadgeSparkle[]>(() =>
		buildAiBadgeSparkles(),
	);

	useEffect(() => {
		let isActive = true;
		let timeoutId: ReturnType<typeof setTimeout> | null = null;

		function scheduleRefresh() {
			timeoutId = setTimeout(
				() => {
					if (!isActive) {
						return;
					}

					setSparkles(buildAiBadgeSparkles());
					scheduleRefresh();
				},
				900 + Math.random() * 1800,
			);
		}

		scheduleRefresh();

		return () => {
			isActive = false;
			if (timeoutId !== null) {
				clearTimeout(timeoutId);
			}
		};
	}, []);

	return (
		<span className="ai-state-badge inline-flex items-center gap-1 rounded-full border border-(--color-border-accent) bg-(--color-accent)/12 px-2 py-0.5 text-[0.67rem] font-semibold tracking-[0.02em] text-(--color-accent-soft)">
			<span aria-hidden="true" className="ai-state-badge__sparkle-layer">
				{sparkles.map((sparkle) => (
					<span
						key={sparkle.id}
						className="ai-state-badge__sparkle"
						style={
							{
								"--ai-sparkle-drift-x": sparkle.driftX,
								"--ai-sparkle-drift-y": sparkle.driftY,
								"--ai-sparkle-delay": sparkle.delay,
								"--ai-sparkle-duration": sparkle.duration,
								"--ai-sparkle-left": sparkle.left,
								"--ai-sparkle-opacity": sparkle.opacity,
								"--ai-sparkle-rotate": sparkle.rotate,
								"--ai-sparkle-size": sparkle.size,
								"--ai-sparkle-top": sparkle.top,
							} as CSSProperties
						}
					/>
				))}
			</span>
			<Sparkles className="ai-state-badge__icon size-3" />
			AI
		</span>
	);
}

function buildAiBadgeSparkles() {
	const count = 1 + Math.floor(Math.random() * 5);
	const sparkles: AiBadgeSparkle[] = [];

	for (let index = 0; index < count; index += 1) {
		const left = 10 + Math.random() * 80;
		const top = 18 + Math.random() * 64;
		const driftX = -18 + Math.random() * 36;
		const driftY = -14 + Math.random() * 28;
		const size = 0.22 + Math.random() * 0.4;
		const duration = 1 + Math.random() * 2.4;
		const delay = -(Math.random() * duration);
		const rotate = Math.round(Math.random() * 180);
		const opacity = 0.42 + Math.random() * 0.48;

		sparkles.push({
			delay: `${delay.toFixed(3)}s`,
			driftX: `${driftX.toFixed(2)}%`,
			driftY: `${driftY.toFixed(2)}%`,
			duration: `${duration.toFixed(3)}s`,
			id: `sparkle-${index}-${Math.random().toString(36).slice(2, 10)}`,
			left: `${left.toFixed(2)}%`,
			opacity: opacity.toFixed(3),
			rotate: `${rotate}deg`,
			size: `${size.toFixed(3)}rem`,
			top: `${top.toFixed(2)}%`,
		});
	}

	return sparkles;
}
