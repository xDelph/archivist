import { Github, Menu, X } from "lucide-react";
import { type MouseEvent, useState } from "react";
import { ThemeControls } from "../components/theme-controls";

const NAV_LINKS = [
	{ href: "#why", label: "Why it matters" },
	{ href: "#features", label: "What it does" },
	{ href: "#workflow", label: "How it works" },
	{ href: "#open-source", label: "Open source" },
];

const REPO_URL = "https://github.com/xDelph/archivist";

function scrollToHash(event: MouseEvent<HTMLAnchorElement>) {
	const hash = event.currentTarget.getAttribute("href");
	if (!hash?.startsWith("#")) return;
	const target = document.getElementById(hash.slice(1));
	if (!target) return;
	event.preventDefault();
	target.scrollIntoView({ behavior: "smooth", block: "start" });
}

export function Header() {
	const [menuOpen, setMenuOpen] = useState(false);

	return (
		<header className="sticky top-0 z-50 border-b border-(--color-border-subtle) bg-(--color-bg-deep)/92 backdrop-blur-sm">
			<div className="mx-auto flex max-w-6xl items-center gap-4 px-6 py-4">
				<button
					type="button"
					className="flex items-center gap-3 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-(--color-accent)/35"
					onClick={() => {
						document
							.getElementById("top")
							?.scrollIntoView({ behavior: "smooth", block: "start" });
					}}
				>
					<span className="flex h-9 w-9 items-center justify-center rounded-xl border border-(--color-border-strong) bg-(--color-bg-panel) text-sm font-semibold text-(--color-text-primary)">
						A
					</span>
					<span className="text-base font-semibold tracking-tight text-(--color-text-primary)">
						Arkivist
					</span>
				</button>

				<nav className="ml-6 hidden items-center gap-6 lg:flex">
					{NAV_LINKS.map((link) => (
						<a
							key={link.href}
							href={link.href}
							onClick={scrollToHash}
							className="text-sm text-(--color-text-secondary) transition-colors hover:text-(--color-text-primary) focus-visible:outline-none focus-visible:text-(--color-text-primary)"
						>
							{link.label}
						</a>
					))}
				</nav>

				<div className="ml-auto flex items-center gap-3">
					<a
						href={REPO_URL}
						target="_blank"
						rel="noreferrer"
						className="hidden items-center gap-2 rounded-full border border-(--color-border-strong) px-4 py-2 text-sm font-medium text-(--color-text-primary) transition-colors hover:border-(--color-border-accent) hover:bg-(--color-bg-panel) sm:inline-flex"
					>
						<Github className="size-4" />
						GitHub
					</a>
					<ThemeControls />
					<button
						type="button"
						className="inline-flex h-10 w-10 items-center justify-center rounded-full border border-(--color-border-subtle) text-(--color-text-primary) lg:hidden"
						onClick={() => setMenuOpen((value) => !value)}
						aria-label={menuOpen ? "Close menu" : "Open menu"}
					>
						{menuOpen ? <X className="size-5" /> : <Menu className="size-5" />}
					</button>
				</div>
			</div>

			{menuOpen ? (
				<div className="border-t border-(--color-border-subtle) px-6 py-4 lg:hidden">
					<nav className="flex flex-col gap-3">
						{NAV_LINKS.map((link) => (
							<a
								key={link.href}
								href={link.href}
								onClick={(event) => {
									scrollToHash(event);
									setMenuOpen(false);
								}}
								className="text-sm text-(--color-text-secondary) transition-colors hover:text-(--color-text-primary)"
							>
								{link.label}
							</a>
						))}
						<a
							href={REPO_URL}
							target="_blank"
							rel="noreferrer"
							className="mt-2 inline-flex items-center gap-2 text-sm font-medium text-(--color-text-primary)"
						>
							<Github className="size-4" />
							Read the code
						</a>
					</nav>
				</div>
			) : null}
		</header>
	);
}
