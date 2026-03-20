import { Menu, X } from "lucide-react";
import { type MouseEvent, useState } from "react";
import { ThemeControls } from "../components/theme-controls";

const NAV_LINKS = [
	{ href: "#features", label: "Features" },
	{ href: "#how-it-works", label: "How it works" },
	{ href: "#open-source", label: "Open source" },
];

function scrollToHash(e: MouseEvent<HTMLAnchorElement>) {
	const hash = e.currentTarget.getAttribute("href");
	if (!hash?.startsWith("#")) return;
	const target = document.getElementById(hash.slice(1));
	if (!target) return;
	e.preventDefault();
	target.scrollIntoView({ behavior: "smooth", block: "start" });
}

export function Header() {
	const [menuOpen, setMenuOpen] = useState(false);

	return (
		<header
			className="relative z-50 shrink-0"
			style={{
				background: "color-mix(in srgb, var(--color-bg-deep) 85%, transparent)",
				backdropFilter: "blur(16px)",
			}}
		>
			<div className="mx-auto flex max-w-6xl items-center justify-between px-6 py-4">
				<button
					type="button"
					className="flex items-center gap-2.5"
					onClick={() => {
						document
							.querySelector(".snap-container")
							?.scrollTo({ top: 0, behavior: "smooth" });
					}}
				>
					<div
						className="flex h-8 w-8 items-center justify-center rounded-lg text-sm font-bold"
						style={{
							background: "var(--color-accent)",
							color: "var(--color-on-accent)",
							boxShadow: "var(--shadow-accent)",
						}}
					>
						R
					</div>
					<span
						className="text-lg font-semibold tracking-tight"
						style={{ color: "var(--color-text-bright)" }}
					>
						Arkivist
					</span>
				</button>

				<nav className="hidden items-center gap-8 md:flex">
					{NAV_LINKS.map((link) => (
						<a
							key={link.href}
							href={link.href}
							onClick={scrollToHash}
							className="text-sm font-medium transition-colors"
							style={{ color: "var(--color-text-secondary)" }}
							onMouseEnter={(e) => {
								e.currentTarget.style.color = "var(--color-text-primary)";
							}}
							onMouseLeave={(e) => {
								e.currentTarget.style.color = "var(--color-text-secondary)";
							}}
						>
							{link.label}
						</a>
					))}
				</nav>

				<div className="flex items-center gap-3">
					<ThemeControls />
					<button
						type="button"
						className="md:hidden"
						style={{ color: "var(--color-text-secondary)" }}
						onClick={() => setMenuOpen(!menuOpen)}
						aria-label={menuOpen ? "Close menu" : "Open menu"}
					>
						{menuOpen ? <X size={24} /> : <Menu size={24} />}
					</button>
				</div>
			</div>

			{menuOpen && (
				<div
					className="border-b px-6 pb-6 pt-2 md:hidden"
					style={{
						background:
							"color-mix(in srgb, var(--color-bg-deep) 95%, transparent)",
						backdropFilter: "blur(16px)",
						borderColor: "var(--color-border-subtle)",
					}}
				>
					{/* biome-ignore lint/a11y/useKeyWithClickEvents: nav menu close on any click */}
					<nav
						className="flex flex-col gap-4"
						onClick={() => setMenuOpen(false)}
					>
						{NAV_LINKS.map((link) => (
							<a
								key={link.href}
								href={link.href}
								onClick={scrollToHash}
								className="text-sm font-medium"
								style={{ color: "var(--color-text-secondary)" }}
							>
								{link.label}
							</a>
						))}
					</nav>
				</div>
			)}
		</header>
	);
}
