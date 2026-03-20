import { MoonStar, SunMedium } from "lucide-react";
import { ACCENT_MODES, type AccentMode, useTheme } from "../lib/theme";

export function ThemeControls() {
	const { theme, toggleAppearance, cycleAccent } = useTheme();
	const isLight = theme.appearance === "light";

	return (
		<div className="flex shrink-0 items-center gap-2">
			<button
				type="button"
				className="theme-control-btn relative inline-flex h-9 w-10 items-center justify-center overflow-hidden rounded-full sm:h-10 sm:w-11"
				aria-label={`Switch to ${isLight ? "dark" : "light"} mode`}
				title={isLight ? "Light mode" : "Dark mode"}
				onClick={toggleAppearance}
			>
				<span className="relative size-7 sm:size-8">
					<span
						aria-hidden="true"
						className="icon-orb absolute top-1/2 left-1/2 flex -translate-x-1/2 -translate-y-1/2 items-center justify-center rounded-full border"
						style={{
							...(isLight
								? {
										zIndex: 10,
										width: "1.25rem",
										height: "1.25rem",
										transform: "translate(0.52rem, 0.14rem)",
										borderColor: "var(--color-border-subtle)",
										opacity: 0.7,
									}
								: {
										zIndex: 20,
										width: "1.75rem",
										height: "1.75rem",
										borderColor:
											"color-mix(in srgb, var(--color-accent) 28%, transparent)",
										background:
											"linear-gradient(145deg, color-mix(in srgb, var(--color-accent) 20%, var(--color-bg-panel)), color-mix(in srgb, var(--color-bg-panel) 88%, white 12%))",
										boxShadow:
											"0 10px 20px color-mix(in srgb, var(--color-accent) 20%, transparent)",
									}),
						}}
					>
						<MoonStar size={isLight ? 10 : 14} />
					</span>
					<span
						aria-hidden="true"
						className="icon-orb absolute top-1/2 left-1/2 flex -translate-x-1/2 -translate-y-1/2 items-center justify-center rounded-full border"
						style={{
							...(isLight
								? {
										zIndex: 20,
										width: "1.75rem",
										height: "1.75rem",
										borderColor:
											"color-mix(in srgb, var(--color-accent) 28%, transparent)",
										background:
											"linear-gradient(145deg, color-mix(in srgb, var(--color-accent) 20%, var(--color-bg-panel)), color-mix(in srgb, var(--color-bg-panel) 88%, white 12%))",
										boxShadow:
											"0 10px 20px color-mix(in srgb, var(--color-accent) 20%, transparent)",
									}
								: {
										zIndex: 10,
										width: "1.25rem",
										height: "1.25rem",
										transform: "translate(0.52rem, 0.14rem)",
										borderColor: "var(--color-border-subtle)",
										opacity: 0.7,
									}),
						}}
					>
						<SunMedium size={isLight ? 14 : 10} />
					</span>
				</span>
			</button>

			<button
				type="button"
				className="theme-control-btn relative inline-flex h-9 w-12 items-center justify-center overflow-visible rounded-2xl px-1 sm:h-10 sm:w-13"
				aria-label="Cycle accent color"
				title={`Accent: ${theme.accent}`}
				onClick={cycleAccent}
			>
				<span aria-hidden="true" className="relative h-5 w-7 sm:h-6 sm:w-8">
					{ACCENT_MODES.map((accent, i) => (
						<span
							key={accent}
							className="accent-card absolute top-1/2 left-1/3 flex rounded-md border shadow-sm"
							style={{
								width: "1rem",
								height: "1.15rem",
								background: accentGradient(accent),
								...(theme.accent === accent
									? {
											zIndex: 30,
											transform: `translateY(-0.9rem) ${accentTransform(i)}`,
											borderColor: "var(--color-text-primary)",
											opacity: 1,
											boxShadow: `0 0 0 2px var(--color-bg-deep), 0 10px 20px ${accentShadow(accent)}`,
										}
									: {
											zIndex: 10,
											transform: `translateY(-50%) ${accentTransform(i)}`,
											borderColor: "var(--color-border-strong)",
											opacity: 0.82,
										}),
							}}
						/>
					))}
				</span>
			</button>
		</div>
	);
}

function accentGradient(accent: AccentMode) {
	return {
		red: "linear-gradient(to bottom right, #fecdd3, #fda4af, #f43f5e)",
		green: "linear-gradient(to bottom right, #a7f3d0, #6ee7b7, #10b981)",
		blue: "linear-gradient(to bottom right, #bae6fd, #93c5fd, #3b82f6)",
	}[accent];
}

function accentTransform(i: number) {
	return [
		"translateX(-1rem) rotate(-18deg)",
		"translateX(-0.25rem) rotate(-2deg)",
		"translateX(0.65rem) rotate(16deg)",
	][i];
}

function accentShadow(accent: AccentMode) {
	return {
		red: "rgba(244, 63, 94, 0.28)",
		green: "rgba(16, 185, 129, 0.28)",
		blue: "rgba(59, 130, 246, 0.28)",
	}[accent];
}
