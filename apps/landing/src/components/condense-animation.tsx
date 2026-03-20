import { Sparkles } from "lucide-react";

const SOURCE_LINES = [
	{ width: "90%", delay: "0s" },
	{ width: "75%", delay: "0.15s" },
	{ width: "85%", delay: "0.3s" },
	{ width: "60%", delay: "0.45s" },
	{ width: "95%", delay: "0.6s" },
	{ width: "70%", delay: "0.75s" },
	{ width: "80%", delay: "0.9s" },
	{ width: "55%", delay: "1.05s" },
];

export function CondenseAnimation({ visible }: { visible: boolean }) {
	if (!visible) return null;

	return (
		<div className="flex items-center gap-4 sm:gap-6">
			{/* Many lines (source) */}
			<div className="flex min-w-0 flex-1 flex-col gap-1.5">
				{SOURCE_LINES.map((line) => (
					<div
						key={line.delay}
						className="h-1.5 rounded-full sm:h-2"
						style={{
							width: line.width,
							background:
								"color-mix(in srgb, var(--color-text-quiet) 30%, transparent)",
							animation: `condense-line 4s var(--ease-out-quart) ${line.delay} infinite`,
						}}
					/>
				))}
			</div>

			{/* Arrow / sparkle */}
			<div
				className="flex shrink-0 flex-col items-center gap-1"
				style={{ color: "var(--color-accent-soft)" }}
			>
				<Sparkles size={16} />
				<div
					className="h-8 w-px"
					style={{
						background:
							"linear-gradient(to bottom, var(--color-accent-soft), transparent)",
					}}
				/>
			</div>

			{/* Few lines (summary) */}
			<div className="flex min-w-0 flex-1 flex-col gap-1.5">
				{[
					{ width: "85%", delay: "2s" },
					{ width: "70%", delay: "2.2s" },
					{ width: "45%", delay: "2.4s" },
				].map((line) => (
					<div
						key={line.delay}
						className="h-1.5 rounded-full sm:h-2"
						style={{
							width: line.width,
							background:
								"color-mix(in srgb, var(--color-accent) 50%, transparent)",
							animation: `condense-result 0.6s var(--ease-out-expo) ${line.delay} infinite both`,
						}}
					/>
				))}
			</div>
		</div>
	);
}
