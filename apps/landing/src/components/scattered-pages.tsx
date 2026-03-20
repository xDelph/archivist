const FRAGMENTS = [
	{
		top: "10%",
		left: "5%",
		delay: "0s",
		duration: "5s",
		scatterX: "50px",
		scatterY: "-40px",
		scatterRot: "18deg",
		opacity: 0.2,
		size: "1.6rem",
	},
	{
		top: "25%",
		right: "8%",
		delay: "1.8s",
		duration: "6s",
		scatterX: "-40px",
		scatterY: "-50px",
		scatterRot: "-22deg",
		opacity: 0.15,
		size: "1.4rem",
	},
	{
		top: "60%",
		left: "8%",
		delay: "3.2s",
		duration: "5.5s",
		scatterX: "60px",
		scatterY: "20px",
		scatterRot: "30deg",
		opacity: 0.18,
		size: "1.2rem",
	},
	{
		top: "45%",
		right: "4%",
		delay: "0.8s",
		duration: "6.5s",
		scatterX: "-30px",
		scatterY: "-60px",
		scatterRot: "-15deg",
		opacity: 0.12,
		size: "1.8rem",
	},
	{
		top: "75%",
		right: "12%",
		delay: "2.5s",
		duration: "5s",
		scatterX: "20px",
		scatterY: "-35px",
		scatterRot: "25deg",
		opacity: 0.14,
		size: "1.3rem",
	},
];

export function ScatteredPages() {
	return (
		<div className="pointer-events-none absolute inset-0 overflow-hidden">
			{FRAGMENTS.map((f) => (
				<div
					key={`${f.top}-${f.delay}`}
					className="absolute"
					style={{
						top: f.top,
						left: f.left,
						right: f.right,
						width: f.size,
						height: `calc(${f.size} * 1.35)`,
						["--scatter-x" as string]: f.scatterX,
						["--scatter-y" as string]: f.scatterY,
						["--scatter-rot" as string]: f.scatterRot,
						["--scatter-opacity" as string]: f.opacity,
						animation: `page-scatter ${f.duration} var(--ease-out-quart) ${f.delay} infinite`,
					}}
				>
					<svg
						viewBox="0 0 40 54"
						fill="none"
						xmlns="http://www.w3.org/2000/svg"
						className="h-full w-full"
						aria-hidden="true"
					>
						{/* Torn/faded page */}
						<path
							d="M4 0h24l12 12v38a4 4 0 01-4 4H4a4 4 0 01-4-4V4a4 4 0 014-4z"
							fill="var(--color-text-quiet)"
							fillOpacity={f.opacity * 1.5}
						/>
						{/* Faded text lines */}
						<rect
							x="6"
							y="18"
							width="20"
							height="1.5"
							rx="0.75"
							fill="var(--color-text-quiet)"
							fillOpacity={f.opacity}
						/>
						<rect
							x="6"
							y="23"
							width="14"
							height="1.5"
							rx="0.75"
							fill="var(--color-text-quiet)"
							fillOpacity={f.opacity * 0.8}
						/>
						<rect
							x="6"
							y="28"
							width="18"
							height="1.5"
							rx="0.75"
							fill="var(--color-text-quiet)"
							fillOpacity={f.opacity * 0.6}
						/>
					</svg>
				</div>
			))}
		</div>
	);
}
