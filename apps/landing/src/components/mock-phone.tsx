import type { ReactNode } from "react";

interface MockPhoneProps {
	children: ReactNode;
}

export function MockPhone({ children }: MockPhoneProps) {
	return (
		<div
			className="flex w-[220px] flex-col overflow-hidden rounded-[1.5rem] border-2"
			style={{
				borderColor: "var(--mock-phone-border)",
				background: "var(--color-bg-deep)",
				boxShadow:
					"var(--shadow-panel), 0 0 60px color-mix(in srgb, var(--color-accent) 10%, transparent)",
				height: "420px",
			}}
		>
			{/* Status bar */}
			<div
				className="flex items-center justify-between px-5 py-1.5"
				style={{ background: "var(--color-bg-deep)" }}
			>
				<span
					className="text-[8px] font-semibold"
					style={{ color: "var(--color-text-secondary)" }}
				>
					9:41
				</span>
				<div
					className="mx-auto h-5 w-20 rounded-full"
					style={{
						background: "color-mix(in srgb, white 8%, transparent)",
					}}
				/>
				<div className="flex items-center gap-1">
					<div
						className="flex items-end gap-[2px]"
						style={{ color: "var(--color-text-secondary)" }}
					>
						{[4, 6, 8, 10].map((h) => (
							<div
								key={h}
								className="rounded-sm"
								style={{
									width: 2,
									height: h,
									background: "currentColor",
								}}
							/>
						))}
					</div>
					<div
						className="ml-1 h-3 w-6 rounded-sm border"
						style={{
							borderColor: "var(--color-text-quiet)",
						}}
					>
						<div
							className="h-full rounded-sm"
							style={{
								width: "70%",
								background: "var(--color-accent)",
							}}
						/>
					</div>
				</div>
			</div>

			{/* Content */}
			<div className="flex flex-1 flex-col overflow-hidden">{children}</div>
		</div>
	);
}
