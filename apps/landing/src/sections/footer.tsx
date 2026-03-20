import { Github } from "lucide-react";

export function Footer() {
	return (
		<footer
			className="border-t px-6 py-12"
			style={{ borderColor: "var(--color-border-subtle)" }}
		>
			<div className="mx-auto flex max-w-5xl flex-col items-center justify-between gap-6 sm:flex-row">
				<div className="flex items-center gap-2.5">
					<div
						className="flex h-7 w-7 items-center justify-center rounded-lg text-xs font-bold"
						style={{
							background: "var(--color-accent)",
							color: "oklch(0.15 0.013 152)",
						}}
					>
						R
					</div>
					<span
						className="text-sm font-semibold"
						style={{ color: "var(--color-text-secondary)" }}
					>
						Arkivist
					</span>
				</div>

				<div
					className="flex items-center gap-6 text-sm"
					style={{ color: "var(--color-text-quiet)" }}
				>
					<a
						href="https://github.com/your-org/arkivist"
						className="flex items-center gap-1.5 transition-colors"
						target="_blank"
						rel="noopener noreferrer"
						onMouseEnter={(e) => {
							e.currentTarget.style.color = "var(--color-text-secondary)";
						}}
						onMouseLeave={(e) => {
							e.currentTarget.style.color = "var(--color-text-quiet)";
						}}
					>
						<Github size={16} />
						GitHub
					</a>
					<span>AGPL-3.0</span>
				</div>
			</div>
		</footer>
	);
}
