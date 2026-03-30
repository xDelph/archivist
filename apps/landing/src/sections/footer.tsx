import { Github } from "lucide-react";

const REPO_URL = "https://github.com/xDelph/archivist";

export function Footer() {
	return (
		<footer className="border-t border-(--color-border-subtle) px-6 py-8">
			<div className="mx-auto flex max-w-6xl flex-col gap-4 text-sm text-(--color-text-secondary) sm:flex-row sm:items-center sm:justify-between">
				<div className="flex items-center gap-3">
					<span className="flex h-8 w-8 items-center justify-center rounded-xl border border-(--color-border-strong) bg-(--color-bg-panel) text-xs font-semibold text-(--color-text-primary)">
						A
					</span>
					<div>
						<p className="font-semibold tracking-tight text-(--color-text-primary)">
							Arkivist
						</p>
						<p className="text-xs uppercase tracking-[0.14em] text-(--color-text-quiet)">
							Slack archive, catch-up, and offline reading
						</p>
					</div>
				</div>
				<div className="flex items-center gap-5">
					<span>AGPL-3.0</span>
					<a
						href={REPO_URL}
						target="_blank"
						rel="noreferrer"
						className="inline-flex items-center gap-2 transition-colors hover:text-(--color-text-primary)"
					>
						<Github className="size-4" />
						GitHub
					</a>
				</div>
			</div>
		</footer>
	);
}
