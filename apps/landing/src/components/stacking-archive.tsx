const PAGES = [
	{ delay: "0.3s", offsetY: 0, rot: "0deg", width: "3rem" },
	{ delay: "0.7s", offsetY: -5, rot: "2deg", width: "2.8rem" },
	{ delay: "1.1s", offsetY: -10, rot: "-1.5deg", width: "2.6rem" },
	{ delay: "1.5s", offsetY: -15, rot: "1deg", width: "2.4rem" },
];

export function StackingArchive({ visible }: { visible: boolean }) {
	if (!visible) return null;

	return (
		<div className="flex flex-col items-center gap-2">
			<div className="relative h-20 w-16">
				{PAGES.map((page) => (
					<div
						key={page.delay}
						className="absolute bottom-0 left-1/2"
						style={{
							width: page.width,
							height: `calc(${page.width} * 1.3)`,
							marginLeft: `calc(-${page.width} / 2)`,
							["--stack-final-y" as string]: `${page.offsetY}px`,
							["--stack-final-rot" as string]: page.rot,
							animation: `page-stack 1.8s var(--ease-out-quart) ${page.delay} infinite`,
						}}
					>
						<svg
							viewBox="0 0 40 52"
							fill="none"
							xmlns="http://www.w3.org/2000/svg"
							className="h-full w-full"
							aria-hidden="true"
						>
							<path
								d="M4 0h24l12 12v36a4 4 0 01-4 4H4a4 4 0 01-4-4V4a4 4 0 014-4z"
								fill="var(--color-accent)"
								fillOpacity={0.18}
							/>
							<path
								d="M28 0v8a4 4 0 004 4h8"
								stroke="var(--color-accent)"
								strokeOpacity={0.25}
								strokeWidth="1.5"
								fill="none"
							/>
							<rect
								x="6"
								y="18"
								width="18"
								height="1.5"
								rx="0.75"
								fill="var(--color-accent)"
								fillOpacity={0.15}
							/>
							<rect
								x="6"
								y="23"
								width="12"
								height="1.5"
								rx="0.75"
								fill="var(--color-accent)"
								fillOpacity={0.12}
							/>
						</svg>
					</div>
				))}
			</div>
			<p
				className="text-[10px] font-medium tracking-wide"
				style={{ color: "var(--color-accent-muted)" }}
			>
				Saving to library...
			</p>
		</div>
	);
}
