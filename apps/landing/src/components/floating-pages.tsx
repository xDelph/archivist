const PAGES = [
	{
		top: "8%",
		left: "6%",
		delay: "0s",
		duration: "7s",
		rotStart: "-12deg",
		rotEnd: "6deg",
		riseEnd: "-140px",
		opacity: 0.14,
		size: "2.5rem",
	},
	{
		top: "18%",
		right: "8%",
		delay: "2.5s",
		duration: "8s",
		rotStart: "8deg",
		rotEnd: "-4deg",
		riseEnd: "-100px",
		opacity: 0.1,
		size: "2rem",
	},
	{
		top: "55%",
		left: "3%",
		delay: "4s",
		duration: "9s",
		rotStart: "-6deg",
		rotEnd: "10deg",
		riseEnd: "-160px",
		opacity: 0.12,
		size: "1.8rem",
	},
	{
		top: "40%",
		right: "5%",
		delay: "1.5s",
		duration: "7.5s",
		rotStart: "14deg",
		rotEnd: "-8deg",
		riseEnd: "-120px",
		opacity: 0.08,
		size: "3rem",
	},
	{
		top: "70%",
		left: "12%",
		delay: "3s",
		duration: "8.5s",
		rotStart: "-4deg",
		rotEnd: "12deg",
		riseEnd: "-130px",
		opacity: 0.1,
		size: "2.2rem",
	},
	{
		top: "30%",
		left: "18%",
		delay: "5.5s",
		duration: "7s",
		rotStart: "10deg",
		rotEnd: "-6deg",
		riseEnd: "-110px",
		opacity: 0.06,
		size: "2.8rem",
	},
];

export function FloatingPages() {
	return (
		<div className="pointer-events-none absolute inset-0 overflow-hidden">
			{PAGES.map((p) => (
				<div
					key={`${p.top}-${p.delay}`}
					className="absolute"
					style={{
						top: p.top,
						left: p.left,
						right: p.right,
						width: p.size,
						height: `calc(${p.size} * 1.35)`,
						["--page-rot-start" as string]: p.rotStart,
						["--page-rot-end" as string]: p.rotEnd,
						["--page-rise-end" as string]: p.riseEnd,
						["--page-opacity" as string]: p.opacity,
						animation: `page-rise ${p.duration} var(--ease-in-out-quad) ${p.delay} infinite`,
					}}
				>
					<svg
						viewBox="0 0 40 54"
						fill="none"
						xmlns="http://www.w3.org/2000/svg"
						className="h-full w-full"
						aria-hidden="true"
					>
						{/* Page shape with folded corner */}
						<path
							d="M4 0h24l12 12v38a4 4 0 01-4 4H4a4 4 0 01-4-4V4a4 4 0 014-4z"
							fill="var(--color-accent)"
							fillOpacity={p.opacity * 2.5}
						/>
						{/* Corner fold */}
						<path
							d="M28 0v8a4 4 0 004 4h8"
							stroke="var(--color-accent)"
							strokeOpacity={p.opacity * 3}
							strokeWidth="1.5"
							fill="none"
						/>
						{/* Text lines */}
						<rect
							x="6"
							y="18"
							width="20"
							height="1.5"
							rx="0.75"
							fill="var(--color-accent)"
							fillOpacity={p.opacity * 2}
						/>
						<rect
							x="6"
							y="23"
							width="16"
							height="1.5"
							rx="0.75"
							fill="var(--color-accent)"
							fillOpacity={p.opacity * 2}
						/>
						<rect
							x="6"
							y="28"
							width="22"
							height="1.5"
							rx="0.75"
							fill="var(--color-accent)"
							fillOpacity={p.opacity * 2}
						/>
						<rect
							x="6"
							y="33"
							width="12"
							height="1.5"
							rx="0.75"
							fill="var(--color-accent)"
							fillOpacity={p.opacity * 2}
						/>
					</svg>
				</div>
			))}
		</div>
	);
}
