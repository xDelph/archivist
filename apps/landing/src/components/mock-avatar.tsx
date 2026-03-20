const TONE_STYLES: Record<number, { bg: string; color: string }> = {
	1: {
		bg: "linear-gradient(to bottom right, color-mix(in srgb, var(--color-accent) 28%, var(--color-bg-base)), var(--color-bg-deep))",
		color: "color-mix(in srgb, var(--color-accent) 84%, white 16%)",
	},
	2: {
		bg: "linear-gradient(to bottom right, color-mix(in srgb, oklch(0.75 0.13 72) 26%, var(--color-bg-base)), var(--color-bg-deep))",
		color: "color-mix(in srgb, oklch(0.75 0.13 72) 84%, white 16%)",
	},
	3: {
		bg: "linear-gradient(to bottom right, color-mix(in srgb, oklch(0.72 0.1 240) 24%, var(--color-bg-base)), var(--color-bg-deep))",
		color: "color-mix(in srgb, oklch(0.72 0.1 240) 84%, white 16%)",
	},
	4: {
		bg: "linear-gradient(to bottom right, #2a1022, #14070f)",
		color: "#f9a8d4",
	},
	5: {
		bg: "linear-gradient(to bottom right, #2c2c0d, #141406)",
		color: "#fde047",
	},
};

const SIZES = {
	sm: "h-6 w-6 text-[8px]",
	md: "h-8 w-8 text-[10px]",
	lg: "h-10 w-10 text-xs",
};

interface MockAvatarProps {
	initials: string;
	tone: number;
	size?: "sm" | "md" | "lg";
}

export function MockAvatar({ initials, tone, size = "md" }: MockAvatarProps) {
	const toneStyle = TONE_STYLES[tone] ?? TONE_STYLES[1];
	const sizeClass = SIZES[size];

	return (
		<div
			className={`${sizeClass} inline-flex shrink-0 items-center justify-center rounded-full border font-semibold`}
			style={{
				backgroundImage: toneStyle.bg,
				color: toneStyle.color,
				borderColor: "var(--color-border-strong)",
			}}
		>
			{initials}
		</div>
	);
}
