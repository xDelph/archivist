import type { ReactNode } from "react";

interface MockBrowserProps {
	children: ReactNode;
	url?: string;
}

export function MockBrowser({
	children,
	url = "app.arkivist.dev",
}: MockBrowserProps) {
	return (
		<div
			className="overflow-hidden rounded-2xl border"
			style={{
				borderColor: "var(--color-border-subtle)",
				background: "var(--color-bg-deep)",
				boxShadow:
					"var(--shadow-panel), 0 0 120px color-mix(in srgb, var(--color-accent) 6%, transparent)",
			}}
		>
			<div
				className="flex items-center gap-2 border-b px-4 py-3"
				style={{
					borderColor: "var(--color-border-subtle)",
					background: "var(--mock-browser-chrome)",
				}}
			>
				<div className="flex gap-1.5">
					<div className="h-2.5 w-2.5 rounded-full bg-red-500/50" />
					<div className="h-2.5 w-2.5 rounded-full bg-yellow-500/50" />
					<div className="h-2.5 w-2.5 rounded-full bg-green-500/50" />
				</div>
				<div
					className="mx-auto rounded-md px-12 py-1 text-[11px] sm:px-20"
					style={{
						background: "color-mix(in srgb, white 4%, transparent)",
						color: "var(--color-text-quiet)",
					}}
				>
					{url}
				</div>
			</div>
			{children}
		</div>
	);
}
