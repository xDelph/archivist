import { ACCENT_MODES, type AccentMode, useTheme } from "@/lib/theme";
import { cn } from "@/lib/utils";
import { MoonStar, SunMedium } from "lucide-react";

const accentLabels: Record<AccentMode, string> = {
	green: "Green",
	red: "Red",
	blue: "Blue",
};

export function ThemeControls({ className }: { className?: string }) {
	const { theme, setAccent, toggleAppearance } = useTheme();
	const isLight = theme.appearance === "light";
	const nextAccent = getNextAccent(theme.accent);

	return (
		<div className={cn("flex shrink-0 items-center gap-2", className)}>
			<button
				type="button"
				className="relative inline-flex h-10 w-11 items-center justify-center overflow-hidden rounded-full border border-(--color-border-strong) bg-(--surface-ghost-bg) shadow-[var(--shadow-panel-soft)] backdrop-blur-xl transition-[border-color,box-shadow,background-color,transform] duration-200 hover:border-(--color-border-accent) hover:-translate-y-0.5 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-(--color-accent)/35 sm:h-11 sm:w-12"
				aria-label={`Switch to ${isLight ? "dark" : "light"} mode`}
				title={isLight ? "Light mode" : "Dark mode"}
				onClick={toggleAppearance}
			>
				<span className="relative size-8 sm:size-9">
					<span
						aria-hidden="true"
						className={cn(
							"absolute top-1/2 left-1/2 flex -translate-x-1/2 -translate-y-1/2 items-center justify-center rounded-full border transition-all duration-300 ease-[var(--ease-out-expo)]",
							isLight
								? "z-10 size-5.5 translate-x-[0.52rem] translate-y-[0.14rem] border-(--color-border-default) bg-transparent text-(--color-text-secondary) opacity-70 sm:size-5"
								: "z-20 size-8 border-(--color-border-accent) bg-[linear-gradient(145deg,color-mix(in_srgb,var(--color-accent)_20%,var(--color-bg-panel)),color-mix(in_srgb,var(--color-bg-panel)_88%,white_12%))] text-(--color-text-primary) shadow-[0_10px_20px_color-mix(in_srgb,var(--color-accent)_20%,transparent)] sm:size-9",
						)}
					>
						<MoonStar
							className={cn(
								"transition-all duration-300",
								isLight
									? "size-2.5 opacity-72 sm:size-3"
									: "size-3.5 sm:size-4",
							)}
						/>
					</span>
					<span
						aria-hidden="true"
						className={cn(
							"absolute top-1/2 left-1/2 flex -translate-x-1/2 -translate-y-1/2 items-center justify-center rounded-full border transition-all duration-300 ease-[var(--ease-out-expo)]",
							isLight
								? "z-20 size-8 border-(--color-border-accent) bg-[linear-gradient(145deg,color-mix(in_srgb,var(--color-accent)_20%,var(--color-bg-panel)),color-mix(in_srgb,var(--color-bg-panel)_88%,white_12%))] text-(--color-text-primary) shadow-[0_10px_20px_color-mix(in_srgb,var(--color-accent)_20%,transparent)] sm:size-9"
								: "z-10 size-5.5 translate-x-[0.52rem] translate-y-[0.14rem] border-(--color-border-default) bg-transparent text-(--color-text-secondary) opacity-70 sm:size-5",
						)}
					>
						<SunMedium
							className={cn(
								"transition-all duration-300",
								isLight
									? "size-3.5 sm:size-4"
									: "size-2.5 opacity-72 sm:size-3",
							)}
						/>
					</span>
				</span>
			</button>

			<button
				type="button"
				className="relative inline-flex h-10 w-[3.2rem] items-center justify-center overflow-visible rounded-[1rem] border border-(--color-border-strong) bg-(--surface-ghost-bg) px-1 shadow-[var(--shadow-panel-soft)] backdrop-blur-xl transition-[border-color,box-shadow,background-color,transform] duration-200 hover:border-(--color-border-accent) hover:-translate-y-0.5 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-(--color-accent)/35 sm:h-11 sm:w-[3.5rem]"
				aria-label={`Switch accent theme to ${accentLabels[nextAccent]}`}
				title={`Accent theme: ${accentLabels[theme.accent]}`}
				onClick={() => setAccent(nextAccent)}
			>
				<span aria-hidden="true" className="relative h-6 w-7 sm:h-6.5 sm:w-8">
					{ACCENT_MODES.map((accent, index) => {
						const isSelected = theme.accent === accent;

						return (
							<span
								key={accent}
								className={cn(
									"absolute top-1/2 left-1/3 flex h-5 w-4 -translate-y-1/2 rounded-[0.4rem] border bg-linear-to-br shadow-[0_6px_14px_color-mix(in_srgb,var(--card-shadow)_20%,transparent)] transition-[transform,border-color,box-shadow,opacity] duration-300 ease-[var(--ease-out-expo)] sm:h-5.5 sm:w-5.5",
									accentCardPalette(accent),
									accentCardPosition(index),
									isSelected
										? "z-30 -translate-y-[0.9rem] border-(--color-text-primary) opacity-100 shadow-[0_0_0_2px_var(--color-bg-panel),0_10px_20px_color-mix(in_srgb,var(--card-shadow)_28%,transparent)]"
										: "z-10 border-(--color-border-strong) opacity-82",
								)}
							/>
						);
					})}
				</span>
			</button>
		</div>
	);
}

function accentCardPalette(accent: AccentMode) {
	return {
		green:
			"from-emerald-200 via-emerald-300 to-emerald-500 [--card-shadow:oklch(0.76_0.18_150)]",
		red: "from-rose-200 via-red-300 to-red-500 [--card-shadow:oklch(0.72_0.18_25)]",
		blue: "from-sky-200 via-blue-300 to-blue-500 [--card-shadow:oklch(0.7_0.16_258)]",
	}[accent];
}

function accentCardPosition(index: number) {
	return [
		"-translate-x-[1rem] rotate-[-18deg] sm:-translate-x-[1.1rem]",
		"-translate-x-[0.25rem] rotate-[-2deg]",
		"translate-x-[0.65rem] rotate-[16deg] sm:translate-x-[0.85rem]",
	][index];
}

function getNextAccent(accent: AccentMode): AccentMode {
	const currentIndex = ACCENT_MODES.indexOf(accent);
	return ACCENT_MODES[(currentIndex + 1) % ACCENT_MODES.length];
}
